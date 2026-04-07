#!/bin/bash
# dump-bluetooth-devices.sh — extract every BLE device your Mac has ever seen
# Usage: sudo ./scripts/dump-bluetooth-devices.sh [--wipe]
#
# Sources:
#   /Library/Bluetooth/com.apple.MobileBluetooth.ledevices.other.db   (all discovered devices)
#   /Library/Bluetooth/com.apple.MobileBluetooth.ledevices.paired.db  (paired devices)
#   ~/Library/Preferences/com.apple.Bluetooth.plist                   (last headset MAC)
#   system_profiler SPBluetoothDataType                               (currently visible)

set -euo pipefail

DB_OTHER="/Library/Bluetooth/com.apple.MobileBluetooth.ledevices.other.db"
DB_PAIRED="/Library/Bluetooth/com.apple.MobileBluetooth.ledevices.paired.db"
USER_PLIST="$HOME/Library/Preferences/com.apple.Bluetooth.plist"

RED='\033[0;31m'
GRN='\033[0;32m'
YLW='\033[0;33m'
CYN='\033[0;36m'
DIM='\033[2m'
RST='\033[0m'

# ── Require root for the system Bluetooth databases ──────────────────────────
if [ "$EUID" -ne 0 ] && [ "$(id -u)" -ne 0 ]; then
  echo -e "${RED}This script needs sudo to read the Bluetooth databases.${RST}"
  echo "  sudo $0 $*"
  exit 1
fi

# ── Handle --wipe ────────────────────────────────────────────────────────────
if [ "${1:-}" = "--wipe" ]; then
  count=$(sqlite3 "$DB_OTHER" "SELECT COUNT(*) FROM OtherDevices;" 2>/dev/null)
  echo -e "${RED}WARNING: This will delete $count discovered BLE device records.${RST}"
  echo -e "${DIM}(Your paired devices are in a separate database and won't be touched.)${RST}"
  echo ""
  read -p "Type YES to confirm: " confirm
  if [ "$confirm" = "YES" ]; then
    sqlite3 "$DB_OTHER" "DELETE FROM OtherDevices;"
    sqlite3 "$DB_OTHER" "DELETE FROM CustomProperties;"
    echo -e "${GRN}Wiped $count device records.${RST}"
    echo -e "${DIM}They'll start accumulating again immediately — there's no macOS setting to stop this.${RST}"
  else
    echo "Aborted."
  fi
  exit 0
fi

# ── Calculate real-time epoch offset ─────────────────────────────────────────
# LastSeenTime is mach_continuous_time (seconds, persists across sleep+reboot).
# We derive the epoch by subtracting the max value from "now".
NOW=$(date +%s)
MAX_SEEN=$(sqlite3 "$DB_OTHER" "SELECT MAX(LastSeenTime) FROM OtherDevices;" 2>/dev/null)
if [ -z "$MAX_SEEN" ] || [ "$MAX_SEEN" = "0" ]; then
  EPOCH_OFFSET=0
  echo -e "${YLW}Could not determine timestamp epoch — dates will show raw values.${RST}"
else
  EPOCH_OFFSET=$((NOW - MAX_SEEN))
fi

# ── Summary ──────────────────────────────────────────────────────────────────
TOTAL=$(sqlite3 "$DB_OTHER" "SELECT COUNT(*) FROM OtherDevices;" 2>/dev/null)
NAMED=$(sqlite3 "$DB_OTHER" "SELECT COUNT(*) FROM OtherDevices WHERE Name IS NOT NULL AND Name != '';" 2>/dev/null)
UNNAMED=$((TOTAL - NAMED))
PUBLIC=$(sqlite3 "$DB_OTHER" "SELECT COUNT(*) FROM OtherDevices WHERE Address LIKE 'Public%';" 2>/dev/null)
RANDOM_ADDR=$(sqlite3 "$DB_OTHER" "SELECT COUNT(*) FROM OtherDevices WHERE Address LIKE 'Random%';" 2>/dev/null)
CUSTOM_PROPS=$(sqlite3 "$DB_OTHER" "SELECT COUNT(*) FROM CustomProperties;" 2>/dev/null)

