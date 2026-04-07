//! Summary section formatters — private helpers for the summary report.

use crate::schema::{Insights, Summary};
use std::fmt::Write;

/// Format the binaries fingerprinted section.
pub(super) fn format_binaries_section(out: &mut String, ins: &Insights) {
    if ins.executables_measured == 0 && ins.fingerprinted_binaries.is_empty() {
        return;
    }

    let _ = writeln!(out, "\n--- Binaries Fingerprinted (CDHash) ---");
    let _ = writeln!(out, "  {} executables measured", ins.executables_measured);

    if !ins.fingerprinted_binaries.is_empty() {
        let _ = writeln!(
            out,
            "  {} unique binaries with cryptographic fingerprints:\n",
            ins.fingerprinted_binaries.len()
        );
        for fp in ins.fingerprinted_binaries.iter().take(10) {
            let id = if fp.signing_id.is_empty() {
                "(unsigned)".to_string()
            } else {
                fp.signing_id.clone()
            };
            let _ = writeln!(out, "  {}  {}", fp.cdhash, id);
        }
        if ins.fingerprinted_binaries.len() > 10 {
            let _ = writeln!(
                out,
                "  ... and {} more",
                ins.fingerprinted_binaries.len() - 10
            );
        }
    }

    let _ = writeln!(
        out,
        "\n  Apple has a cryptographic inventory of every binary on your machine."
    );
}

/// Format the ML models section, grouped by type and sorted.
pub(super) fn format_ml_section(out: &mut String, ins: &Insights) {
    if ins.ml_models.is_empty() {
        return;
    }

    let _ = writeln!(
        out,
        "\n--- {} ML Models Running On Your Data ---",
        ins.ml_models.len()
    );

    let mut named: Vec<&crate::schema::MlModelInsight> = Vec::new();
    let mut espresso: Vec<&crate::schema::MlModelInsight> = Vec::new();
    for m in &ins.ml_models {
        if m.name.starts_with("espresso:") || m.name == "espresso" {
            espresso.push(m);
        } else {
            named.push(m);
        }
    }
    named.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    espresso.sort_by(|a, b| a.name.cmp(&b.name));

    for m in &named {
        let cu = if m.compute_unit.is_empty() {
            String::new()
        } else {
            format!(" [{}]", m.compute_unit)
        };
        let _ = writeln!(out, "  {}{} ({})", m.name, cu, m.bundle);
    }

    if !espresso.is_empty() {
        // Count models by compute unit for a compact summary.
        let mut cu_counts: Vec<(&str, usize)> = Vec::new();
        for m in &espresso {
            let cu = if m.compute_unit.is_empty() {
                "unknown"
            } else {
                m.compute_unit.as_str()
            };
            if let Some(entry) = cu_counts.iter_mut().find(|(k, _)| *k == cu) {
                entry.1 += 1;
            } else {
                cu_counts.push((cu, 1));
            }
        }
        let cu_summary: Vec<String> = cu_counts
            .iter()
            .map(|(cu, n)| format!("{cu} \u{00d7}{n}"))
            .collect();
        let _ = writeln!(
            out,
            "  espresso ({}) {}",
            espresso.len(),
            cu_summary.join(" \u{00b7} ")
        );
    }
}

/// Categorize a prediction view name into a display group.
fn prediction_group(name: &str) -> &'static str {
    if name.starts_with("ITD") || name.starts_with("SP") {
        "Index / Storage"
    } else if name.contains("Entity")
        || name.contains("entity")
        || name.contains("person")
        || name.contains("Person")
        || name.contains("loi")
    {
        "Entity Relevance"
    } else if name.contains("Interaction") || name.contains("interaction") {
        "Interaction Tracking"
    } else if name.contains("Context") || name.contains("context") {
        "Context Signals"
    } else if name.contains("siri") || name.contains("Siri") {
        "Siri"
    } else if name.contains("Apps") || name.contains("apps") {
        "App Usage"
    } else {
        "Other"
    }
}

/// The display order for prediction groups.
const PREDICTION_GROUP_ORDER: &[&str] = &[
    "Context Signals",
    "Entity Relevance",
    "App Usage",
    "Interaction Tracking",
    "Siri",
    "Index / Storage",
    "Other",
];

