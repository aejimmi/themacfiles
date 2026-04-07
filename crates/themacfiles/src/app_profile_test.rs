#![allow(clippy::indexing_slicing)] // Panicking on bad index is fine in tests

use super::*;
use crate::category::Category;
use crate::schema::DecodedRecord;

fn make_record(event: &str, fields: Vec<(&str, serde_json::Value)>) -> DecodedRecord {
    DecodedRecord {
        event_names: vec![event.into()],
        transform_name: "test".into(),
        category: Category::Other,
        config_type: "Main".into(),
        config_enabled: true,
        fields: fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
        event_count: 1,
    }
}

#[test]
fn test_extract_bundle_id_from_app_description() {
    let r = make_record(
        "com.apple.osanalytics.appUsage",
        vec![
            (
                "appDescription",
                serde_json::json!("com.example.MyApp ||| 2.1.0"),
            ),
            ("foreground", serde_json::json!("YES")),
            ("sum_of_activeTime", serde_json::json!(120)),
            ("sum_of_uptime", serde_json::json!(300)),
        ],
    );
    let bid = extract_bundle_id(&r);
    assert_eq!(bid.as_deref(), Some("com.example.MyApp"));
}

#[test]
fn test_extract_bundle_id_from_app_identifier() {
    let r = make_record(
        "com.apple.appkit.app_config",
        vec![("appIdentifier", serde_json::json!("com.apple.Safari"))],
    );
    let bid = extract_bundle_id(&r);
    assert_eq!(bid.as_deref(), Some("com.apple.Safari"));
}

#[test]
fn test_extract_bundle_id_from_bundle_identifier() {
    let r = make_record(
        "com.apple.CoreML.MLLoader",
        vec![
            ("bundleIdentifier", serde_json::json!("com.apple.Photos")),
            ("modelName", serde_json::json!("faces")),
        ],
    );
    let bid = extract_bundle_id(&r);
    assert_eq!(bid.as_deref(), Some("com.apple.Photos"));
}

#[test]
fn test_extract_bundle_id_from_signing_identifier() {
    let r = make_record(
        "com.apple.syspolicy.ExecutableMeasurement",
        vec![
            ("signingIdentifier", serde_json::json!("dev.zed.Zed")),
            ("cdhash", serde_json::json!("abc123")),
        ],
    );
    let bid = extract_bundle_id(&r);
    assert_eq!(bid.as_deref(), Some("dev.zed.Zed"));
}

#[test]
fn test_extract_bundle_id_from_path_fallback() {
    let r = make_record(
        "com.apple.tle.constraints.usage",
        vec![(
            "constraint",
            serde_json::json!("/Applications/Zed.app/Contents/MacOS/zed"),
        )],
    );
    let bid = extract_bundle_id(&r);
    assert_eq!(bid.as_deref(), Some("Zed.app"));
}

#[test]
fn test_extract_bundle_id_none_for_garbage() {
    let r = make_record(
        "com.apple.something",
        vec![("random_field", serde_json::json!(42))],
    );
    assert!(extract_bundle_id(&r).is_none());
}

#[test]
fn test_looks_like_bundle_id() {
    assert!(looks_like_bundle_id("com.apple.Safari"));
    assert!(looks_like_bundle_id("dev.zed.Zed"));
    assert!(!looks_like_bundle_id("Safari"));
    assert!(!looks_like_bundle_id(""));
    assert!(!looks_like_bundle_id("some thing with spaces.app"));
}

#[test]
fn test_build_app_profiles_groups_by_bundle() {
    let records = vec![
        make_record(
            "com.apple.osanalytics.appUsage",
            vec![
                (
                    "appDescription",
                    serde_json::json!("com.apple.Safari ||| 18.0"),
                ),
                ("foreground", serde_json::json!("YES")),
                ("sum_of_activeTime", serde_json::json!(100)),
                ("sum_of_uptime", serde_json::json!(200)),
                ("sum_of_activations", serde_json::json!(5)),
                ("sum_of_activityPeriods", serde_json::json!(2)),
            ],
        ),
        make_record(
            "com.apple.osanalytics.appUsage",
            vec![
                (
                    "appDescription",
                    serde_json::json!("com.apple.Safari ||| 18.0"),
                ),
                ("foreground", serde_json::json!("NO")),
                ("sum_of_activeTime", serde_json::json!(50)),
                ("sum_of_uptime", serde_json::json!(500)),
                ("sum_of_activations", serde_json::json!(0)),
                ("sum_of_activityPeriods", serde_json::json!(0)),
            ],
        ),
        make_record(
            "com.apple.appkit.app_config",
            vec![("appIdentifier", serde_json::json!("dev.zed.Zed"))],
        ),
    ];

    let profiles = build_app_profiles(&records, None);
    assert_eq!(profiles.len(), 2);

    let safari = profiles.iter().find(|p| p.bundle_id == "com.apple.Safari");
    assert!(safari.is_some());
    let safari = safari.expect("safari profile missing");
    assert_eq!(safari.record_count, 2);
    assert_eq!(safari.active_seconds, 150);
    assert_eq!(safari.uptime_seconds, 700);
    assert!(safari.foreground);
    assert_eq!(safari.version, "18.0");
}

