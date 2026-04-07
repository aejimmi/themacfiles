//! App profile extraction: groups decoded records by bundle ID and classifies capabilities.

use crate::extract::parse_app_description;
use crate::schema::{AppCapability, AppNetworkInfo, AppProfile, BinaryFingerprint, DecodedRecord};
use crate::{field_i64, field_str};
use std::collections::HashMap;

/// Known field names that contain a bundle identifier.
const BUNDLE_FIELDS: &[&str] = &[
    "appDescription",
    "appIdentifier",
    "bundleIdentifier",
    "app_bundleid",
    "BundleID",
    "bundle_id",
    "signingIdentifier",
];

/// Build per-app profiles from decoded records.
///
/// When `query` is provided, uses fuzzy matching: any record where the query
/// appears in a bundle ID field, a path, or ANY string field value is included.
/// Profiles with different keys that match the same query are merged.
pub fn build_app_profiles(records: &[DecodedRecord], query: Option<&str>) -> Vec<AppProfile> {
    let mut profiles: HashMap<String, AppProfile> = HashMap::new();

    for r in records {
        // When querying, match broadly: any field value containing the query
        if let Some(q) = query
            && !record_matches_query(r, q)
        {
            continue;
        }

        let bundle_id = extract_bundle_id(r)
            .or_else(|| {
                // Fallback for query mode: scan all string values for bundle-like IDs
                query.and_then(|q| extract_bundle_id_fuzzy(r, q))
            })
            .unwrap_or_else(|| "(unknown)".into());

        let profile = profiles
            .entry(bundle_id.clone())
            .or_insert_with(|| AppProfile {
                bundle_id: bundle_id.clone(),
                version: String::new(),
                active_seconds: 0,
                uptime_seconds: 0,
                foreground: false,
                activations: 0,
                launches: 0,
                capabilities: Vec::new(),
                binaries: Vec::new(),
                security_apis: Vec::new(),
                network: AppNetworkInfo::default(),
                hardware: Vec::new(),
                record_count: 0,
            });

        profile.record_count += 1;
        classify_record(r, profile);
    }

    // When querying, merge profiles that are clearly the same app
    // e.g. "Zed.app" and "dev.zed.Zed" should merge
    if let Some(q) = query {
        merge_related_profiles(&mut profiles, q);
    }

    let mut result: Vec<AppProfile> = profiles
        .into_values()
        .filter(|p| p.bundle_id != "(unknown)")
        .collect();

    result.sort_by(|a, b| {
        b.record_count
            .cmp(&a.record_count)
            .then_with(|| a.bundle_id.cmp(&b.bundle_id))
    });

    result
}

/// Check if any field value in a record contains the query (case-insensitive).
fn record_matches_query(record: &DecodedRecord, query: &str) -> bool {
    let q = query.to_ascii_lowercase();
    for (_, v) in &record.fields {
        let s = match v {
            serde_json::Value::String(s) => s.to_ascii_lowercase(),
            _ => v.to_string().to_ascii_lowercase(),
        };
        if s.contains(&q) {
            return true;
        }
    }
    false
}