/// Format the behavioral predictions section, grouped and sorted.
pub(super) fn format_predictions_section(out: &mut String, ins: &Insights) {
    if ins.intelligence_views.is_empty() {
        return;
    }

    let _ = writeln!(
        out,
        "\n--- {} Behavioral Predictions Generated ---",
        ins.intelligence_views.len()
    );

    let mut sorted = ins.intelligence_views.clone();
    sorted.sort_by(|a, b| {
        let ga = PREDICTION_GROUP_ORDER
            .iter()
            .position(|&g| g == prediction_group(a))
            .unwrap_or(usize::MAX);
        let gb = PREDICTION_GROUP_ORDER
            .iter()
            .position(|&g| g == prediction_group(b))
            .unwrap_or(usize::MAX);
        ga.cmp(&gb)
            .then_with(|| a.to_lowercase().cmp(&b.to_lowercase()))
    });

    // Group items, then render each group as a single compact line.
    let mut groups: Vec<(&str, Vec<&str>)> = Vec::new();
    for v in &sorted {
        let group = prediction_group(v);
        if let Some(entry) = groups.iter_mut().find(|(g, _)| *g == group) {
            entry.1.push(v.as_str());
        } else {
            groups.push((group, vec![v.as_str()]));
        }
    }
    for (group, items) in &groups {
        let _ = writeln!(out, "  [{}] {}", group, items.join(", "));
    }
}

/// Format the surveillance counters section.
pub(super) fn format_counters_section(out: &mut String, ins: &Insights) {
    out.push('\n');
    out.push_str("--- Surveillance Counters ---\n");
    if ins.bt_devices_found > 0 {
        let _ = writeln!(
            out,
            "  Bluetooth: {} unique devices detected nearby",
            ins.bt_devices_found
        );
    }
    if ins.wifi_scans > 0 {
        let _ = writeln!(out, "  WiFi: {} scan result records", ins.wifi_scans);
    }
    if ins.profiling_items > 0 {
        let _ = writeln!(
            out,
            "  Behavioral profile: {} unique items about you",
            ins.profiling_items
        );
    }
    if ins.enrichment_rules > 0 {
        let _ = writeln!(
            out,
            "  {} event enrichment rules injecting device state into events",
            ins.enrichment_rules
        );
    }
    if ins.total_event_types > 0 {
        let _ = writeln!(
            out,
            "  {} total event types defined (the full surveillance catalog)",
            ins.total_event_types
        );
    }
    if !ins.budget_disabled.is_empty() {
        let _ = writeln!(
            out,
            "  {} transforms hit their data budget cap and were throttled",
            ins.budget_disabled.len()
        );
    }
    if !ins.behavioral_domains.is_empty() {
        let _ = writeln!(
            out,
            "  Behavioral feedback loops: {}",
            ins.behavioral_domains.join(", ")
        );
    }
}

/// Format the data sinks section.
pub(super) fn format_sinks_section(out: &mut String, ins: &Insights) {
    if ins.data_sinks.is_empty() {
        return;
    }

    out.push_str("\n--- Where Your Data Goes ---\n");
    for s in &ins.data_sinks {
        let label = match s.name.as_str() {
            "Daily" => "Daily (submitted to Apple every day)",
            "Never" => "Never (local only / feeds other transforms)",
            "90Day" => "90Day (quarterly submission to Apple)",
            "da2" => "da2 (secondary Apple pipeline)",
            other => other,
        };
        let _ = writeln!(out, "  {} transforms -> {}", s.transform_count, label);
    }
}

/// Format the sampling section.
pub(super) fn format_sampling_section(out: &mut String, ins: &Insights) {
    let samp = &ins.sampling;
    if samp.collecting == 0 && samp.sampled_out == 0 {
        return;
    }

    out.push_str("\n--- Sampling (your device's lottery) ---\n");
    let _ = writeln!(
        out,
        "  {} transforms actively collecting on YOUR device",
        samp.collecting
    );
    let _ = writeln!(
        out,
        "  {} transforms you were sampled OUT of (not collecting)",
        samp.sampled_out
    );
    let _ = writeln!(
        out,
        "  {} transforms with no sampling (always collected)",
        samp.unsampled
    );
}

