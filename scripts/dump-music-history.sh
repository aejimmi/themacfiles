#!/bin/bash
# dump-music-history.sh — Apple Music listening history from Intelligence Platform

DB="$HOME/Library/IntelligencePlatform/Artifacts/appleMusicEvent/appleMusicEventView.db"

echo "=== TABLES ==="
sqlite3 "$DB" ".tables"

echo ""
echo "=== SCHEMA ==="
sqlite3 "$DB" ".schema" | head -30

echo ""
echo "=== DATA (first 30 rows from each table) ==="
for table in $(sqlite3 "$DB" ".tables"); do
  count=$(sqlite3 "$DB" "SELECT count(*) FROM \"$table\";" 2>/dev/null)
  if [ "$count" != "0" ]; then
    echo ""
    echo "--- $table ($count rows) ---"
    sqlite3 -header -column "$DB" "SELECT * FROM \"$table\" LIMIT 30;"
  fi
done
