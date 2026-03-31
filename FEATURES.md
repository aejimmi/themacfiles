# Features

## Telemetry Decoding

- Full database decode — reads Apple's analyticsd config.sqlite and state.sqlite, cross-references transform definitions with collected state data, and produces labeled, categorized records.
- Labeled fields — dimension and measure names from transform definitions are zipped with raw JSON values so every data point has a human-readable name.
- OptOut detection — identifies records collected regardless of whether the user disabled analytics, separate from Main config records.
- Safe read-only access — copies databases (including WAL/SHM files) to a temp directory before opening, avoiding lock contention with the live analyticsd process.

## Category Classification

- Automatic categorization — classifies every event into one of 11 categories (Apps, Location, Network, AI, Behavioral, Media, Comms, Security, Safari, System, Other) based on event name prefix matching.
- 36 prefix rules — covers Apple telemetry domains including CoreML, Espresso, WiFi, Bluetooth, locationd, syspolicy, Safari, Messages, Siri, Photos, and more.

## Summary Report

- "What Apple Knows About You" overview — single-command summary showing total records, opt-out collection counts, app usage with active/uptime durations, ML models running on-device, behavioral predictions, and surveillance counters.
- App usage tracking — extracts app names, versions, active foreground time, total uptime, activation counts, and launch counts from appUsage telemetry, with foreground and background apps distinguished.
- ML model inventory — lists CoreML models and Espresso neural engine models with their bundle identifiers and compute units (CPU/ANE/GPU).
- Intelligence views — surfaces Apple Intelligence behavioral prediction view names.
- Surveillance counters — Bluetooth devices detected nearby, WiFi scan records, behavioral profiling item counts, enrichment rules injecting device state, total event types in the surveillance catalog, and budget-throttled transforms.
- Binary fingerprint inventory — cryptographic CDHash and signing identifier for every executable measured by syspolicy.
- Data pipeline visibility — shows where collected data goes (Daily, 90Day, da2 pipelines) and how many transforms feed each destination.
- Sampling breakdown — how many transforms are actively collecting on your device, how many you were sampled out of, and how many always collect regardless.
- Collection periods — shows daily, weekly, monthly, and quarterly aggregation session time ranges.
- Device state — displays queried state key-value pairs stored by the system.
- Top events — ranks the 20 most frequent event types by record count.

## Event Listing

- Event catalog — lists all event types from config.sqlite with their categories and associated transform counts, sorted by transform count.

## Filtering

- Category filter — narrow any command output to a single telemetry category.
- Event name filter — substring match on event names to find specific telemetry.
- OptOut-only filter — show only data collected despite analytics being disabled.
- Record limit — cap the number of output records.

## Output Formats

- Table output — formatted terminal tables with rounded borders for decode, summary, and events commands.
- JSON output — structured JSON for decode and events commands, suitable for piping to other tools.
- Human-readable durations — active time and uptime displayed as hours/minutes/seconds.

## CLI

- Three subcommands — `decode` (full telemetry dump), `summary` (high-level overview), `events` (event type catalog).
- Auto-detection — defaults to `/private/var/db/analyticsd` so `sudo macstalker summary` works immediately.
- Custom path — point at any directory containing copied databases for offline analysis.
