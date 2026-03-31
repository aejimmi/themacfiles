//! themacfiles — Apple telemetry decoder library.
//!
//! Reads the analyticsd SQLite databases (`config.sqlite` and `state.sqlite`)
//! and cross-references transform definitions with collected state data to
//! produce labeled, categorized, human-readable telemetry records.
//!
//! # Usage
//!
//! ```no_run
//! use std::path::Path;
//!
//! let config = Path::new("/private/var/db/analyticsd/config.sqlite");
//! let state = Path::new("/private/var/db/analyticsd/state.sqlite");
//!
//! let records = themacfiles::decode_databases(config, state).unwrap();
//! let events = themacfiles::list_events(config).unwrap();
//! let summary = themacfiles::summary(config, state).unwrap();
//! ```

pub mod category;
pub mod db;
pub mod decode;
pub mod error;
pub mod output;
pub mod schema;

#[cfg(test)]
mod testutil;

use crate::error::{MacfilesError, Result};
use crate::schema::{
    AppInsight, BinaryFingerprint, DecodedRecord, EventInfo, Insights, MlModelInsight, Summary,
};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::warn;

/// Decode all collected telemetry from the analyticsd databases.
///
/// Opens both databases read-only, copies them to a temporary directory to
/// avoid WAL lock contention, then joins config transforms with state data.
pub fn decode_databases(config_path: &Path, state_path: &Path) -> Result<Vec<DecodedRecord>> {
    let (config_conn, state_conn, _tmpdir) = open_databases(config_path, state_path)?;
    decode::decode(&config_conn, &state_conn)
}

/// List all event types from config.sqlite with categories and transform counts.
pub fn list_events(config_path: &Path) -> Result<Vec<EventInfo>> {
    let (config_conn, _tmpdir) = open_single_db(config_path)?;
    db::load_events_with_counts(&config_conn)
}

/// Generate a high-level summary of collected telemetry.
pub fn summary(config_path: &Path, state_path: &Path) -> Result<Summary> {
    let (config_conn, state_conn, _tmpdir) = open_databases(config_path, state_path)?;
    let records = decode::decode(&config_conn, &state_conn)?;

    let mut category_counts: HashMap<crate::category::Category, usize> = HashMap::new();
    let mut event_counts: HashMap<String, usize> = HashMap::new();
    let mut opt_out_count = 0usize;
    let mut main_count = 0usize;

    for r in &records {
        *category_counts.entry(r.category).or_default() += 1;
        if let Some(name) = r.event_names.first() {
            *event_counts.entry(name.clone()).or_default() += 1;
        }
        match r.config_type.as_str() {
            "OptOut" => opt_out_count += 1,
            _ => main_count += 1,
        }
    }

    let mut cat_vec: Vec<_> = category_counts.into_iter().collect();
    cat_vec.sort_by(|a, b| b.1.cmp(&a.1));

    let mut top_events: Vec<_> = event_counts.into_iter().collect();
    top_events.sort_by(|a, b| b.1.cmp(&a.1));
    top_events.truncate(20);

    let collection_periods = db::load_agg_sessions(&state_conn)?;
    let queried_states = db::load_queried_states(&state_conn)?;
    let mut insights = extract_insights(&records);

    // Add config-level insights
    let sinks = db::load_sinks(&config_conn)?;
    insights.data_sinks = sinks
        .into_iter()
        .map(|(name, count)| schema::SinkInfo {
            name,
            transform_count: count,
        })
        .collect();

    let (collecting, sampled_out, unsampled) = db::load_sampling_info(&config_conn)?;
    insights.sampling = schema::SamplingInfo {
        collecting,
        sampled_out,
        unsampled,
    };

    insights.enrichment_rules = db::count_enrichment_rules(&config_conn)?;
    insights.total_event_types = db::count_events(&config_conn);
    insights.budget_disabled = db::load_budget_disabled(&config_conn)?;

    Ok(Summary {
        category_counts: cat_vec,
        opt_out_count,
        main_count,
        total_records: records.len(),
        top_events,
        collection_periods,
        queried_states,
        insights,
    })
}

