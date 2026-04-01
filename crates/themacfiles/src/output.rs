//! Output formatting for terminal tables and JSON.

#[path = "output_summary.rs"]
mod output_summary;

use crate::schema::{AppProfile, DecodedRecord, EventInfo, Summary};
use output_summary::{
    format_binaries_section, format_counters_section, format_device_section, format_ml_section,
    format_periods_inline, format_predictions_section, format_sampling_section,
    format_sinks_section,
};
use std::fmt::Write;
use tabled::{
    Table,
    settings::{Remove, Style, object::Rows},
};

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
    let mut out = String::with_capacity(8192);
    let ins = &summary.insights;

    // 1. Header with total records
    out.push_str("=== What Apple Knows About You ===\n\n");

    let _ = writeln!(
        out,
        "{} telemetry records collected this period",
        summary.total_records
    );
    if !summary.collection_periods.is_empty() {
        format_periods_inline(&mut out, &summary.collection_periods);
    }
    if summary.opt_out_count > 0 {
        let _ = writeln!(
            out,
            "{} records collected DESPITE analytics being disabled",
            summary.opt_out_count
        );
    }
    out.push('\n');

    // 2. Apps — full list, foreground marked with *
    format_apps_section(&mut out, ins);

    // 3. Binaries Fingerprinted
    format_binaries_section(&mut out, ins);

    // 4. ML Models
    format_ml_section(&mut out, ins);

    // 5. Behavioral Predictions
    format_predictions_section(&mut out, ins);

    // 6. Your Machine
    format_device_section(&mut out, ins, summary);

    // 7. Surveillance Counters
    format_counters_section(&mut out, ins);

    // 8. Where Your Data Goes
    format_sinks_section(&mut out, ins);

    // 9. Sampling
    format_sampling_section(&mut out, ins);

    out
}

/// Format the apps section with foreground/background distinction and capability indicators.
fn format_apps_section(out: &mut String, ins: &crate::schema::Insights) {
    if ins.apps.is_empty() {
        return;
    }

    let fg_count = ins.apps.iter().filter(|a| a.foreground).count();
    let bg_count = ins.apps.len() - fg_count;
    let _ = writeln!(
        out,
        "--- Apps You Used ({fg_count} foreground, {bg_count} background) ---",
    );
    let _ = writeln!(out, "  (* = foreground app)\n");

    let has_caps = ins.apps.iter().any(|a| !a.caps.is_empty());

    let app_rows: Vec<[String; 7]> = ins
        .apps
        .iter()
        .map(|a| {
            let marker = if a.foreground { "*" } else { " " };
            let name = format!("{}{}", marker, a.name);
            [
                name,
                a.version.clone(),
                format_duration(a.active_seconds),
                format_duration(a.uptime_seconds),
                a.activations.to_string(),
                a.launches.to_string(),
                a.caps.clone(),
            ]
        })
        .collect();

    let mut header = vec![[
        "App".into(),
        "Version".into(),
        "Active".into(),
        "Uptime".into(),
        "Activations".into(),
        "Launches".into(),
        "Caps".into(),
    ]];
    header.extend(app_rows);
    let table = Table::new(header)
        .with(Remove::row(Rows::first()))
        .with(Style::rounded())
        .to_string();
    out.push_str(&table);

    if has_caps {
        let _ = writeln!(out, "  C=clipboard K=keychain N=network S=security");
    }
    out.push('\n');
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

/// Format a detailed per-app profile for terminal display.
pub fn format_app_profile(profiles: &[AppProfile]) -> String {
    if profiles.is_empty() {
        return "No matching app profiles found.".into();
    }

    let mut out = String::with_capacity(4096);

    for p in profiles {
        let _ = writeln!(out, "=== {} ===", p.bundle_id);
        if !p.version.is_empty() {
            let _ = writeln!(out, "  Version: {}", p.version);
        }
        let _ = writeln!(out, "  Records: {}", p.record_count);

        if p.active_seconds > 0 || p.uptime_seconds > 0 {
            let _ = writeln!(
                out,
                "  Active: {}  Uptime: {}  Foreground: {}",
                format_duration(p.active_seconds),
                format_duration(p.uptime_seconds),
                if p.foreground { "yes" } else { "no" },
            );
            let _ = writeln!(
                out,
                "  Activations: {}  Launches: {}",
                p.activations, p.launches,
            );
        }

        if !p.capabilities.is_empty() {
            let _ = writeln!(out, "\n  Capabilities:");
            for cap in &p.capabilities {
                let _ = writeln!(out, "    {} ({})", cap.kind, cap.source_event);
            }
        }

        if !p.security_apis.is_empty() {
            let _ = writeln!(out, "\n  Security APIs:");
            for api in &p.security_apis {
                let _ = writeln!(out, "    {api}");
            }
        }

        if !p.binaries.is_empty() {
            let _ = writeln!(out, "\n  Binaries Fingerprinted:");
            for b in &p.binaries {
                let id = if b.signing_id.is_empty() {
                    "(unsigned)"
                } else {
                    &b.signing_id
                };
                let _ = writeln!(out, "    {}  {}", b.cdhash, id);
            }
        }

        if !p.network.interface.is_empty() || !p.network.bytes_values.is_empty() {
            let _ = writeln!(out, "\n  Network:");
            if !p.network.interface.is_empty() {
                let _ = writeln!(out, "    Interface: {}", p.network.interface);
            }
            if !p.network.bytes_values.is_empty() {
                let _ = writeln!(out, "    Byte values: {:?}", p.network.bytes_values);
            }
        }

        if !p.hardware.is_empty() {
            let _ = writeln!(out, "\n  Hardware:");
            for (k, v) in &p.hardware {
                let _ = writeln!(out, "    {k}: {v}");
            }
        }

        out.push('\n');
    }

    out
}

/// Format app profiles as JSON.
pub fn format_app_profile_json(profiles: &[AppProfile]) -> serde_json::Result<String> {
    serde_json::to_string_pretty(profiles)
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
#[cfg(test)]
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
#[cfg(test)]
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
