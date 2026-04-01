#!/bin/bash
# dump-siri-remembers.sh — things Siri has stored about you

echo "=== SIRI REMEMBERS ==="
DB="$HOME/Library/IntelligencePlatform/Artifacts/siri/remembers/view.db"
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
echo "=== SIRI RESOLVER INTERACTIONS ==="
DB2="$HOME/Library/IntelligencePlatform/Artifacts/siri/defaultResolverInteractions/view.db"
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