/// Extract high-level insights from decoded records.
fn extract_insights(records: &[DecodedRecord]) -> Insights {
    let mut insights = Insights::default();
    let mut seen_apps: HashSet<String> = HashSet::new();
    let mut seen_models: HashSet<String> = HashSet::new();
    let mut seen_views: HashSet<String> = HashSet::new();
    let mut seen_binaries: HashSet<String> = HashSet::new();
    let mut max_bt_devices: u64 = 0;

    for r in records {
        let ev = r.event_names.first().map_or("", String::as_str);
        let fields = &r.fields;

        // App usage — include both foreground and background apps
        if ev == "com.apple.osanalytics.appUsage"
            && let Some(desc) = field_str(fields, "appDescription")
            && !seen_apps.contains(&desc)
        {
            seen_apps.insert(desc.clone());
            let is_fg = field_str(fields, "foreground").as_deref() == Some("YES");
            let (name, version) = parse_app_description(&desc);
            let active = field_i64(fields, "sum_of_activeTime");
            let uptime = field_i64(fields, "sum_of_uptime");
            let activations = field_i64(fields, "sum_of_activations");
            let launches = field_i64(fields, "sum_of_activityPeriods");
            insights.apps.push(AppInsight {
                name,
                version,
                active_seconds: active,
                uptime_seconds: uptime,
                foreground: is_fg,
                activations,
                launches,
            });
        }

        // ML models
        if ev == "com.apple.CoreML.MLLoader"
            && let Some(model_name) = field_str(fields, "modelName")
        {
            let bundle = field_str(fields, "bundleIdentifier").unwrap_or_default();
            let key = format!("{model_name}:{bundle}");
            if !seen_models.contains(&key) {
                seen_models.insert(key);
                insights.ml_models.push(MlModelInsight {
                    name: model_name,
                    bundle,
                    compute_unit: String::new(),
                });
            }
        }

        // Espresso (neural engine)
        if ev == "com.apple.Espresso.SegmentationAnalytics"
            && let Some(cu) = field_str(fields, "computeUnit")
        {
            let bundle = field_str(fields, "bundleIdentifier").unwrap_or_default();
            let hash = field_str(fields, "modelHash").unwrap_or_default();
            let key = format!("{hash}:{cu}");
            if !seen_models.contains(&key) {
                seen_models.insert(key);
                insights.ml_models.push(MlModelInsight {
                    name: format!("espresso:{}", &hash[..8.min(hash.len())]),
                    bundle,
                    compute_unit: cu,
                });
            }
        }

        // Intelligence views
        if ev.contains("intelligenceplatform.ViewGeneration")
            && let Some(view) = field_str(fields, "ViewName")
            && !seen_views.contains(&view)
        {
            seen_views.insert(view.clone());
            insights.intelligence_views.push(view);
        }

        // Bluetooth scan — track max devices found
        if ev == "com.apple.Bluetooth.LEScanSession" {
            let found = field_u64(fields, "NumberOfUniqueDevicesFound");
            if found > max_bt_devices {
                max_bt_devices = found;
            }
        }

        // WiFi scans
        if ev == "com.apple.wifi.scanResults" {
            insights.wifi_scans += 1;
        }

        // Executable measurement — count and extract fingerprints
        if ev == "com.apple.syspolicy.ExecutableMeasurement" {
            insights.executables_measured += 1;
            let cdhash = field_str(fields, "cdhash").unwrap_or_default();
            let signing_id = field_str(fields, "signingIdentifier").unwrap_or_default();
            if !cdhash.is_empty() && !seen_binaries.contains(&cdhash) {
                seen_binaries.insert(cdhash.clone());
                insights
                    .fingerprinted_binaries
                    .push(BinaryFingerprint { cdhash, signing_id });
            }
        }

        // Personalization portrait
        if ev.contains("PersonalizationPortrait.TopicStoreStats") {
            let items = field_u64(fields, "daily_maximum_uniqueItems");
            if items > insights.profiling_items {
                insights.profiling_items = items;
            }
        }
    }

    insights.bt_devices_found = max_bt_devices;
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
    insights
}

