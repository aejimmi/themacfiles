//! Detail-level extractors for WiFi, Bluetooth, Safari, Privacy, Location,
//! Security APIs, Photos, and behavioral feedback domains.

use crate::schema::{BluetoothScanInsight, DecodedRecord, Insights, WifiSessionInsight};
use crate::{field_i64, field_str, field_u64};
use std::collections::{HashMap, HashSet};

/// Mutable state for detail extraction, accumulated across the record loop.
pub(super) struct DetailState {
    pub seen_wifi_ouis: HashSet<String>,
    pub seen_locales: HashSet<String>,
    pub seen_feedback_domains: HashSet<String>,
    pub seen_security_apps: HashMap<String, Vec<String>>,
    pub bt_scan_map: HashMap<String, BluetoothScanInsight>,
}

impl DetailState {
    pub fn new() -> Self {
        Self {
            seen_wifi_ouis: HashSet::new(),
            seen_locales: HashSet::new(),
            seen_feedback_domains: HashSet::new(),
            seen_security_apps: HashMap::new(),
            bt_scan_map: HashMap::new(),
        }
    }
}

/// Extract WiFi link session details (network and location revealing).
pub(super) fn extract_wifi_session(
    r: &DecodedRecord,
    fields: &[(String, serde_json::Value)],
    insights: &mut Insights,
    state: &mut DetailState,
) {
    let ev = r.event_names.first().map_or("", String::as_str);
    if ev != "com.apple.wifi.linksession" {
        return;
    }
    if !r.transform_name.contains("WiFiLinkSessionStats") {
        return;
    }
    let oui = field_str(fields, "NetworkBssOui").unwrap_or_default();
    if oui.is_empty() || state.seen_wifi_ouis.contains(&oui) {
        return;
    }
    state.seen_wifi_ouis.insert(oui.clone());
    insights.wifi_sessions.push(WifiSessionInsight {
        oui,
        country: field_str(fields, "NetworkCountryCodeAdvertised").unwrap_or_default(),
        band: field_str(fields, "NetworkBssBand").unwrap_or_default(),
        is_personal_hotspot: field_str(fields, "NetworkIsPersonalHotspot").as_deref()
            == Some("True"),
        has_wpa3: field_str(fields, "NetworkHasWpa3").as_deref() == Some("True"),
        join_reason: field_str(fields, "WiFiNetworkJoinReason").unwrap_or_default(),
        disconnect_reason: field_str(fields, "WiFiNetworkDisconnectReason").unwrap_or_default(),
        private_mac_type: field_str(fields, "NetworkPrivateMacType").unwrap_or_default(),
        session_duration_secs: field_i64(fields, "sum_of_SessionDuration"),
        rx_bytes: field_i64(fields, "sum_of_NetIfWiFiRxBytes"),
        tx_bytes: field_i64(fields, "sum_of_NetIfWiFiTxBytes"),
    });
}

/// Extract Bluetooth scan detail per app.
pub(super) fn extract_bt_scan(
    ev: &str,
    fields: &[(String, serde_json::Value)],
    state: &mut DetailState,
) {
    if ev != "com.apple.Bluetooth.LEScanSession" {
        return;
    }
    let bundle = field_str(fields, "BundleID").unwrap_or_default();
    if bundle.is_empty() {
        return;
    }
    let count = field_u64(fields, "Count").max(1);
    let devices = field_u64(fields, "NumberOfUniqueDevicesFound");
    let paired = field_u64(fields, "NumberOfUniquePairedDevicesFound");
    let use_case = field_str(fields, "CBUseCase").unwrap_or_default();
    let entry = state
        .bt_scan_map
        .entry(bundle.clone())
        .or_insert_with(|| BluetoothScanInsight {
            bundle_id: bundle,
            use_case: use_case.clone(),
            max_devices_found: 0,
            paired_found: 0,
            scan_count: 0,
        });
    entry.scan_count += count;
    if devices > entry.max_devices_found {
        entry.max_devices_found = devices;
    }
    if paired > entry.paired_found {
        entry.paired_found = paired;
    }
    if !use_case.is_empty() && use_case != "Unspecified" && entry.use_case == "Unspecified" {
        entry.use_case = use_case;
    }
}