/// Format collection periods as a single punchy line with date range.
///
/// Produces e.g. `Tracking you since Jan 1 through Mar 31 — daily · weekly · monthly · quarterly`
pub(super) fn format_periods_inline(out: &mut String, periods: &[crate::schema::CollectionPeriod]) {
    let mut sorted: Vec<&crate::schema::CollectionPeriod> = periods.iter().collect();
    sorted.sort_by_key(|p| p.period_type);

    let earliest_start = sorted
        .iter()
        .map(|p| p.start_timestamp.as_str())
        .min()
        .unwrap_or("");
    let latest_end = sorted
        .iter()
        .map(|p| p.end_boundary.as_str())
        .max()
        .unwrap_or("");

    let start = format_short_date(earliest_start);
    let end = format_short_date(latest_end);

    let _ = write!(out, "Tracking you since {start} through {end} \u{2014} ");
    for (i, p) in sorted.iter().enumerate() {
        if i > 0 {
            out.push_str(" \u{00b7} ");
        }
        out.push_str(p.period_label());
    }
    out.push('\n');
}

/// Extract a short "Mon DD" date from an ISO timestamp like "2026-03-30T00:09:02".
fn format_short_date(ts: &str) -> String {
    let date_part = ts.split('T').next().unwrap_or(ts);
    let parts: Vec<&str> = date_part.split('-').collect();
    let (Some(mm), Some(dd)) = (parts.get(1), parts.get(2)) else {
        return date_part.to_string();
    };
    let month = match *mm {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        other => other,
    };
    let day: u32 = dd.parse().unwrap_or(0);
    format!("{month} {day}")
}

/// Format the device identity section — what Apple knows about your hardware.
pub(super) fn format_device_section(out: &mut String, ins: &Insights, summary: &Summary) {
    let dev = &ins.device;
    let has_device_info = !dev.platform.is_empty()
        || !dev.os_version.is_empty()
        || !dev.safari_version.is_empty()
        || !dev.wifi_radio.is_empty()
        || !dev.network_interface.is_empty()
        || !summary.queried_states.is_empty();

    if !has_device_info {
        return;
    }

    out.push_str("\n--- Your Machine ---\n");

    if !dev.platform.is_empty() || !dev.os_version.is_empty() {
        let platform = if dev.platform.is_empty() {
            "unknown"
        } else {
            &dev.platform
        };
        if dev.os_version.is_empty() {
            let _ = writeln!(out, "  Platform: {platform}");
        } else {
            let _ = writeln!(out, "  Platform: {platform} {}", dev.os_version);
        }
    }

    if !dev.model_hash.is_empty() {
        let _ = writeln!(out, "  Model ID: {} (opaque hash)", dev.model_hash);
    }

    if !dev.safari_version.is_empty() {
        let _ = writeln!(out, "  Safari: {}", dev.safari_version);
    }

    if !dev.wifi_radio.is_empty() {
        let label = wifi_label(&dev.wifi_radio);
        let _ = writeln!(out, "  WiFi: {label}");
    }

    if !dev.network_interface.is_empty() {
        let _ = writeln!(out, "  Network: {}", dev.network_interface);
    }

    if !dev.thermal_state.is_empty() {
        let _ = writeln!(out, "  Thermal: {}", dev.thermal_state);
    }

    if !dev.low_power_mode.is_empty() {
        let _ = writeln!(out, "  Low Power Mode: {}", dev.low_power_mode);
    }

    if !dev.ai_locale.is_empty() {
        let _ = writeln!(out, "  Apple Intelligence: locale={}", dev.ai_locale);
    }
}

/// Map WiFi radio tech codes to human-readable labels.
fn wifi_label(code: &str) -> String {
    let name = match code {
        "11B" => "WiFi 1",
        "11A" => "WiFi 2",
        "11G" => "WiFi 3",
        "11N" => "WiFi 4",
        "11AC" => "WiFi 5",
        "11AX" => "WiFi 6",
        "11BE" => "WiFi 7",
        _ => return code.to_string(),
    };
    format!("{code} ({name})")
}
