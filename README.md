# macstalker

Your Mac records which apps you use and for how long, which ML models run on your data, how many Bluetooth devices are near you, which executables you launch, and how Apple Intelligence profiles your behavior. It does this even when every analytics toggle in System Settings is off.

Two SQLite databases in `/private/var/db/analyticsd/` hold everything. `macstalker` decodes them into plain language.

## Quick start

```
cargo install --path crates/macstalker
sudo macstalker summary
```

## Commands

**`summary`** — the full picture in one shot. Record counts, app usage with active time, ML models loaded on-device, behavioral predictions, Bluetooth/WiFi surveillance counters, and how many of those records were collected despite opting out.

```
sudo macstalker summary
```

**`decode`** — every telemetry record, labeled and categorized.

```
sudo macstalker decode
sudo macstalker decode --category ai
sudo macstalker decode --event appUsage --json
sudo macstalker decode --opt-out-only --limit 50
```

**`events`** — catalog of all event types Apple has registered on your machine, with categories and transform counts.

```
sudo macstalker events
sudo macstalker events --category network --json
```

## What it surfaces

- **Apps** — every app tracked, with version, active foreground time, and total uptime
- **AI** — CoreML models, Espresso neural engine models, compute units (CPU/ANE/GPU), and which apps loaded them
- **Behavioral** — Apple Intelligence prediction views and personalization profiling item counts
- **Location** — WiFi positioning, geofencing, location visit telemetry
- **Network** — Bluetooth devices found nearby, WiFi scan records
- **Security** — executables fingerprinted by syspolicy
- **Media** — photo analysis, image recognition, camera sessions
- **Comms** — Messages, Siri, keyboard telemetry
- **Safari** — browsing, autofill, extensions, search, tabs
- **System** — power, memory, scheduling, cleanup telemetry

Each record shows whether it came from a `Main` or `OptOut` config — so you can see exactly what Apple collects regardless of your settings.

## Offline analysis

The databases require root access. To analyze on another machine or without repeated `sudo`:

```
sudo cp -r /private/var/db/analyticsd/ ~/analyticsd-copy
macstalker summary ~/analyticsd-copy
```

`macstalker` copies databases to a temp directory before reading, so it never holds locks on the live files.

## Requirements

- macOS (the databases are macOS-specific)
- Rust 1.94+