/// Extract Safari browsing profile data.
pub(super) fn extract_safari(
    r: &DecodedRecord,
    ev: &str,
    fields: &[(String, serde_json::Value)],
    insights: &mut Insights,
    state: &mut DetailState,
) {
    if !ev.starts_with("com.apple.Safari") && !r.transform_name.contains("Safari") {
        return;
    }
    if let Some(engine) = field_str(fields, "defaultSearchProviderIdentifier")
        && insights.safari.search_engine.is_empty()
    {
        insights.safari.search_engine = engine;
    }
    if let Some(region) = field_str(fields, "userRegion")
        && insights.safari.user_region.is_empty()
    {
        insights.safari.user_region = region;
    }
    let tabs = field_u64(fields, "bucketed_tabCount");
    if tabs > insights.safari.tab_count {
        insights.safari.tab_count = tabs;
    }
    if field_str(fields, "isSearch").as_deref() == Some("True") {
        insights.safari.search_count += 1;
    }
    if r.transform_name.contains("DidSubmitForm") {
        insights.safari.form_submissions += 1;
    }
    extract_safari_locale(fields, insights, state);
}

/// Extract webpage locale from Safari events.
fn extract_safari_locale(
    fields: &[(String, serde_json::Value)],
    insights: &mut Insights,
    state: &mut DetailState,
) {
    if let Some(locale) = field_str(fields, "webpageLocale")
        && !locale.is_empty()
        && !state.seen_locales.contains(&locale)
    {
        state.seen_locales.insert(locale.clone());
        insights.safari.webpage_locales.push(locale);
    }
}

/// Extract privacy tool awareness (VPN, content filter, DNS proxy).
pub(super) fn extract_privacy(
    ev: &str,
    fields: &[(String, serde_json::Value)],
    insights: &mut Insights,
) {
    if !ev.contains("privacyProxyStalls") {
        return;
    }
    if field_str(fields, "vpnConnected").as_deref() == Some("1") {
        insights.privacy.vpn_detected = true;
    }
    if field_str(fields, "contentFilterConnected").as_deref() == Some("1") {
        insights.privacy.content_filter_detected = true;
    }
    if field_str(fields, "dnsProxyConnected").as_deref() == Some("1") {
        insights.privacy.dns_proxy_detected = true;
    }
    if let Some(status) = field_str(fields, "privacyProxyServiceStatus")
        && insights.privacy.private_relay_status.is_empty()
    {
        insights.privacy.private_relay_status = status;
    }
    let stalls = field_i64(fields, "sum_of_dnsStall");
    if stalls > insights.privacy.dns_stalls {
        insights.privacy.dns_stalls = stalls;
    }
    let fails = field_i64(fields, "sum_of_connectionFailed");
    if fails > insights.privacy.connection_failures {
        insights.privacy.connection_failures = fails;
    }
}

/// Extract location tracking profile.
pub(super) fn extract_location(
    ev: &str,
    fields: &[(String, serde_json::Value)],
    insights: &mut Insights,
) {
    if ev.contains("MicroLocation.Visit") {
        let home = field_u64(fields, "bucketed_loiHomeCount");
        if home > 0 {
            insights.location.home_detected = true;
        }
    }
    if ev.contains("locationd.AlsRequest") {
        let total = field_u64(fields, "sum_of_totalRequestCount");
        if total > insights.location.location_queries {
            insights.location.location_queries = total;
        }
    }
    if ev.contains("CoreRoutine.XPCActivitySuccessRate") {
        extract_location_routine(fields, insights);
    }
}

