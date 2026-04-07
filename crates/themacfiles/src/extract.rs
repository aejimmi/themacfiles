//! Insight extraction from decoded telemetry records.

#[path = "extract_detail.rs"]
mod extract_detail;

use crate::schema::{
    AppInsight, BinaryFingerprint, BluetoothScanInsight, DecodedRecord, DeviceInsight, Insights,
    MlModelInsight, SecurityApiInsight,
};
use crate::{field_i64, field_str, field_u64};
use extract_detail::DetailState;
use std::collections::HashSet;

/// Tracking state accumulated during the extraction loop.
struct ExtractionState {
    seen_apps: HashSet<String>,
    seen_models: HashSet<String>,
    seen_views: HashSet<String>,
    seen_binaries: HashSet<String>,
    max_bt_devices: u64,
    detail: DetailState,
}

impl ExtractionState {
    fn new() -> Self {
        Self {
            seen_apps: HashSet::new(),
            seen_models: HashSet::new(),
            seen_views: HashSet::new(),
            seen_binaries: HashSet::new(),
            max_bt_devices: 0,
            detail: DetailState::new(),
        }
    }
}

/// Extract high-level insights from decoded records.
pub(crate) fn extract_insights(records: &[DecodedRecord]) -> Insights {
    let mut insights = Insights::default();
    let mut state = ExtractionState::new();

    for r in records {
        let ev = r.event_names.first().map_or("", String::as_str);
        let fields = &r.fields;

        extract_app_usage(ev, fields, &mut insights, &mut state);
        extract_ml_models(ev, fields, &mut insights, &mut state);
        extract_intelligence(ev, fields, &mut insights, &mut state);
        extract_bt_count(ev, fields, &mut state);
        extract_wifi_scan(ev, &mut insights);
        extract_executable(ev, fields, &mut insights, &mut state);
        extract_profiling(ev, fields, &mut insights);

        // Detail extractors (wifi, bt, safari, privacy, location, etc.)
        extract_detail::extract_wifi_session(r, fields, &mut insights, &mut state.detail);
        extract_detail::extract_bt_scan(ev, fields, &mut state.detail);
        extract_detail::extract_safari(r, ev, fields, &mut insights, &mut state.detail);
        extract_detail::extract_privacy(ev, fields, &mut insights);
        extract_detail::extract_location(ev, fields, &mut insights);
        extract_detail::extract_security_api(ev, fields, &mut state.detail);
        extract_detail::extract_photos(r, ev, fields, &mut insights);
        extract_detail::extract_behavioral_feedback(r, fields, &mut insights, &mut state.detail);
    }

    insights.bt_devices_found = state.max_bt_devices;
    finalize_insights(&mut insights, state);
    insights
}

/// Sort apps and finalize aggregated collections.
fn finalize_insights(insights: &mut Insights, state: ExtractionState) {
    // Sort: foreground first (by active time desc), then background (by uptime desc)
    insights.apps.sort_by(|a, b| {
        b.foreground.cmp(&a.foreground).then_with(|| {
            if a.foreground {
                b.active_seconds.cmp(&a.active_seconds)
            } else {
                b.uptime_seconds.cmp(&a.uptime_seconds)
            }
        })
    });

    // Finalize Bluetooth scans
    let mut bt_scans: Vec<BluetoothScanInsight> = state.detail.bt_scan_map.into_values().collect();
    bt_scans.sort_by(|a, b| b.max_devices_found.cmp(&a.max_devices_found));
    insights.bt_scans = bt_scans;

    // Finalize security APIs
    let mut sec_apis: Vec<SecurityApiInsight> = state
        .detail
        .seen_security_apps
        .into_iter()
        .map(|(app, apis)| SecurityApiInsight { app, apis })
        .collect();
    sec_apis.sort_by(|a, b| a.app.cmp(&b.app));
    insights.security_apis = sec_apis;

    // Sort behavioral domains
    insights.behavioral_domains.sort();
}

