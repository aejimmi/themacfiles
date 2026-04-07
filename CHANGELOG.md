# Changelog

## v0.1.1

New:
- summary: WiFi session detail — network OUI, country code, band, data volume, and join/disconnect reasons reveal where you've been
- summary: per-app Bluetooth scanning breakdown — which apps scan, how often, and how many nearby devices they find
- summary: Safari browsing profile — search engine, tab count, searches tracked, form submissions, and page locales
- summary: privacy tool awareness — Apple knows when you use a VPN, content filter, DNS proxy, or Private Relay
- summary: location tracking profile — home detection, WiFi-based location queries, heartbeats, and POI tile downloads
- summary: per-app security API usage showing which apps call legacy keychain APIs
- summary: photos library profile — asset count, moments, face and scene analysis progress, iCloud Photos and Apple Music status
- summary: behavioral feedback domains listed in surveillance counters
- cli: dump-bluetooth-devices script extracts every BLE device your Mac has ever seen from system databases

## v0.1.0

New:
- app: per-app deep dive showing everything Apple tracks — usage time, capabilities, binaries, network, security APIs
- summary: "Your Machine" section reveals the device identity Apple has built — platform, OS version, WiFi radio, thermal state, power mode
- summary: app capability indicators showing which apps access clipboard, keychain, and network
- cli: fuzzy search across all telemetry fields when querying by app name

Fix:
- summary: cleaner output layout with dedicated section rendering