/// Extract CoreRoutine location awareness heartbeats and POI downloads.
fn extract_location_routine(fields: &[(String, serde_json::Value)], insights: &mut Insights) {
    let Some(id) = field_str(fields, "identifier") else {
        return;
    };
    if id.contains("locationAwareness.heartbeat") {
        let count = field_u64(fields, "Count");
        insights.location.heartbeat_count += count;
    }
    if id.contains("bluePOITileManager") {
        insights.location.poi_tile_downloads = true;
    }
}

/// Extract per-app security API usage.
pub(super) fn extract_security_api(
    ev: &str,
    fields: &[(String, serde_json::Value)],
    state: &mut DetailState,
) {
    if ev != "com.apple.security.LegacyAPICounts" {
        return;
    }
    if let Some(app) = field_str(fields, "app")
        && let Some(api) = field_str(fields, "api")
    {
        let apis = state.seen_security_apps.entry(app).or_default();
        if !apis.contains(&api) {
            apis.push(api);
        }
    }
}

/// Extract photos library analysis profile.
pub(super) fn extract_photos(
    r: &DecodedRecord,
    ev: &str,
    fields: &[(String, serde_json::Value)],
    insights: &mut Insights,
) {
    if !ev.starts_with("com.apple.photos")
        && !ev.contains("photoanalysis")
        && !r.transform_name.contains("Photo")
    {
        return;
    }
    let assets = field_u64(fields, "sum_of_totalAssetCount");
    if assets > insights.photos.total_assets {
        insights.photos.total_assets = assets;
    }
    let moments = field_u64(fields, "sum_of_numOfMoments");
    if moments > insights.photos.moments {
        insights.photos.moments = moments;
    }
    if let Some(size) = field_str(fields, "cpa_common_librarySizeRange")
        && insights.photos.library_size.is_empty()
    {
        insights.photos.library_size = size;
    }
    extract_photos_flags(fields, insights);
    extract_photos_analysis(fields, insights);
}

/// Extract Apple Music and iCloud Photos flags from photo events.
fn extract_photos_flags(fields: &[(String, serde_json::Value)], insights: &mut Insights) {
    if field_str(fields, "cpa_music_hasAppleMusicSubscription").as_deref() == Some("True") {
        insights.photos.has_apple_music = true;
    }
    if field_str(fields, "cpa_common_icpl_enabled").as_deref() == Some("True") {
        insights.photos.icloud_photos_enabled = true;
    }
}

/// Extract face and scene analysis progress from photos events.
fn extract_photos_analysis(fields: &[(String, serde_json::Value)], insights: &mut Insights) {
    let face = fields
        .iter()
        .find_map(|(k, v)| {
            if k == "daily_maximum_cpa_common_faceAnalysisProgress" {
                v.as_f64()
            } else {
                None
            }
        })
        .unwrap_or(0.0);
    if face > insights.photos.face_analysis_progress {
        insights.photos.face_analysis_progress = face;
    }
    let scene = fields
        .iter()
        .find_map(|(k, v)| {
            if k == "daily_maximum_cpa_common_sceneAnalysisProgress" {
                v.as_f64()
            } else {
                None
            }
        })
        .unwrap_or(0.0);
    if scene > insights.photos.scene_analysis_progress {
        insights.photos.scene_analysis_progress = scene;
    }
}

/// Extract behavioral feedback domain tracking.
pub(super) fn extract_behavioral_feedback(
    r: &DecodedRecord,
    fields: &[(String, serde_json::Value)],
    insights: &mut Insights,
    state: &mut DetailState,
) {
    if !r.transform_name.contains("FeedbackFiles") {
        return;
    }
    if let Some(client) = field_str(fields, "client")
        && !state.seen_feedback_domains.contains(&client)
    {
        state.seen_feedback_domains.insert(client.clone());
        insights.behavioral_domains.push(client);
    }
}