/// Extract app usage from `com.apple.osanalytics.appUsage` events.
fn extract_app_usage(
    ev: &str,
    fields: &[(String, serde_json::Value)],
    insights: &mut Insights,
    state: &mut ExtractionState,
) {
    if ev != "com.apple.osanalytics.appUsage" {
        return;
    }
    let Some(desc) = field_str(fields, "appDescription") else {
        return;
    };
    if state.seen_apps.contains(&desc) {
        return;
    }
    state.seen_apps.insert(desc.clone());
    let is_fg = field_str(fields, "foreground").as_deref() == Some("YES");
    let (name, version) = parse_app_description(&desc);
    insights.apps.push(AppInsight {
        name,
        version,
        active_seconds: field_i64(fields, "sum_of_activeTime"),
        uptime_seconds: field_i64(fields, "sum_of_uptime"),
        foreground: is_fg,
        activations: field_i64(fields, "sum_of_activations"),
        launches: field_i64(fields, "sum_of_activityPeriods"),
        caps: String::new(),
    });
}

/// Extract CoreML and Espresso ML model events.
fn extract_ml_models(
    ev: &str,
    fields: &[(String, serde_json::Value)],
    insights: &mut Insights,
    state: &mut ExtractionState,
) {
    if ev == "com.apple.CoreML.MLLoader" {
        extract_coreml_model(fields, insights, state);
    }
    if ev == "com.apple.Espresso.SegmentationAnalytics" {
        extract_espresso_model(fields, insights, state);
    }
}

/// Extract a CoreML model from an MLLoader event.
fn extract_coreml_model(
    fields: &[(String, serde_json::Value)],
    insights: &mut Insights,
    state: &mut ExtractionState,
) {
    if let Some(model_name) = field_str(fields, "modelName") {
        let bundle = field_str(fields, "bundleIdentifier").unwrap_or_default();
        let key = format!("{model_name}:{bundle}");
        if !state.seen_models.contains(&key) {
            state.seen_models.insert(key);
            insights.ml_models.push(MlModelInsight {
                name: model_name,
                bundle,
                compute_unit: String::new(),
            });
        }
    }
}

/// Extract an Espresso neural engine model.
fn extract_espresso_model(
    fields: &[(String, serde_json::Value)],
    insights: &mut Insights,
    state: &mut ExtractionState,
) {
    if let Some(cu) = field_str(fields, "computeUnit") {
        let bundle = field_str(fields, "bundleIdentifier").unwrap_or_default();
        let hash = field_str(fields, "modelHash").unwrap_or_default();
        let key = format!("{hash}:{cu}");
        if !state.seen_models.contains(&key) {
            state.seen_models.insert(key);
            insights.ml_models.push(MlModelInsight {
                name: format!("espresso:{}", &hash[..8.min(hash.len())]),
                bundle,
                compute_unit: cu,
            });
        }
    }
}

/// Extract intelligence view generation events.
fn extract_intelligence(
    ev: &str,
    fields: &[(String, serde_json::Value)],
    insights: &mut Insights,
    state: &mut ExtractionState,
) {
    if !ev.contains("intelligenceplatform.ViewGeneration") {
        return;
    }
    if let Some(view) = field_str(fields, "ViewName")
        && !state.seen_views.contains(&view)
    {
        state.seen_views.insert(view.clone());
        insights.intelligence_views.push(view);
    }
}

/// Track max Bluetooth devices found.
fn extract_bt_count(ev: &str, fields: &[(String, serde_json::Value)], state: &mut ExtractionState) {
    if ev == "com.apple.Bluetooth.LEScanSession" {
        let found = field_u64(fields, "NumberOfUniqueDevicesFound");
        if found > state.max_bt_devices {
            state.max_bt_devices = found;
        }
    }
}

/// Count WiFi scan results.
fn extract_wifi_scan(ev: &str, insights: &mut Insights) {
    if ev == "com.apple.wifi.scanResults" {
        insights.wifi_scans += 1;
    }
}