# Earliest and latest real dates
EARLIEST_RAW=$(sqlite3 "$DB_OTHER" "SELECT MIN(LastSeenTime) FROM OtherDevices WHERE LastSeenTime > 0;" 2>/dev/null)
LATEST_RAW=$MAX_SEEN
if [ "$EPOCH_OFFSET" -gt 0 ] 2>/dev/null; then
  EARLIEST_DATE=$(date -r $((EPOCH_OFFSET + EARLIEST_RAW)) "+%Y-%m-%d %H:%M" 2>/dev/null || echo "unknown")
  LATEST_DATE=$(date -r $((EPOCH_OFFSET + LATEST_RAW)) "+%Y-%m-%d %H:%M" 2>/dev/null || echo "unknown")
  SPAN_DAYS=$(( (LATEST_RAW - EARLIEST_RAW) / 86400 ))
else
  EARLIEST_DATE="unknown"
  LATEST_DATE="unknown"
  SPAN_DAYS="?"
fi

echo ""
echo -e "${CYN}════════════════════════════════════════════════════════════════${RST}"
echo -e "${CYN}  BLUETOOTH DEVICE SURVEILLANCE REPORT${RST}"
echo -e "${CYN}════════════════════════════════════════════════════════════════${RST}"
echo ""
echo -e "  Total devices recorded:  ${RED}${TOTAL}${RST}"
echo -e "  Named devices:           ${NAMED}"
echo -e "  Anonymous (no name):     ${UNNAMED}"
echo -e "  Public MAC addresses:    ${PUBLIC}"
echo -e "  Randomized MACs:         ${RANDOM_ADDR}"
echo -e "  Custom property blobs:   ${CUSTOM_PROPS}"
echo ""
echo -e "  Date range:              ${EARLIEST_DATE} → ${LATEST_DATE} (${SPAN_DAYS} days)"
echo -e "  ${DIM}No expiration. No rotation. No opt-out.${RST}"
echo ""

# ── Device categories ────────────────────────────────────────────────────────
echo -e "${CYN}── Device Categories ──────────────────────────────────────────${RST}"
echo ""

# Detect patterns in device names to categorize
echo -e "  ${YLW}TVs & Displays${RST}"
sqlite3 "$DB_OTHER" "SELECT COUNT(*) FROM OtherDevices WHERE Name LIKE '%TV%' OR Name LIKE '%Samsung%' OR Name LIKE '%LG%' OR Name LIKE '%Roku%';" 2>/dev/null | while read c; do echo "    $c devices"; done

echo -e "  ${YLW}Audio (speakers, headphones, earbuds)${RST}"
sqlite3 "$DB_OTHER" "SELECT COUNT(*) FROM OtherDevices WHERE Name LIKE '%Buds%' OR Name LIKE '%JBL%' OR Name LIKE '%ACTON%' OR Name LIKE '%Beats%' OR Name LIKE '%earphone%' OR Name LIKE '%AirPods%' OR Name LIKE '%speaker%' OR Name LIKE '%Flip%' OR Name LIKE '%UGREEN%';" 2>/dev/null | while read c; do echo "    $c devices"; done

echo -e "  ${YLW}Wearables (watches, rings, trackers)${RST}"
sqlite3 "$DB_OTHER" "SELECT COUNT(*) FROM OtherDevices WHERE Name LIKE '%Watch%' OR Name LIKE '%oura%' OR Name LIKE '%WHOOP%' OR Name LIKE '%Smartwatch%' OR Name LIKE '%Fitbit%' OR Name LIKE '%Garmin%';" 2>/dev/null | while read c; do echo "    $c devices"; done

echo -e "  ${YLW}Vehicles${RST}"
sqlite3 "$DB_OTHER" "SELECT COUNT(*) FROM OtherDevices WHERE Name LIKE '%BYD%' OR Name LIKE '%Tesla%' OR Name LIKE '%70mai%' OR Name LIKE '%dashcam%' OR Name LIKE '%Car%';" 2>/dev/null | while read c; do echo "    $c devices"; done

echo -e "  ${YLW}Cameras${RST}"
sqlite3 "$DB_OTHER" "SELECT COUNT(*) FROM OtherDevices WHERE Name LIKE '%GoPro%' OR Name LIKE '%EOS%' OR Name LIKE '%Canon%' OR Name LIKE '%Sony%' OR Name LIKE '%camera%';" 2>/dev/null | while read c; do echo "    $c devices"; done

