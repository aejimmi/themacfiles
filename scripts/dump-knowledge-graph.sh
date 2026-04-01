#!/bin/bash
# dump-knowledge-graph.sh — Apple's knowledge graph about you
# People, entities, aliases, interaction histograms, music events

DB="$HOME/Library/IntelligencePlatform/Artifacts/internal/views.db"

echo "=== YOUR ALIASES (how Apple identifies you) ==="
echo ""
sqlite3 -header -column "$DB" "
SELECT MD_ID, alias, signal_type, confirmation_confidence
FROM entity_alias
ORDER BY MD_ID, signal_type;
"

echo ""
echo "=== PEOPLE SUBGRAPH (relationships and properties) ==="
echo ""
sqlite3 -header -column "$DB" "
SELECT subject, predicate, object
FROM people_subgraph
ORDER BY subject, predicate;
"

echo ""
echo "=== INTERACTION HISTOGRAMS (who you interact with, how often) ==="
echo ""
sqlite3 -header -column "$DB" "
SELECT * FROM personEntityHistogramKeys LIMIT 30;
"
echo ""
sqlite3 -header -column "$DB" "
SELECT * FROM personEntityHistograms LIMIT 30;
"

echo ""
echo "=== INTERACTION MECHANISMS (how you reach people) ==="
echo ""
sqlite3 -header -column "$DB" "
SELECT * FROM personInteractionMechanisms LIMIT 30;
"

echo ""
echo "=== HANDLE MAP (phone numbers, emails, linked to entities) ==="
echo ""
sqlite3 -header -column "$DB" "
SELECT * FROM handle_id LIMIT 30;
"

echo ""
echo "=== LOCATION CONTEXT EVENTS ==="
echo ""
sqlite3 -header -column "$DB" "
SELECT * FROM loiContextEvents LIMIT 20;
"

echo ""
echo "=== WIFI CONTEXT EVENTS ==="
echo ""
sqlite3 -header -column "$DB" "
SELECT * FROM wifiContextEvents LIMIT 20;
"

echo ""
echo "=== PHOTOS: OBSERVED AGES ==="
echo ""
sqlite3 -header -column "$DB" "
SELECT * FROM photosObservedAges LIMIT 20;
"

echo ""
echo "=== TABLE ROW COUNTS ==="
echo ""
for table in $(sqlite3 "$DB" ".tables"); do
  count=$(sqlite3 "$DB" "SELECT count(*) FROM \"$table\";" 2>/dev/null)
  [ "$count" != "0" ] && printf "  %-50s %s\n" "$table" "$count"
done
