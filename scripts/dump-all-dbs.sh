#!/bin/bash
# dump-all-dbs.sh — inventory every Intelligence Platform database with row counts

DB_BASE="$HOME/Library/IntelligencePlatform/Artifacts"

echo "=== ALL INTELLIGENCE PLATFORM DATABASES ==="
echo ""
find "$DB_BASE" -name "*.db" ! -name "*fullRebuild*" | sort | while read db; do
  relpath="${db#$DB_BASE/}"
  size=$(ls -lh "$db" | awk '{print $5}')
  echo "=== $relpath ($size) ==="
  for table in $(sqlite3 "$db" ".tables" 2>/dev/null); do
    count=$(sqlite3 "$db" "SELECT count(*) FROM \"$table\";" 2>/dev/null)
    [ "$count" != "0" ] && printf "  %-50s %s rows\n" "$table" "$count"
  done
  echo ""
done

echo "=== KNOWLEDGEC ==="
KC="$HOME/Library/Application Support/Knowledge/knowledgeC.db"
size=$(ls -lh "$KC" 2>/dev/null | awk '{print $5}')
echo "knowledgeC.db ($size)"
sqlite3 "$KC" "SELECT ZSTREAMNAME, count(*) as rows FROM ZOBJECT GROUP BY ZSTREAMNAME ORDER BY rows DESC;" 2>/dev/null
