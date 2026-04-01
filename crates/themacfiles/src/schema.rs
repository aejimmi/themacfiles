//! Types for representing analyticsd database contents.

use crate::category::Category;
use serde::{Deserialize, Serialize};

/// A transform definition parsed from the `transform_def` JSON column in config.sqlite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformDef {
    /// Human-readable name of the transform (e.g. "AppUsage_Aggregate").
    pub name: String,
    /// Unique identifier matching state.sqlite entries.
    #[serde(default)]
    pub uuid: String,
    /// Transform type (e.g. "aggregate", "sample").
    #[serde(rename = "type", default)]
    pub transform_type: String,
    /// Dimension columns — keys in the aggregation.
    #[serde(default)]
    pub dimensions: Vec<Dimension>,
    /// Measure columns — aggregated values.
    #[serde(default)]
    pub measures: Vec<Measure>,
}

/// A dimension (grouping key) within a transform definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimension {
    /// Column name for this dimension.
    pub name: String,
    /// Data type (e.g. "string", "int").
    #[serde(rename = "type", default)]
    pub dim_type: String,
}

/// A measure (aggregated value) within a transform definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measure {
    /// Column name for this measure.
    pub name: String,
    /// Aggregation function (e.g. "sum", "count").
    #[serde(default)]
    pub function: String,
    /// Data type (e.g. "int", "float").
    #[serde(rename = "type", default)]
    pub value_type: String,
}

/// A decoded telemetry record produced by joining config and state databases.
#[derive(Debug, Clone, Serialize)]
pub struct DecodedRecord {
    /// Event names associated with this transform.
    pub event_names: Vec<String>,
    /// Name of the transform that produced this record.
    pub transform_name: String,
    /// Telemetry category derived from event name.
    pub category: Category,
    /// Config type: "OptOut" or "Main".
    pub config_type: String,
    /// Whether the config was enabled.
    pub config_enabled: bool,
    /// Labeled field values: dimension names + measure names zipped with their values.
    pub fields: Vec<(String, serde_json::Value)>,
    /// Number of events aggregated into this record.
    pub event_count: u64,
}

/// Config metadata associated with a transform.
#[derive(Debug, Clone, Serialize)]
pub struct ConfigInfo {
    /// Config type: "OptOut" or "Main".
    pub config_type: String,
    /// Whether the config is enabled.
    pub config_enabled: bool,
}

/// Information about an event type from config.sqlite.
#[derive(Debug, Clone, Serialize)]
pub struct EventInfo {
    /// Fully qualified event name.
    pub event_name: String,
    /// Category derived from the event name.
    pub category: Category,
    /// Number of transforms associated with this event.
    pub transform_count: u32,
}

/// High-level summary of collected telemetry.
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    /// Number of decoded records per category.
    pub category_counts: Vec<(Category, usize)>,
    /// Number of records from OptOut configs.
    pub opt_out_count: usize,
    /// Number of records from Main configs.
    pub main_count: usize,
    /// Total decoded records.
    pub total_records: usize,
    /// Top events by record count.
    pub top_events: Vec<(String, usize)>,
    /// Collection time periods from agg_session.
    pub collection_periods: Vec<CollectionPeriod>,
    /// Device state key-value pairs from queried_states.
    pub queried_states: Vec<(String, String)>,
    /// Extracted insights from the data.
    pub insights: Insights,
}

/// Extracted high-level insights — the "what Apple knows about you" highlights.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Insights {
    /// Apps tracked with name, version, and active seconds.
    pub apps: Vec<AppInsight>,
    /// ML models loaded on-device (name, bundle, compute unit).
    pub ml_models: Vec<MlModelInsight>,
    /// Intelligence views generated (behavioral predictions).
    pub intelligence_views: Vec<String>,
    /// Bluetooth devices found nearby.
    pub bt_devices_found: u64,
    /// WiFi networks scanned.
    pub wifi_scans: usize,
    /// Executable binaries measured by syspolicy.
    pub executables_measured: usize,
    /// Fingerprinted binaries with CDHash and signing ID.
    pub fingerprinted_binaries: Vec<BinaryFingerprint>,
    /// Behavioral profiling domains and item counts.
    pub profiling_items: u64,
    /// Data pipeline: where collected data gets sent.
    pub data_sinks: Vec<SinkInfo>,
    /// Sampling: how many transforms are actively collected vs sampled out.
    pub sampling: SamplingInfo,
    /// Event enrichment rules that inject device state into events.
    pub enrichment_rules: usize,
    /// Total event types defined in config.
    pub total_event_types: usize,
    /// Transforms disabled by Apple (hit budget cap).
    pub budget_disabled: Vec<String>,
    /// Device identity information extracted from telemetry.
    pub device: DeviceInsight,
}

/// Device identity information Apple has collected.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DeviceInsight {
    /// Platform (e.g. "macOS").
    pub platform: String,
    /// OS version range (e.g. "26.0-26.1").
    pub os_version: String,
    /// Safari version string (e.g. "622.2.11.11.9").
    pub safari_version: String,
    /// WiFi radio technology (e.g. "11AX" = WiFi 6).
    pub wifi_radio: String,
    /// Primary network interface (e.g. "WiFi").
    pub network_interface: String,
    /// Thermal pressure level (e.g. "Nominal").
    pub thermal_state: String,
    /// Low-power mode status.
    pub low_power_mode: String,
    /// Device model hash (opaque identifier from Tips URL).
    pub model_hash: String,
    /// Apple Intelligence locale.
    pub ai_locale: String,
}

