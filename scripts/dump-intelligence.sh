#!/bin/bash
# dump-intelligence.sh — dump what Apple's Intelligence Platform knows about you
# Run: ./scripts/dump-intelligence.sh

DB_BASE="$HOME/Library/IntelligencePlatform/Artifacts"

echo "=== ENTITY TAGGING (who Apple thinks your contacts are) ==="
echo ""
sqlite3 -header -column "$DB_BASE/entityTagging/entityTagging.db" \
  "SELECT mdid,
    printf('%.3f',mother) as mother,
    printf('%.3f',father) as father,
    printf('%.3f',parent) as parent,
    printf('%.3f',sister) as sister,
    printf('%.3f',brother) as brother,
    printf('%.3f',sibling) as sibling,
    printf('%.3f',family) as family,
    printf('%.3f',friend) as friend,
    printf('%.3f',familyAndFriends) as famfriend,
    printf('%.3f',partner) as partner,
    printf('%.3f',coworker) as coworker,
    printf('%.3f',alumni) as alumni,
    printf('%.3f',child) as child,
    printf('%.3f',son) as son,
    printf('%.3f',daughter) as daughter,
    printf('%.3f',myself) as myself,
    printf('%.3f',unknown) as unknown
  FROM entity_tagging;" 2>/dev/null || echo "(database not found or empty)"

echo ""
echo "=== ENTITY IMPORTANCE (favorites, emergency contacts, family) ==="
echo ""
sqlite3 -header -column "$DB_BASE/entityRelevance/entityImportanceSignals.db" \
  "SELECT * FROM entityImportanceSignalsIDMap;" 2>/dev/null || echo "(database not found or empty)"

echo ""
echo "=== ACTIVE SIGNALS (what's being tracked) ==="
echo ""
sqlite3 -header -column "$DB_BASE/feature/features.db" \
  "SELECT DISTINCT viewName FROM kv ORDER BY viewName;" 2>/dev/null || echo "(database not found or empty)"

echo ""
echo "=== FEATURE STORE (raw signal data) ==="
echo ""
sqlite3 -header -column "$DB_BASE/feature/features.db" \
  "SELECT viewName, featureName, subidentifierName, length(value) as value_bytes, confidence
   FROM kv ORDER BY viewName, featureName;" 2>/dev/null || echo "(database not found or empty)"

echo ""
echo "=== KNOWLEDGE DB (knowledgeC — behavioral history) ==="
echo ""
KC="$HOME/Library/Application Support/Knowledge/knowledgeC.db"
if [ -f "$KC" ]; then
  echo "Tables:"
  sqlite3 "$KC" ".tables"
  echo ""
  echo "Recent entries (last 10):"
  sqlite3 -header -column "$KC" \
    "SELECT ZOBJECT.ZSTREAMNAME, ZOBJECT.ZVALUESTRING,
      datetime(ZOBJECT.ZCREATIONDATE + 978307200, 'unixepoch', 'localtime') as created,
      datetime(ZOBJECT.ZSTARTDATE + 978307200, 'unixepoch', 'localtime') as start,
      datetime(ZOBJECT.ZENDDATE + 978307200, 'unixepoch', 'localtime') as end
    FROM ZOBJECT ORDER BY ZCREATIONDATE DESC LIMIT 10;" 2>/dev/null
else
  echo "(knowledgeC.db not found — may have been cleaned or moved to Biome)"
fi

echo ""
echo "=== DATABASE SIZES ==="
echo ""
find "$DB_BASE" -name "*.db" -exec ls -lh {} \; 2>/dev/null
ls -lh "$KC" 2>/dev/null
