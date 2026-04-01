#!/bin/bash
# dump-life-events.sh — life events Apple detected + future predictions

echo "=== LIFE EVENTS ==="
DB="$HOME/Library/IntelligencePlatform/Artifacts/lifeEvent/lifeEventView.db"
echo "Tables:"
sqlite3 "$DB" ".tables"
for table in $(sqlite3 "$DB" ".tables"); do
  count=$(sqlite3 "$DB" "SELECT count(*) FROM \"$table\";" 2>/dev/null)
  if [ "$count" != "0" ]; then
    echo ""
    echo "--- $table ($count rows) ---"
    sqlite3 -header -column "$DB" "SELECT * FROM \"$table\" LIMIT 20;"
  fi
done

echo ""
echo "=== FUTURE LIFE EVENTS (predictions) ==="
DB2="$HOME/Library/IntelligencePlatform/Artifacts/futureLifeEvent/futureLifeEventView.db"
echo "Tables:"
sqlite3 "$DB2" ".tables"
for table in $(sqlite3 "$DB2" ".tables"); do
  count=$(sqlite3 "$DB2" "SELECT count(*) FROM \"$table\";" 2>/dev/null)
  if [ "$count" != "0" ]; then
    echo ""
    echo "--- $table ($count rows) ---"
    sqlite3 -header -column "$DB2" "SELECT * FROM \"$table\" LIMIT 20;"
  fi
done