echo -e "  ${YLW}Smart Home${RST}"
sqlite3 "$DB_OTHER" "SELECT COUNT(*) FROM OtherDevices WHERE Name LIKE '%Fridge%' OR Name LIKE '%Sleepytroll%' OR Name LIKE '%ELK-BLEDOM%' OR Name LIKE '%Atmosphere%' OR Name LIKE '%Smart%' OR Name LIKE '%XPED%' OR Name LIKE '%XPWL%';" 2>/dev/null | while read c; do echo "    $c devices"; done

echo ""

# ── Devices with people's names (privacy leak) ──────────────────────────────
echo -e "${CYN}── Devices That Leak Owner Names ──────────────────────────────${RST}"
echo ""
sqlite3 -separator '|' "$DB_OTHER" "
  SELECT Name, Address, LastSeenTime FROM OtherDevices
  WHERE Name LIKE '%''s %'
     OR Name LIKE 'LE-%'
  ORDER BY LastSeenTime DESC;
" 2>/dev/null | while IFS='|' read -r name addr seen; do
  if [ "$EPOCH_OFFSET" -gt 0 ] 2>/dev/null; then
    real_date=$(date -r $((EPOCH_OFFSET + seen)) "+%Y-%m-%d %H:%M" 2>/dev/null || echo "?")
  else
    real_date="?"
  fi
  mac=$(echo "$addr" | sed 's/^Public //' | sed 's/^Random //')
  printf "  %-35s %-20s %s\n" "$name" "$mac" "$real_date"
done

echo ""

# ── Hotel / location fingerprints ────────────────────────────────────────────
echo -e "${CYN}── Location Fingerprints (hotels, rooms, venues) ─────────────${RST}"
echo ""
sqlite3 -separator '|' "$DB_OTHER" "
  SELECT Name, Address, LastSeenTime FROM OtherDevices
  WHERE Name LIKE '%Room%'
     OR Name LIKE '%Hotel%'
     OR Name LIKE '%Lobby%'
     OR Name LIKE '%Airport%'
  ORDER BY LastSeenTime DESC;
" 2>/dev/null | while IFS='|' read -r name addr seen; do
  if [ "$EPOCH_OFFSET" -gt 0 ] 2>/dev/null; then
    real_date=$(date -r $((EPOCH_OFFSET + seen)) "+%Y-%m-%d %H:%M" 2>/dev/null || echo "?")
  else
    real_date="?"
  fi
  mac=$(echo "$addr" | sed 's/^Public //' | sed 's/^Random //')
  printf "  %-40s %-20s %s\n" "$name" "$mac" "$real_date"
done

echo ""

# ── Most recent 25 devices ──────────────────────────────────────────────────
echo -e "${CYN}── 25 Most Recently Seen Devices ─────────────────────────────${RST}"
echo ""
printf "  ${DIM}%-35s %-12s %-20s %s${RST}\n" "NAME" "TYPE" "MAC ADDRESS" "LAST SEEN"
printf "  ${DIM}%-35s %-12s %-20s %s${RST}\n" "---" "----" "-----------" "---------"

sqlite3 -separator '|' "$DB_OTHER" "
  SELECT
    COALESCE(NULLIF(Name,''), '(anonymous)'),
    CASE WHEN Address LIKE 'Public%' THEN 'Public' ELSE 'Random' END,
    REPLACE(REPLACE(Address, 'Public ', ''), 'Random ', ''),
    LastSeenTime
  FROM OtherDevices
  ORDER BY LastSeenTime DESC
  LIMIT 25;
" 2>/dev/null | while IFS='|' read -r name addrtype mac seen; do
  if [ "$EPOCH_OFFSET" -gt 0 ] 2>/dev/null; then
    real_date=$(date -r $((EPOCH_OFFSET + seen)) "+%Y-%m-%d %H:%M" 2>/dev/null || echo "?")
  else
    real_date="?"
  fi
  printf "  %-35s %-12s %-20s %s\n" "$name" "$addrtype" "$mac" "$real_date"
done

echo ""

# ── Oldest 25 devices (how far back does this go?) ──────────────────────────
echo -e "${CYN}── 25 Oldest Devices (how far back?) ─────────────────────────${RST}"
echo ""
printf "  ${DIM}%-35s %-12s %-20s %s${RST}\n" "NAME" "TYPE" "MAC ADDRESS" "LAST SEEN"
printf "  ${DIM}%-35s %-12s %-20s %s${RST}\n" "---" "----" "-----------" "---------"

