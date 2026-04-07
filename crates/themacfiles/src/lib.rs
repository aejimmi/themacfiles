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

pub mod app_profile;
pub mod category;
pub mod db;
pub mod decode;
pub mod error;
pub mod extract;
pub mod output;
pub mod schema;

#[cfg(test)]
mod testutil;

use crate::error::{MacfilesError, Result};
use crate::schema::{DecodedRecord, EventInfo, Summary};
use rusqlite::Connection;
use std::collections::HashMap;
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

/// Build app profiles from decoded telemetry, optionally filtered by a query string.
///
/// Returns a list of [`AppProfile`](schema::AppProfile) grouped by bundle ID.
/// If `query` is `Some`, only profiles whose bundle ID contains the query
/// string (case-insensitive) are returned.
pub fn app_profiles_for(
    config_path: &Path,
    state_path: &Path,
    query: Option<&str>,
) -> Result<Vec<schema::AppProfile>> {
    let (config_conn, state_conn, _tmpdir) = open_databases(config_path, state_path)?;
    let records = decode::decode(&config_conn, &state_conn)?;
    Ok(app_profile::build_app_profiles(&records, query))
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
    let mut insights = extract::extract_insights(&records);

    // Enrich app insights with capability indicators from app profiles.
    let profiles = app_profile::build_app_profiles(&records, None);
    let caps_map: HashMap<String, String> = profiles
        .iter()
        .map(|p| (p.bundle_id.clone(), p.caps_string()))
        .collect();
    for app in &mut insights.apps {
        if let Some(caps) = caps_map.get(&app.name) {
            app.caps.clone_from(caps);
        }
    }

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

    // Enrich device insight from queried_states.
    for (k, v) in &queried_states {
        let clean = v.trim_matches('"');
        match k.as_str() {
            "lowPowerModeEnabled" => insights.device.low_power_mode = clean.to_string(),
            "thermalPressure" => insights.device.thermal_state = clean.to_string(),
            "wiFiRadioTech" => insights.device.wifi_radio = clean.to_string(),
            "primaryNetworkInterface" => insights.device.network_interface = clean.to_string(),
            _ => {}
        }
    }
    // Enrich device insight from decoded records.
    extract::extract_device_insights(&records, &mut insights.device);

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

/// Get a string field value by name.
pub(crate) fn field_str(fields: &[(String, serde_json::Value)], name: &str) -> Option<String> {
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
pub(crate) fn field_i64(fields: &[(String, serde_json::Value)], name: &str) -> i64 {
    fields
        .iter()
        .find_map(|(k, v)| if k == name { v.as_i64() } else { None })
        .unwrap_or(0)
}

/// Get a u64 field value by name.
pub(crate) fn field_u64(fields: &[(String, serde_json::Value)], name: &str) -> u64 {
    fields
        .iter()
        .find_map(|(k, v)| if k == name { v.as_u64() } else { None })
        .unwrap_or(0)
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
