//! Detail section formatters for the summary report -- WiFi, Bluetooth, Safari,
//! Privacy, Location, Security APIs, and Photos.

use crate::schema::Insights;
use std::fmt::Write;

/// Format WiFi sessions showing network details and travel patterns.
pub(super) fn format_wifi_section(out: &mut String, ins: &Insights) {
    if ins.wifi_sessions.is_empty() {
        return;
    }
    let _ = writeln!(
        out,
        "\n--- WiFi Sessions \u{2014} Where You\u{2019}ve Been ---"
    );
    for s in &ins.wifi_sessions {
        let hotspot = if s.is_personal_hotspot {
            " [hotspot]"
        } else {
            ""
        };
        let wpa3 = if s.has_wpa3 { " WPA3" } else { "" };
        let _ = writeln!(
            out,
            "  OUI {} \u{2014} country:{} band:{}GHz{}{} mac:{}",
            s.oui, s.country, s.band, hotspot, wpa3, s.private_mac_type
        );
        let _ = writeln!(
            out,
            "    join:{} disconnect:{} duration:{}s rx:{} tx:{}",
            s.join_reason,
            s.disconnect_reason,
            s.session_duration_secs,
            format_bytes(s.rx_bytes),
            format_bytes(s.tx_bytes),
        );
    }
    format_wifi_countries(out, ins);
}

/// Emit a note when multiple country codes reveal travel patterns.
fn format_wifi_countries(out: &mut String, ins: &Insights) {
    let countries: Vec<&str> = ins
        .wifi_sessions
        .iter()
        .map(|s| s.country.as_str())
        .filter(|c| !c.is_empty())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    if countries.len() > 1 {
        let _ = writeln!(
            out,
            "\n  Country codes ({}) reveal travel patterns without GPS.",
            countries.join(", ")
        );
    }
}

/// Format Bluetooth scan detail per scanning application.
pub(super) fn format_bt_detail_section(out: &mut String, ins: &Insights) {
    if ins.bt_scans.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n--- Bluetooth \u{2014} Who\u{2019}s Scanning ---");
    for s in &ins.bt_scans {
        let use_case = if s.use_case.is_empty() || s.use_case == "Unspecified" {
            String::new()
        } else {
            format!(" ({})", s.use_case)
        };
        let _ = writeln!(
            out,
            "  {} \u{2014} {} sessions, max {} devices found, {} paired{}",
            s.bundle_id, s.scan_count, s.max_devices_found, s.paired_found, use_case,
        );
    }
}

/// Format Safari browsing profile.
pub(super) fn format_safari_section(out: &mut String, ins: &Insights) {
    let s = &ins.safari;
    if s.search_engine.is_empty() && s.tab_count == 0 && s.search_count == 0 {
        return;
    }
    let _ = writeln!(out, "\n--- Safari \u{2014} Your Browsing Profile ---");
    if !s.search_engine.is_empty() {
        let _ = writeln!(out, "  Search engine: {}", s.search_engine);
    }
    if s.tab_count > 0 {
        let _ = writeln!(out, "  Typical tab count: ~{}", s.tab_count);
    }
    if s.search_count > 0 {
        let _ = writeln!(out, "  Searches tracked: {}", s.search_count);
    }
    if s.form_submissions > 0 {
        let _ = writeln!(out, "  Form submissions: {}", s.form_submissions);
    }
    if !s.user_region.is_empty() {
        let _ = writeln!(out, "  User region: {}", s.user_region);
    }
    if !s.webpage_locales.is_empty() {
        let _ = writeln!(
            out,
            "  Page locales visited: {}",
            s.webpage_locales.join(", ")
        );
    }
}