/// Extract executable measurement fingerprints.
fn extract_executable(
    ev: &str,
    fields: &[(String, serde_json::Value)],
    insights: &mut Insights,
    state: &mut ExtractionState,
) {
    if ev != "com.apple.syspolicy.ExecutableMeasurement" {
        return;
    }
    insights.executables_measured += 1;
    let cdhash = field_str(fields, "cdhash").unwrap_or_default();
    let signing_id = field_str(fields, "signingIdentifier").unwrap_or_default();
    if !cdhash.is_empty() && !state.seen_binaries.contains(&cdhash) {
        state.seen_binaries.insert(cdhash.clone());
        insights
            .fingerprinted_binaries
            .push(BinaryFingerprint { cdhash, signing_id });
    }
}

/// Extract personalization portrait profiling items.
fn extract_profiling(ev: &str, fields: &[(String, serde_json::Value)], insights: &mut Insights) {
    if ev.contains("PersonalizationPortrait.TopicStoreStats") {
        let items = field_u64(fields, "daily_maximum_uniqueItems");
        if items > insights.profiling_items {
            insights.profiling_items = items;
        }
    }
}

/// Extract device identity information from decoded records.
///
/// Scans for Safari version (reveals browser/OS), Apple Intelligence locale,
/// and Tips URLs (contain platform, OS version, and model hash).
pub(crate) fn extract_device_insights(records: &[DecodedRecord], device: &mut DeviceInsight) {
    for r in records {
        let ev = r.event_names.first().map_or("", String::as_str);
        let fields = &r.fields;

        extract_device_safari(ev, r, fields, device);
        extract_device_ai_locale(ev, fields, device);
        extract_device_tips_url(fields, device);
    }
}

/// Extract Safari client and version for device identity.
fn extract_device_safari(
    ev: &str,
    r: &DecodedRecord,
    fields: &[(String, serde_json::Value)],
    device: &mut DeviceInsight,
) {
    if !ev.starts_with("com.apple.Safari") && !r.transform_name.contains("Safari") {
        return;
    }
    if let Some(client) = field_str(fields, "safariClient")
        && device.platform.is_empty()
        && client.contains("Mac")
    {
        device.platform = "macOS".to_string();
    }
    if let Some(ver) = field_str(fields, "safariVersion")
        && device.safari_version.is_empty()
    {
        device.safari_version = ver;
    }
}

/// Extract Apple Intelligence locale for device identity.
fn extract_device_ai_locale(
    ev: &str,
    fields: &[(String, serde_json::Value)],
    device: &mut DeviceInsight,
) {
    if !ev.contains("AppleIntelligence") && !ev.contains("AppleIntelligenceReporting") {
        return;
    }
    if let Some(locale) = field_str(fields, "AppleIntelligenceLocale")
        && device.ai_locale.is_empty()
    {
        device.ai_locale = locale;
    }
}

/// Extract device identity from Tips feed URLs.
fn extract_device_tips_url(fields: &[(String, serde_json::Value)], device: &mut DeviceInsight) {
    for (_, val) in fields {
        if let Some(s) = val.as_str()
            && s.contains("ipcdn.apple.com")
            && s.contains("osVersion=")
        {
            parse_tips_url(s, device);
        }
    }
}

/// Parse device identity parameters from an Apple Tips URL.
pub(crate) fn parse_tips_url(url: &str, device: &mut DeviceInsight) {
    for param in url.split('&') {
        let param = param.split('?').next_back().unwrap_or(param);
        if let Some((key, val)) = param.split_once('=') {
            match key {
                "osVersion" if device.os_version.is_empty() => {
                    device.os_version = val.to_string();
                }
                "platform" if device.platform.is_empty() => {
                    device.platform = val.to_string();
                }
                "model" if device.model_hash.is_empty() => {
                    device.model_hash = val.to_string();
                }
                _ => {}
            }
        }
    }
}

/// Parse "com.example.app ||| 1.2.3 (build)" into (name, version).
pub(crate) fn parse_app_description(desc: &str) -> (String, String) {
    if let Some((name, version)) = desc.split_once(" ||| ") {
        (name.trim().to_string(), version.trim().to_string())
    } else {
        (desc.to_string(), String::new())
    }
}
