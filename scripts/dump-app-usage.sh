#!/bin/bash
# dump-app-usage.sh — every app switch Apple recorded, with timestamps
# Shows the last N entries (default 50, pass a number to change)

LIMIT=${1:-50}
KC="$HOME/Library/Application Support/Knowledge/knowledgeC.db"

echo "=== APP USAGE (last $LIMIT entries) ==="
echo ""
sqlite3 -header -column "$KC" "
SELECT
  ZVALUESTRING as app,
  datetime(ZSTARTDATE + 978307200, 'unixepoch', 'localtime') as start,
  datetime(ZENDDATE + 978307200, 'unixepoch', 'localtime') as end,
  CAST((ZENDDATE - ZSTARTDATE) AS INTEGER) as seconds
FROM ZOBJECT
WHERE ZSTREAMNAME = '/app/usage'
ORDER BY ZSTARTDATE DESC
LIMIT $LIMIT;
"

echo ""
echo "=== WEB USAGE (last $LIMIT entries) ==="
echo ""
sqlite3 -header -column "$KC" "
SELECT
  ZVALUESTRING as app,
  datetime(ZSTARTDATE + 978307200, 'unixepoch', 'localtime') as start,
  datetime(ZENDDATE + 978307200, 'unixepoch', 'localtime') as end
FROM ZOBJECT
WHERE ZSTREAMNAME = '/app/webUsage'
ORDER BY ZSTARTDATE DESC
LIMIT $LIMIT;
"

echo ""
echo "=== TOTAL ENTRIES ==="
sqlite3 "$KC" "SELECT ZSTREAMNAME as stream, count(*) as entries FROM ZOBJECT GROUP BY ZSTREAMNAME ORDER BY entries DESC;"
