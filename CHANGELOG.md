# Changelog

New:
- decode: full telemetry dump from Apple's analyticsd databases with labeled fields, categories, and opt-out detection
- summary: one-command overview of everything Apple knows — apps, ML models, behavioral predictions, surveillance counters
- summary: data pipeline visibility showing where collected data goes and sampling lottery breakdown
- summary: binary fingerprint inventory with cryptographic hashes for every measured executable
- summary: background and foreground app tracking with activation and launch counts
- events: catalog of all registered event types with categories and transform counts
- filtering: narrow output by category, event name, opt-out-only, or record limit
- output: table and JSON formats for decode and events commands
- cli: defaults to /private/var/db/analyticsd, supports custom paths for offline analysis
