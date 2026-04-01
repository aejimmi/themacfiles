# themacfiles

> "Privacy. That's Apple." — [apple.com/privacy](https://www.apple.com/privacy/)

In the first half of 2025, Apple received 82,088 government requests covering 994,059 devices, accounts, and financial identifiers — and complied with most of them. 9,528 included your photos, emails, and iCloud backups. ([source](https://www.apple.com/legal/transparency/))

That's the data they admit to sharing. What follows is the data they don't talk about.

Your Mac records which apps you use and for how long, which ML models run on your data, how many Bluetooth devices are near you, which executables you launch, and how Apple Intelligence profiles your behavior. It does this even when every analytics toggle in System Settings is off.

Two SQLite databases in `/private/var/db/analyticsd/` hold everything. `themacfiles` decodes them into plain language.

## What Apple knows about you

- **What devices are near you** — Bluetooth scans detect up to 94 unique devices in a single sweep, even with Find My disabled. Your AirPods battery level and in-ear status are logged per bud.
- **Where you travel** — WiFi router fingerprints embed the manufacturer's country of origin. Your scans show US and Thailand routers — no GPS required to log international travel.
- **What apps you use and for how long** — exact foreground minutes and switch counts per app. One day of data reveals: developer, privacy-conscious, codes with music.
- **Your daily routine** — location visits classified as home, work, school. Predictions built per day-of-week and hour. Apple learns your weekly pattern.
- **Your social relationships** — ML classifiers named `Family` and `FamilyAndFriends` infer who matters to you. Social sharing behavior tracked per app.
- **What you browse** — 7,394 named entities (people, places, things) extracted from Safari. Topic profiles built from your browsing using 3 separate algorithms.
- **Your aesthetic taste** — photos ranked into "GoldAssets", "ShinyGems", and "RegularGems" by a model that learned what you find beautiful.
- **What you'll do next** — 21 ML models run on-device: predicting your next action, when you'll lock your screen, your routine, your preferences.

*Run `sudo themacfiles summary` to see your own numbers.*

## What's actually running

We extracted and documented the ML models that ship inside macOS private frameworks. The models are files on your SSD — not burned into the chip — and their metadata is fully readable. Full inventory with input features, output schemas, and reversing instructions: **[macos-ml-models.txt](macos-ml-models.txt)**

Highlights:

- **IntelligencePlatform** — `EntityTagging_Family` is an XGBoost tree classifier with 144 input features. It reads your contacts, photos, call patterns, and location history to classify every person in your life as mother, father, sister, brother, partner, coworker, alumni, child, son, daughter, friend, or unknown. Updated every 2 hours. `EntityRelevanceModel` cross-references your geohash, WiFi network, time of day, and day of week to decide who matters to you right now.
- **PeopleSuggester** — `ContactRankerModel` uses 35 features including association-rule mining scores from your interaction patterns. Four separate iCloud family detection model variants (gradient-boosted, decision tree, random forest).
- **PersonalizationPortrait** — context prediction, social highlight scoring, notification filtering.
- **CoreRoutine** — place type classification and visit trajectory analysis. Learns your commute patterns, classifies locations as home/work/gym/etc.
- **MediaAnalysis** — 55+ models for face pose, blink/smile detection, human/pet pose, hand gestures, scene classification, text safety, video highlights, action recognition.
- **VisualLookUp** — 12 models including food recognition, nature/plant identification, address extraction from OCR.
- **SiriInference** — "people-centric" predictors that learn which app you use to reach each person (iMessage vs WhatsApp, Phone vs FaceTime).
- **SensitiveContentAnalysis** — on-device nudity scanner (Communication Safety).
- **SpotlightResources** — per-app search ranking models, query understanding per locale.
- **PromotedContentPrediction** — tap-through-rate prediction for promoted content.
- **CallIntelligence** — predicts whether you'll return a missed call.

165 signal config files feed the Intelligence Platform knowledge graph, tracking: call/message patterns, app launches, WiFi networks, ambient light, charging state, motion state (walking/driving), CarPlay connection, workout activity, sound analysis, and more.

## Quick start

```
cargo install --path crates/themacfiles
sudo themacfiles summary
```

## Commands

**`summary`** — the full picture in one shot. Record counts, app usage with active time, ML models loaded on-device, behavioral predictions, Bluetooth/WiFi surveillance counters, and how many of those records were collected despite opting out.

```
sudo themacfiles summary
```

**`decode`** — every telemetry record, labeled and categorized.

```
sudo themacfiles decode
sudo themacfiles decode --category ai
sudo themacfiles decode --event appUsage --json
sudo themacfiles decode --opt-out-only --limit 50
```

**`events`** — catalog of all event types Apple has registered on your machine, with categories and transform counts.

```
sudo themacfiles events
sudo themacfiles events --category network --json
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
themacfiles summary ~/analyticsd-copy
```

`themacfiles` copies databases to a temp directory before reading, so it never holds locks on the live files.

## Requirements

- macOS (the databases are macOS-specific)
- Rust 1.94+