/// Try to find a bundle-ID-like string that contains the query.
fn extract_bundle_id_fuzzy(record: &DecodedRecord, query: &str) -> Option<String> {
    let q = query.to_ascii_lowercase();
    for (_, v) in &record.fields {
        if let Some(s) = v.as_str() {
            let lower = s.to_ascii_lowercase();
            if lower.contains(&q) {
                // If it's a path, extract .app name
                if s.contains(".app")
                    && let Some(app) = extract_bundle_from_path(s)
                {
                    return Some(app);
                }
                // If it looks like a bundle ID
                if looks_like_bundle_id(s) {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

/// Merge profiles that refer to the same app (e.g. "Zed.app" + "dev.zed.Zed").
fn merge_related_profiles(profiles: &mut HashMap<String, AppProfile>, query: &str) {
    let q = query.to_ascii_lowercase();
    let keys: Vec<String> = profiles.keys().cloned().collect();

    // Find the "canonical" key — prefer the bundle ID format (has 2+ dots)
    let canonical = keys
        .iter()
        .filter(|k| k.to_ascii_lowercase().contains(&q))
        .max_by_key(|k| {
            let dots = k.chars().filter(|c| *c == '.').count();
            let is_bundle = if dots >= 2 { 100 } else { 0 };
            is_bundle + k.len()
        })
        .cloned();

    let Some(canonical) = canonical else { return };

    // Merge all other matching profiles into the canonical one
    let others: Vec<String> = keys
        .into_iter()
        .filter(|k| k != &canonical && k.to_ascii_lowercase().contains(&q))
        .collect();

    for other_key in others {
        if let Some(other) = profiles.remove(&other_key)
            && let Some(target) = profiles.get_mut(&canonical)
        {
            merge_into(target, other);
        }
    }
}

/// Merge one profile into another.
fn merge_into(target: &mut AppProfile, source: AppProfile) {
    target.record_count += source.record_count;
    if target.version.is_empty() && !source.version.is_empty() {
        target.version = source.version;
    }
    target.active_seconds += source.active_seconds;
    target.uptime_seconds += source.uptime_seconds;
    if source.foreground {
        target.foreground = true;
    }
    target.activations += source.activations;
    target.launches += source.launches;
    for cap in source.capabilities {
        if !target.capabilities.iter().any(|c| c.kind == cap.kind) {
            target.capabilities.push(cap);
        }
    }
    for bin in source.binaries {
        if !target.binaries.iter().any(|b| b.cdhash == bin.cdhash) {
            target.binaries.push(bin);
        }
    }
    for api in source.security_apis {
        if !target.security_apis.contains(&api) {
            target.security_apis.push(api);
        }
    }
    if target.network.interface.is_empty() && !source.network.interface.is_empty() {
        target.network = source.network;
    }
    for hw in source.hardware {
        if !target.hardware.contains(&hw) {
            target.hardware.push(hw);
        }
    }
}

/// Extract a bundle ID from a decoded record by checking known field names.
fn extract_bundle_id(record: &DecodedRecord) -> Option<String> {
    for field_name in BUNDLE_FIELDS {
        if let Some(val) = field_str(&record.fields, field_name) {
            let candidate = if *field_name == "appDescription" {
                let (name, _) = parse_app_description(&val);
                name
            } else {
                val
            };
            if looks_like_bundle_id(&candidate) {
                return Some(candidate);
            }
        }
    }

    // Fallback: check field values for bundle IDs or .app paths
    let mut path_bundle: Option<String> = None;
    for (_, v) in &record.fields {
        if let Some(s) = v.as_str() {
            // Direct bundle ID as value (e.g. "dev.zed.Zed" appearing as a value)
            if looks_like_bundle_id(s) && s.contains("com.")
                || s.contains("dev.")
                || s.contains("ch.")
                || s.contains("us.")
                || s.contains("org.")
                || s.contains("io.")
            {
                return Some(s.to_string());
            }
            // Path-based extraction as last resort
            if path_bundle.is_none()
                && let Some(bid) = extract_bundle_from_path(s)
            {
                path_bundle = Some(bid);
            }
        }
    }

    path_bundle
}

/// Check whether a string looks like a bundle identifier.
///
/// Bundle IDs contain dots and no spaces (e.g. "com.apple.Safari").
fn looks_like_bundle_id(s: &str) -> bool {
    !s.is_empty() && s.contains('.') && !s.contains(' ')
}

/// Try to extract a bundle-like name from an application path.
///
/// Paths like `/Applications/Zed.app/Contents/MacOS/zed` yield `Zed.app`.
fn extract_bundle_from_path(s: &str) -> Option<String> {
    if !s.contains("/Applications/") && !s.contains(".app/") {
        return None;
    }
    // Find the .app component
    for segment in s.split('/') {
        if std::path::Path::new(segment)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
        {
            return Some(segment.to_string());
        }
    }
    None
}

/// Classify a decoded record and enrich the app profile.
fn classify_record(record: &DecodedRecord, profile: &mut AppProfile) {
    let ev = record.event_names.first().map_or("", String::as_str);

    classify_app_usage(ev, record, profile);
    classify_dataexfil(ev, record, profile);
    classify_security_apis(ev, record, profile);
    classify_binaries(ev, record, profile);
    classify_network(ev, record, profile);
    classify_hardware(ev, record, profile);
}

/// Extract app usage stats from osanalytics.appUsage.
fn classify_app_usage(ev: &str, record: &DecodedRecord, profile: &mut AppProfile) {
    if ev != "com.apple.osanalytics.appUsage" {
        return;
    }
    let is_fg = field_str(&record.fields, "foreground").as_deref() == Some("YES");
    if is_fg {
        profile.foreground = true;
    }
    if let Some(desc) = field_str(&record.fields, "appDescription") {
        let (_, version) = parse_app_description(&desc);
        if !version.is_empty() && profile.version.is_empty() {
            profile.version = version;
        }
    }
    let active = field_i64(&record.fields, "sum_of_activeTime");
    let uptime = field_i64(&record.fields, "sum_of_uptime");
    let activations = field_i64(&record.fields, "sum_of_activations");
    let launches = field_i64(&record.fields, "sum_of_activityPeriods");
    profile.active_seconds += active;
    profile.uptime_seconds += uptime;
    profile.activations += activations;
    profile.launches += launches;
}

/// Detect DataExfil capabilities by scanning ALL field values.
fn classify_dataexfil(_ev: &str, record: &DecodedRecord, profile: &mut AppProfile) {
    // DataExfil patterns appear as field VALUES, not in event names
    let all_text: String = record
        .fields
        .iter()
        .filter_map(|(_, v)| v.as_str().map(String::from))
        .collect::<Vec<_>>()
        .join(" ");

    let ev_name = record.event_names.first().map_or("", String::as_str);

    if all_text.contains("DataExfil.Clipboard")
        && !profile.capabilities.iter().any(|c| c.kind == "Clipboard")
    {
        profile.capabilities.push(AppCapability {
            kind: "Clipboard".into(),
            source_event: ev_name.to_string(),
        });
    }
    if all_text.contains("DataExfil.Keychain")
        && !profile.capabilities.iter().any(|c| c.kind == "Keychain")
    {
        profile.capabilities.push(AppCapability {
            kind: "Keychain".into(),
            source_event: ev_name.to_string(),
        });
    }
    if all_text.contains("Network.Outgoing")
        && !profile
            .capabilities
            .iter()
            .any(|c| c.kind == "NetworkOutgoing")
    {
        profile.capabilities.push(AppCapability {
            kind: "NetworkOutgoing".into(),
            source_event: ev_name.to_string(),
        });
    }
}

/// Detect security API usage — look for known API names in field values.
fn classify_security_apis(_ev: &str, record: &DecodedRecord, profile: &mut AppProfile) {
    const SECURITY_APIS: &[&str] = &[
        "SecItemCopyMatching",
        "SecItemAdd",
        "SecItemUpdate",
        "SecItemDelete",
        "CSSM_ModuleDetach",
        "CSSM_ModuleUnload",
        "CSSM_ModuleLoad",
        "SessionGetInfo",
        "SecKeychainItemCopyContent",
    ];

    for (_, v) in &record.fields {
        if let Some(s) = v.as_str() {
            for api in SECURITY_APIS {
                if s.contains(api) && !profile.security_apis.contains(&(*api).to_string()) {
                    profile.security_apis.push((*api).to_string());
                }
            }
        }
    }
}

/// Detect binary fingerprints from syspolicy.ExecutableMeasurement.
fn classify_binaries(ev: &str, record: &DecodedRecord, profile: &mut AppProfile) {
    if ev != "com.apple.syspolicy.ExecutableMeasurement" {
        return;
    }
    let cdhash = field_str(&record.fields, "cdhash").unwrap_or_default();
    let signing_id = field_str(&record.fields, "signingIdentifier").unwrap_or_default();
    if !cdhash.is_empty() && !profile.binaries.iter().any(|b| b.cdhash == cdhash) {
        profile
            .binaries
            .push(BinaryFingerprint { cdhash, signing_id });
    }
}

/// Detect network info — look for WiFi/Cellular in field values with byte counts.
fn classify_network(_ev: &str, record: &DecodedRecord, profile: &mut AppProfile) {
    // Check if any field value mentions WiFi or Cellular
    let has_network_ref = record.fields.iter().any(|(_, v)| {
        v.as_str()
            .is_some_and(|s| s == "WiFi" || s == "Cellular" || s == "PersonalHotspot")
    });

    if !has_network_ref {
        return;
    }

    if profile.network.interface.is_empty() {
        for (_, v) in &record.fields {
            if let Some(s) = v.as_str()
                && (s == "WiFi" || s == "Cellular" || s == "PersonalHotspot")
            {
                profile.network.interface = s.to_string();
                break;
            }
        }
    }
    // Collect large numeric values as potential byte counts
    for (_, v) in &record.fields {
        if let Some(n) = v.as_i64()
            && n > 1000
            && !profile.network.bytes_values.contains(&n)
        {
            profile.network.bytes_values.push(n);
        }
    }
}

/// Detect hardware info — extract memory, GPU, CPU, thermal data with labels.
fn classify_hardware(ev: &str, record: &DecodedRecord, profile: &mut AppProfile) {
    // Memory footprint
    if ev.contains("memorytools.stats.footprint") {
        if let Some(kb) = field_str(&record.fields, "bucketed_app_footprint_kb")
            && !profile.hardware.iter().any(|(k, _)| k == "Memory")
        {
            profile.hardware.push(("Memory".into(), format!("{kb} KB")));
        }
        if let Some(kb) = field_str(&record.fields, "bucketed_app_neural_footprint_kb")
            && !profile
                .hardware
                .iter()
                .any(|(k, _)| k == "Neural Engine Memory")
        {
            profile
                .hardware
                .push(("Neural Engine Memory".into(), format!("{kb} KB")));
        }
        return;
    }

    // GPU/Metal — scan field values
    let has_metal = record
        .fields
        .iter()
        .any(|(_, v)| v.as_str().is_some_and(|s| s == "Metal"));
    if has_metal {
        if !profile.hardware.iter().any(|(k, _)| k == "GPU") {
            profile.hardware.push(("GPU".into(), "Metal".into()));
        }
        return;
    }

    // CPU model — look for chip names in field values
    for (_, v) in &record.fields {
        if let Some(s) = v.as_str()
            && (s.starts_with("M1")
                || s.starts_with("M2")
                || s.starts_with("M3")
                || s.starts_with("M4"))
            && !profile.hardware.iter().any(|(k, _)| k == "CPU")
        {
            profile.hardware.push(("CPU".into(), s.to_string()));
        }
    }

    // Thermal state
    for (_, v) in &record.fields {
        if let Some(s) = v.as_str()
            && (s == "Nominal" || s == "Fair" || s == "Serious" || s == "Critical")
            && !profile.hardware.iter().any(|(k, _)| k == "Thermal")
        {
            profile.hardware.push(("Thermal".into(), s.to_string()));
        }
    }
}

#[cfg(test)]
#[path = "app_profile_test.rs"]
mod app_profile_test;