sqlite3 -separator '|' "$DB_OTHER" "
  SELECT
    COALESCE(NULLIF(Name,''), '(anonymous)'),
    CASE WHEN Address LIKE 'Public%' THEN 'Public' ELSE 'Random' END,
    REPLACE(REPLACE(Address, 'Public ', ''), 'Random ', ''),
    LastSeenTime
  FROM OtherDevices
  WHERE LastSeenTime > 0
  ORDER BY LastSeenTime ASC
  LIMIT 25;
" 2>/dev/null | while IFS='|' read -r name addrtype mac seen; do
  if [ "$EPOCH_OFFSET" -gt 0 ] 2>/dev/null; then
    real_date=$(date -r $((EPOCH_OFFSET + seen)) "+%Y-%m-%d %H:%M" 2>/dev/null || echo "?")
  else
    real_date="?"
  fi
  printf "  %-35s %-12s %-20s %s\n" "$name" "$addrtype" "$mac" "$real_date"
done

echo ""

# ── CustomProperties (extra metadata per device) ────────────────────────────
echo -e "${CYN}── Custom Properties (extra metadata blobs) ──────────────────${RST}"
echo ""
PROP_COUNT=$(sqlite3 "$DB_OTHER" "SELECT COUNT(*) FROM CustomProperties;" 2>/dev/null)
echo -e "  ${PROP_COUNT} devices have extra JSON metadata stored"
echo ""

if [ "$PROP_COUNT" -gt 0 ]; then
  sqlite3 -separator '|' "$DB_OTHER" "
    SELECT cp.Uuid, od.Name, cp.JSON
    FROM CustomProperties cp
    LEFT JOIN OtherDevices od ON cp.Uuid = od.Uuid
    LIMIT 10;
  " 2>/dev/null | while IFS='|' read -r uuid name json; do
    echo -e "  ${YLW}${name:-unknown}${RST} (${uuid})"
    echo "    $json" | head -c 200
    echo ""
  done
  echo ""
fi

# ── Paired LE devices ───────────────────────────────────────────────────────
echo -e "${CYN}── Paired LE Devices ─────────────────────────────────────────${RST}"
echo ""

# Try to read the paired database — schema may vary
PAIRED_TABLES=$(sqlite3 "$DB_PAIRED" ".tables" 2>/dev/null || echo "")
if [ -n "$PAIRED_TABLES" ]; then
  echo -e "  Tables: ${PAIRED_TABLES}"
  echo ""
  for tbl in $PAIRED_TABLES; do
    count=$(sqlite3 "$DB_PAIRED" "SELECT COUNT(*) FROM \"$tbl\";" 2>/dev/null || echo 0)
    if [ "$count" != "0" ]; then
      echo -e "  ${YLW}${tbl}${RST} (${count} rows):"
      sqlite3 -header -column "$DB_PAIRED" "SELECT * FROM \"$tbl\" LIMIT 10;" 2>/dev/null | sed 's/^/    /'
      echo ""
    fi
  done
else
  echo -e "  ${DIM}Could not read paired device database.${RST}"
fi

# ── Currently visible (no sudo needed) ───────────────────────────────────────
echo -e "${CYN}── Currently Visible Devices (system_profiler) ────────────────${RST}"
echo ""
system_profiler SPBluetoothDataType 2>/dev/null | grep -A4 -E "^\s+(Address|Name|Minor Type|Connected|RSSI)" | sed 's/^/  /'
echo ""

# ── User preferences ────────────────────────────────────────────────────────
echo -e "${CYN}── User Bluetooth Preferences ────────────────────────────────${RST}"
echo ""
if [ -f "$USER_PLIST" ]; then
  last_headset=$(plutil -p "$USER_PLIST" 2>/dev/null | grep -i "headset\|NowPlay" || echo "  (none found)")
  echo "$last_headset" | sed 's/^/  /'
fi

echo ""
echo -e "${CYN}════════════════════════════════════════════════════════════════${RST}"
echo -e "  ${DIM}Database: ${DB_OTHER}${RST}"
echo -e "  ${DIM}To wipe:  sudo $0 --wipe${RST}"
echo -e "${CYN}════════════════════════════════════════════════════════════════${RST}"
echo ""