#[test]
fn test_build_app_profiles_query_filter() {
    let records = vec![
        make_record(
            "com.apple.appkit.app_config",
            vec![("appIdentifier", serde_json::json!("com.apple.Safari"))],
        ),
        make_record(
            "com.apple.appkit.app_config",
            vec![("appIdentifier", serde_json::json!("dev.zed.Zed"))],
        ),
    ];

    let profiles = build_app_profiles(&records, Some("zed"));
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].bundle_id, "dev.zed.Zed");
}

#[test]
fn test_build_app_profiles_query_case_insensitive() {
    let records = vec![make_record(
        "com.apple.appkit.app_config",
        vec![("appIdentifier", serde_json::json!("dev.zed.Zed"))],
    )];

    let profiles = build_app_profiles(&records, Some("ZED"));
    assert_eq!(profiles.len(), 1);
}

#[test]
fn test_capability_detection_clipboard() {
    let records = vec![make_record(
        "com.apple.tle.constraints.usage",
        vec![
            (
                "constraint",
                serde_json::json!("/Applications/Zed.app/Contents/MacOS/zed"),
            ),
            ("category", serde_json::json!("DataExfil.Clipboard.Read")),
        ],
    )];

    let profiles = build_app_profiles(&records, None);
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].capabilities.len(), 1);
    assert_eq!(profiles[0].capabilities[0].kind, "Clipboard");
}

#[test]
fn test_capability_detection_keychain() {
    let records = vec![make_record(
        "com.apple.tle.constraints.usage",
        vec![
            ("appIdentifier", serde_json::json!("com.app.Test")),
            ("category", serde_json::json!("DataExfil.Keychain.Access")),
        ],
    )];

    let profiles = build_app_profiles(&records, None);
    assert_eq!(profiles.len(), 1);
    assert!(
        profiles[0]
            .capabilities
            .iter()
            .any(|c| c.kind == "Keychain"),
        "should detect keychain capability"
    );
}

#[test]
fn test_capability_detection_network() {
    let records = vec![make_record(
        "com.apple.tle.constraints.usage",
        vec![
            ("appIdentifier", serde_json::json!("com.app.Test")),
            ("category", serde_json::json!("Network.Outgoing.Connection")),
        ],
    )];

    let profiles = build_app_profiles(&records, None);
    assert_eq!(profiles.len(), 1);
    assert!(
        profiles[0]
            .capabilities
            .iter()
            .any(|c| c.kind == "NetworkOutgoing"),
        "should detect network capability"
    );
}

#[test]
fn test_caps_string_all() {
    let mut profile = AppProfile {
        bundle_id: "test".into(),
        version: String::new(),
        active_seconds: 0,
        uptime_seconds: 0,
        foreground: false,
        activations: 0,
        launches: 0,
        capabilities: vec![
            AppCapability {
                kind: "Clipboard".into(),
                source_event: "test".into(),
            },
            AppCapability {
                kind: "Keychain".into(),
                source_event: "test".into(),
            },
            AppCapability {
                kind: "NetworkOutgoing".into(),
                source_event: "test".into(),
            },
        ],
        binaries: Vec::new(),
        security_apis: vec!["SecItemCopyMatching".into()],
        network: AppNetworkInfo::default(),
        hardware: Vec::new(),
        record_count: 1,
    };

    assert_eq!(profile.caps_string(), "CKNS");

    profile.capabilities.clear();
    profile.security_apis.clear();
    assert_eq!(profile.caps_string(), "");
}

#[test]
fn test_binary_fingerprint_extraction() {
    let records = vec![make_record(
        "com.apple.syspolicy.ExecutableMeasurement",
        vec![
            ("signingIdentifier", serde_json::json!("dev.zed.Zed")),
            ("cdhash", serde_json::json!("deadbeef")),
        ],
    )];

    let profiles = build_app_profiles(&records, None);
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].binaries.len(), 1);
    assert_eq!(profiles[0].binaries[0].cdhash, "deadbeef");
    assert_eq!(profiles[0].binaries[0].signing_id, "dev.zed.Zed");
}

#[test]
fn test_empty_records_empty_profiles() {
    let profiles = build_app_profiles(&[], None);
    assert!(profiles.is_empty());
}