/// Where collected data goes.
#[derive(Debug, Clone, Serialize)]
pub struct SinkInfo {
    /// Sink name (Daily, Never, 90Day, da2).
    pub name: String,
    /// Number of transforms feeding this sink.
    pub transform_count: usize,
}

/// Sampling breakdown across transforms.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SamplingInfo {
    /// Transforms actively collecting on this device.
    pub collecting: usize,
    /// Transforms this device was sampled OUT of.
    pub sampled_out: usize,
    /// Transforms with no sampling (always collected).
    pub unsampled: usize,
}

/// An app tracked by appUsage.
#[derive(Debug, Clone, Serialize)]
pub struct AppInsight {
    /// Bundle identifier or description.
    pub name: String,
    /// App version string.
    pub version: String,
    /// Seconds of active (foreground) use.
    pub active_seconds: i64,
    /// Total uptime in seconds.
    pub uptime_seconds: i64,
    /// Whether the app was used in the foreground.
    pub foreground: bool,
    /// Number of times the user switched to this app.
    pub activations: i64,
    /// Number of times the app was launched.
    pub launches: i64,
    /// Short capability indicators: C=clipboard, K=keychain, N=network, S=security.
    pub caps: String,
}

/// A binary fingerprinted by syspolicy ExecutableMeasurement.
#[derive(Debug, Clone, Serialize)]
pub struct BinaryFingerprint {
    /// Content hash of the binary (CDHash).
    pub cdhash: String,
    /// Code-signing identifier.
    pub signing_id: String,
}

/// An ML model loaded on-device.
#[derive(Debug, Clone, Serialize)]
pub struct MlModelInsight {
    /// Model name (e.g. "punc_model").
    pub name: String,
    /// Bundle that loaded it.
    pub bundle: String,
    /// Compute unit (CPU, ANE, GPU).
    pub compute_unit: String,
}

/// A collection time period from the agg_session table.
#[derive(Debug, Clone, Serialize)]
pub struct CollectionPeriod {
    /// Start timestamp (ISO text, e.g. "2026-03-30T00:09:02").
    pub start_timestamp: String,
    /// End boundary timestamp (ISO text).
    pub end_boundary: String,
    /// Period type: 0=daily, 1=weekly, 2=monthly, 3=quarterly.
    pub period_type: i64,
}

impl CollectionPeriod {
    /// Returns a human-readable label for the period type.
    pub fn period_label(&self) -> &'static str {
        match self.period_type {
            0 => "daily",
            1 => "weekly",
            2 => "monthly",
            3 => "quarterly",
            _ => "unknown",
        }
    }
}

/// A raw row from the transform_states table before decoding.
#[derive(Debug, Clone)]
pub struct TransformStateRow {
    /// The transform UUID linking to config.sqlite.
    pub transform_uuid: String,
    /// JSON array of dimension values.
    pub transform_key: String,
    /// JSON array of measure values.
    pub transform_value: String,
    /// Number of events aggregated.
    pub event_count: u64,
}

/// Everything Apple tracks about a single application.
#[derive(Debug, Clone, Serialize)]
pub struct AppProfile {
    /// Bundle identifier (e.g. "com.apple.Safari").
    pub bundle_id: String,
    /// App version string.
    pub version: String,
    /// Seconds of active (foreground) use.
    pub active_seconds: i64,
    /// Total uptime in seconds.
    pub uptime_seconds: i64,
    /// Whether the app was used in the foreground.
    pub foreground: bool,
    /// Number of times the user switched to this app.
    pub activations: i64,
    /// Number of times the app was launched.
    pub launches: i64,
    /// Detected capabilities (clipboard, keychain, network access).
    pub capabilities: Vec<AppCapability>,
    /// Fingerprinted binaries associated with this app.
    pub binaries: Vec<BinaryFingerprint>,
    /// Security API calls detected.
    pub security_apis: Vec<String>,
    /// Network transfer information.
    pub network: AppNetworkInfo,
    /// Hardware-related key-value pairs (GPU, CPU, Metal, memory, thermal).
    pub hardware: Vec<(String, String)>,
    /// Total number of telemetry records referencing this app.
    pub record_count: usize,
}

impl AppProfile {
    /// Short capability string: C=clipboard, K=keychain, N=network, S=security.
    pub fn caps_string(&self) -> String {
        let mut s = String::with_capacity(4);
        if self.capabilities.iter().any(|c| c.kind == "Clipboard") {
            s.push('C');
        }
        if self.capabilities.iter().any(|c| c.kind == "Keychain") {
            s.push('K');
        }
        if self
            .capabilities
            .iter()
            .any(|c| c.kind == "NetworkOutgoing")
        {
            s.push('N');
        }
        if !self.security_apis.is_empty() {
            s.push('S');
        }
        s
    }
}

/// A detected capability for an application.
#[derive(Debug, Clone, Serialize)]
pub struct AppCapability {
    /// Capability kind: "Clipboard", "Keychain", "NetworkOutgoing".
    pub kind: String,
    /// The event that revealed this capability.
    pub source_event: String,
}

/// Network transfer information for an application.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AppNetworkInfo {
    /// Network interface type (e.g. "WiFi", "Cellular").
    pub interface: String,
    /// Raw network byte values from records.
    pub bytes_values: Vec<i64>,
}