/// Get a string field value by name.
fn field_str(fields: &[(String, serde_json::Value)], name: &str) -> Option<String> {
    fields.iter().find_map(|(k, v)| {
        if k == name {
            match v {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Number(n) => Some(n.to_string()),
                _ => v.as_str().map(String::from),
            }
        } else {
            None
        }
    })
}

/// Get an i64 field value by name.
fn field_i64(fields: &[(String, serde_json::Value)], name: &str) -> i64 {
    fields
        .iter()
        .find_map(|(k, v)| if k == name { v.as_i64() } else { None })
        .unwrap_or(0)
}

/// Get a u64 field value by name.
fn field_u64(fields: &[(String, serde_json::Value)], name: &str) -> u64 {
    fields
        .iter()
        .find_map(|(k, v)| if k == name { v.as_u64() } else { None })
        .unwrap_or(0)
}

/// Parse "com.example.app ||| 1.2.3 (build)" into (name, version).
fn parse_app_description(desc: &str) -> (String, String) {
    if let Some((name, version)) = desc.split_once(" ||| ") {
        (name.trim().to_string(), version.trim().to_string())
    } else {
        (desc.to_string(), String::new())
    }
}

/// Open both databases by copying to a temp directory first.
///
/// Returns the connections and the [`tempfile::TempDir`] guard — the temp
/// directory is deleted when the guard is dropped, so the caller must hold it.
fn open_databases(
    config_path: &Path,
    state_path: &Path,
) -> Result<(Connection, Connection, tempfile::TempDir)> {
    validate_path(config_path)?;
    validate_path(state_path)?;

    let tmpdir = tempfile::TempDir::new().map_err(|e| MacfilesError::Io { source: e })?;

    let config_copy = tmpdir.path().join("config.sqlite");
    let state_copy = tmpdir.path().join("state.sqlite");

    std::fs::copy(config_path, &config_copy)?;
    std::fs::copy(state_path, &state_copy)?;

    // Also copy WAL/SHM files if they exist (for consistency)
    copy_if_exists(config_path, "config.sqlite-wal", tmpdir.path());
    copy_if_exists(config_path, "config.sqlite-shm", tmpdir.path());
    copy_if_exists(state_path, "state.sqlite-wal", tmpdir.path());
    copy_if_exists(state_path, "state.sqlite-shm", tmpdir.path());

    let config_conn =
        Connection::open_with_flags(&config_copy, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| MacfilesError::DatabaseOpen {
                source: e,
                path: config_path.to_path_buf(),
            })?;

    let state_conn =
        Connection::open_with_flags(&state_copy, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| MacfilesError::DatabaseOpen {
                source: e,
                path: state_path.to_path_buf(),
            })?;

    Ok((config_conn, state_conn, tmpdir))
}

/// Open a single database by copying to a temp directory first.
fn open_single_db(db_path: &Path) -> Result<(Connection, tempfile::TempDir)> {
    validate_path(db_path)?;

    let tmpdir = tempfile::TempDir::new().map_err(|e| MacfilesError::Io { source: e })?;

    let file_name = db_path
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("db.sqlite"));
    let copy_path = tmpdir.path().join(file_name);
    std::fs::copy(db_path, &copy_path)?;

    let conn = Connection::open_with_flags(&copy_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| MacfilesError::DatabaseOpen {
            source: e,
            path: db_path.to_path_buf(),
        })?;

    Ok((conn, tmpdir))
}

/// Validate that a database file exists.
fn validate_path(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(MacfilesError::DatabaseNotFound {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

/// Copy a sibling file (WAL/SHM) if it exists next to the source database.
fn copy_if_exists(db_path: &Path, sibling_name: &str, dest_dir: &Path) {
    if let Some(parent) = db_path.parent() {
        let sibling = parent.join(sibling_name);
        if sibling.exists()
            && let Err(e) = std::fs::copy(&sibling, dest_dir.join(sibling_name))
        {
            warn!(
                path = %sibling.display(),
                error = %e,
                "failed to copy WAL/SHM sibling file (non-fatal)"
            );
        }
    }
}
