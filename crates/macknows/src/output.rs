//! Output formatting for terminal tables and JSON.

use crate::schema::{DecodedRecord, EventInfo, Summary};
use std::fmt::Write;
use tabled::{Table, settings::Style};

/// Format decoded records as a human-readable table grouped by category.
pub fn format_decode_table(records: &[DecodedRecord]) -> String {
    if records.is_empty() {
        return "No records found.".into();
    }

    let mut rows: Vec<[String; 5]> = Vec::with_capacity(records.len());
    for r in records {
        let event = r
            .event_names
            .first()
            .cloned()
            .unwrap_or_else(|| "(none)".into());
        let fields_str = format_fields(&r.fields);
        rows.push([
            r.category.to_string(),
            event,
            r.transform_name.clone(),
            r.config_type.clone(),
            fields_str,
        ]);
    }

    let header = ["Category", "Event", "Transform", "Config", "Fields"];
    let mut table_rows = vec![header.map(String::from)];
    table_rows.extend(rows);

    Table::new(table_rows).with(Style::rounded()).to_string()
}

/// Format decoded records as JSON.
pub fn format_decode_json(records: &[DecodedRecord]) -> serde_json::Result<String> {
    serde_json::to_string_pretty(records)
}

/// Format a summary as a human-readable report.
pub fn format_summary(summary: &Summary) -> String {
    let mut out = String::with_capacity(4096);
    let ins = &summary.insights;

    out.push_str("=== What Apple Knows About You ===\n\n");

    let _ = writeln!(out, "{} telemetry records collected this period", summary.total_records);
    if summary.opt_out_count > 0 {
        let _ = writeln!(
            out,
            "{} records collected DESPITE analytics being disabled",
            summary.opt_out_count
        );
    }
    out.push('\n');

    // Apps — the headline
    if !ins.apps.is_empty() {
        out.push_str("--- Apps You Used (with exact versions + timing) ---\n");
        let app_rows: Vec<[String; 4]> = ins
            .apps
            .iter()
            .map(|a| {
                [
                    a.name.clone(),
                    a.version.clone(),
                    format_duration(a.active_seconds),
                    format_duration(a.uptime_seconds),
                ]
            })
            .collect();
        let mut header = vec![["App".into(), "Version".into(), "Active".into(), "Uptime".into()]];
        header.extend(app_rows);
        let table = Table::new(header).with(Style::rounded()).to_string();
        out.push_str(&table);
        out.push('\n');
    }

    // ML models
    if !ins.ml_models.is_empty() {
        let _ = writeln!(
            out,
            "\n--- {} ML Models Running On Your Data ---",
            ins.ml_models.len()
        );
        for m in &ins.ml_models {
            let cu = if m.compute_unit.is_empty() {
                String::new()
            } else {
                format!(" [{}]", m.compute_unit)
            };
            let _ = writeln!(out, "  {}{} ({})", m.name, cu, m.bundle);
        }
    }

    // Intelligence views
    if !ins.intelligence_views.is_empty() {
        let _ = writeln!(
            out,
            "\n--- {} Behavioral Predictions Generated ---",
            ins.intelligence_views.len()
        );
        for v in &ins.intelligence_views {
            let _ = writeln!(out, "  {v}");
        }
    }

    // One-liners
    out.push('\n');
    out.push_str("--- Surveillance Counters ---\n");
    if ins.bt_devices_found > 0 {
        let _ = writeln!(out, "  Bluetooth: {} unique devices detected nearby", ins.bt_devices_found);
    }
    if ins.wifi_scans > 0 {
        let _ = writeln!(out, "  WiFi: {} scan result records", ins.wifi_scans);
    }
    if ins.executables_measured > 0 {
        let _ = writeln!(out, "  Binaries measured: {} executables fingerprinted", ins.executables_measured);
    }
    if ins.profiling_items > 0 {
        let _ = writeln!(out, "  Behavioral profile: {} unique items about you", ins.profiling_items);
    }

    // Category breakdown
    out.push_str("\n--- Records by Category ---\n");
    let cat_rows: Vec<[String; 2]> = summary
        .category_counts
        .iter()
        .map(|(cat, count)| [cat.to_string(), count.to_string()])
        .collect();

    if !cat_rows.is_empty() {
        let mut header_rows = vec![["Category".into(), "Records".into()]];
        header_rows.extend(cat_rows);
        let table = Table::new(header_rows).with(Style::rounded()).to_string();
        out.push_str(&table);
        out.push('\n');
    }

    // Collection periods
    if !summary.collection_periods.is_empty() {
        out.push_str("\n--- Collection Periods ---\n");
        for period in &summary.collection_periods {
            let start = &period.start_timestamp;
            let end = &period.end_boundary;
            let _ = writeln!(out, "  {} | {} -> {}", period.period_label(), start, end);
        }
    }

    // Device state
    if !summary.queried_states.is_empty() {
        out.push_str("\n--- Device State ---\n");
        for (k, v) in &summary.queried_states {
            let _ = writeln!(out, "  {k}: {v}");
        }
    }

    out
}

/// Format seconds into a human-readable duration.
fn format_duration(secs: i64) -> String {
    if secs <= 0 {
        return "0s".into();
    }
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

/// Format event info as a human-readable table.
pub fn format_events_table(events: &[EventInfo]) -> String {
    if events.is_empty() {
        return "No events found.".into();
    }

    let mut table_rows = vec![["Event".into(), "Category".into(), "Transforms".into()]];

    for e in events {
        table_rows.push([
            e.event_name.clone(),
            e.category.to_string(),
            e.transform_count.to_string(),
        ]);
    }

    Table::new(table_rows).with(Style::rounded()).to_string()
}

/// Format event info as JSON.
pub fn format_events_json(events: &[EventInfo]) -> serde_json::Result<String> {
    serde_json::to_string_pretty(events)
}

/// Format a vector of labeled fields as a compact display string.
fn format_fields(fields: &[(String, serde_json::Value)]) -> String {
    if fields.is_empty() {
        return "(empty)".into();
    }

    fields
        .iter()
        .map(|(name, val)| format!("{name}={}", format_value(val)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Format a single JSON value for display, keeping it compact.
fn format_value(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => "null".into(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => val.to_string(),
    }
}

/// Format a microsecond timestamp as a human-readable date string.
fn format_timestamp(microseconds: i64) -> String {
    // Convert microseconds to seconds
    let secs = microseconds / 1_000_000;
    // Basic UTC formatting without pulling in chrono
    let days_since_epoch = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;

    // Simple date calculation from days since Unix epoch
    let (year, month, day) = days_to_ymd(days_since_epoch);
    format!("{year:04}-{month:02}-{day:02} {hours:02}:{minutes:02} UTC")
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(days: i64) -> (i64, i64, i64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
#[path = "output_test.rs"]
mod output_test;