/// Format privacy tool awareness section.
pub(super) fn format_privacy_section(out: &mut String, ins: &Insights) {
    let p = &ins.privacy;
    if !p.vpn_detected && !p.content_filter_detected && p.private_relay_status.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n--- Privacy Tools \u{2014} Apple Knows ---");
    if p.vpn_detected {
        let _ = writeln!(out, "  VPN: detected");
    }
    if p.content_filter_detected {
        let _ = writeln!(out, "  Content filter: detected");
    }
    if p.dns_proxy_detected {
        let _ = writeln!(out, "  DNS proxy: detected");
    }
    if !p.private_relay_status.is_empty() {
        let _ = writeln!(out, "  iCloud Private Relay: {}", p.private_relay_status);
    }
    if p.dns_stalls > 0 {
        let _ = writeln!(out, "  DNS stalls observed: {}", p.dns_stalls);
    }
    if p.connection_failures > 0 {
        let _ = writeln!(out, "  Connection failures: {}", p.connection_failures);
    }
}

/// Format location tracking profile.
pub(super) fn format_location_section(out: &mut String, ins: &Insights) {
    let l = &ins.location;
    if !l.home_detected && l.location_queries == 0 && l.heartbeat_count == 0 {
        return;
    }
    let _ = writeln!(out, "\n--- Location \u{2014} Tracking Your Movements ---");
    if l.home_detected {
        let _ = writeln!(out, "  Home location: identified");
    }
    if l.location_queries > 0 {
        let _ = writeln!(out, "  WiFi-based location queries: {}", l.location_queries);
    }
    if l.heartbeat_count > 0 {
        let _ = writeln!(
            out,
            "  Location awareness heartbeats: {}",
            l.heartbeat_count
        );
    }
    if l.poi_tile_downloads {
        let _ = writeln!(out, "  POI map tiles: pre-downloaded on battery");
    }
}

/// Format per-app security API usage.
pub(super) fn format_security_api_section(out: &mut String, ins: &Insights) {
    if ins.security_apis.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n--- Security APIs \u{2014} App Keychain Access ---");
    for s in &ins.security_apis {
        let _ = writeln!(out, "  {}: {}", s.app, s.apis.join(", "));
    }
}

/// Format photos library profile.
pub(super) fn format_photos_section(out: &mut String, ins: &Insights) {
    let p = &ins.photos;
    if p.total_assets == 0 && p.moments == 0 && p.library_size.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n--- Photos \u{2014} Your Library Profile ---");
    if p.total_assets > 0 {
        let _ = writeln!(out, "  Total assets: {}", p.total_assets);
    }
    if p.moments > 0 {
        let _ = writeln!(out, "  Moments: {}", p.moments);
    }
    if !p.library_size.is_empty() {
        let _ = writeln!(out, "  Library size: {}", p.library_size);
    }
    let _ = writeln!(
        out,
        "  Apple Music: {}",
        if p.has_apple_music {
            "subscribed"
        } else {
            "no"
        }
    );
    let _ = writeln!(
        out,
        "  iCloud Photos: {}",
        if p.icloud_photos_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    format_photos_analysis(out, p);
}

/// Format face and scene analysis progress lines.
fn format_photos_analysis(out: &mut String, p: &crate::schema::PhotosInsight) {
    if p.face_analysis_progress > 0.0 {
        let _ = writeln!(
            out,
            "  Face analysis: {:.0}%",
            p.face_analysis_progress * 100.0
        );
    }
    if p.scene_analysis_progress > 0.0 {
        let _ = writeln!(
            out,
            "  Scene analysis: {:.0}%",
            p.scene_analysis_progress * 100.0
        );
    }
}

/// Format byte counts as human-readable sizes.
fn format_bytes(bytes: i64) -> String {
    if bytes <= 0 {
        return "0B".into();
    }
    let abs = bytes as f64;
    if abs >= 1_073_741_824.0 {
        format!("{:.1}GB", abs / 1_073_741_824.0)
    } else if abs >= 1_048_576.0 {
        format!("{:.1}MB", abs / 1_048_576.0)
    } else if abs >= 1024.0 {
        format!("{:.1}KB", abs / 1024.0)
    } else {
        format!("{bytes}B")
    }
}
