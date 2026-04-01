#!/bin/bash
# dump-entity.sh — resolve an Intelligence Platform entity ID to everything Apple knows
# Usage: ./scripts/dump-entity.sh 191042975731975379
#        ./scripts/dump-entity.sh all

DB_KG="$HOME/Library/IntelligencePlatform/Artifacts/internal/views.db"
DB_ET="$HOME/Library/IntelligencePlatform/Artifacts/entityTagging/entityTagging.db"
DB_EI="$HOME/Library/IntelligencePlatform/Artifacts/entityRelevance/entityImportanceSignals.db"
DB_FT="$HOME/Library/IntelligencePlatform/Artifacts/feature/features.db"
PHOTOS="$HOME/Pictures/Photos Library.photoslibrary/database/Photos.sqlite"

if [ -z "$1" ]; then
  echo "Usage: $0 <entity_id | all>"
  echo ""
  echo "Known entities:"
  sqlite3 -header -column "$DB_KG" "
    SELECT DISTINCT
      ea.MD_ID as id,
      group_concat(DISTINCT ea.alias) as aliases,
      group_concat(DISTINCT ea.signal_type) as signal_types
    FROM entity_alias ea
    GROUP BY ea.MD_ID;
  " 2>/dev/null
  echo ""
  echo "All people_subgraph subjects:"
  sqlite3 "$DB_KG" "SELECT DISTINCT subject FROM people_subgraph;" 2>/dev/null
  exit 0
fi

resolve_entity() {
  local ID="$1"

  echo "================================================================"
  echo "ENTITY: $ID"
  echo "================================================================"

  echo ""
  echo "--- Aliases (names Apple uses for this entity) ---"
  sqlite3 -header -column "$DB_KG" "
    SELECT alias, signal_type, confirmation_confidence
    FROM entity_alias WHERE MD_ID = '$ID';
  " 2>/dev/null

  echo ""
  echo "--- People Subgraph (all properties) ---"
  sqlite3 -header -column "$DB_KG" "
    SELECT predicate, object
    FROM people_subgraph WHERE subject = '$ID'
    ORDER BY predicate;
  " 2>/dev/null

  echo ""
  echo "--- Relationship Scores ---"
  sqlite3 -header -column "$DB_ET" "
    SELECT
      printf('%.3f',mother) as mother,
      printf('%.3f',father) as father,
      printf('%.3f',parent) as parent,
      printf('%.3f',sister) as sister,
      printf('%.3f',brother) as brother,
      printf('%.3f',sibling) as sibling,
      printf('%.3f',family) as family,
      printf('%.3f',friend) as friend,
      printf('%.3f',familyAndFriends) as fam_friend,
      printf('%.3f',partner) as partner,
      printf('%.3f',coworker) as coworker,
      printf('%.3f',alumni) as alumni,
      printf('%.3f',child) as child,
      printf('%.3f',son) as son,
      printf('%.3f',daughter) as daughter,
      printf('%.3f',myself) as myself,
      printf('%.3f',unknown) as unknown
    FROM entity_tagging WHERE mdid = '$ID';
  " 2>/dev/null

  echo ""
  echo "--- Entity Importance ---"
  sqlite3 -header -column "$DB_EI" "
    SELECT isFavorite, isEmergencyContact, isICloudFamily, relationshipLabel
    FROM entityImportanceSignalsIDMap WHERE entityIdentifier = '$ID';
  " 2>/dev/null

  echo ""
  echo "--- Photos Person ID Map ---"
  sqlite3 -header -column "$DB_KG" "
    SELECT phPersonIdentifier, id FROM phperson_id_map WHERE id = '$ID';
  " 2>/dev/null

  echo ""
  echo "--- Photos Face Cluster (from Photos.sqlite) ---"
  # Find the phPerson UUID for this entity — strip /L0/070 suffix for Photos.sqlite lookup
  local PH_UUID_RAW=$(sqlite3 "$DB_KG" "
    SELECT phPersonIdentifier FROM phperson_id_map WHERE id = '$ID';
  " 2>/dev/null)
  local PH_UUID=$(echo "$PH_UUID_RAW" | sed 's|/L0/.*||')

  if [ -n "$PH_UUID" ]; then
    sqlite3 -header -column "$PHOTOS" "
      SELECT
        ZPERSONUUID as uuid,
        ZDISPLAYNAME as display_name,
        ZFULLNAME as full_name,
        ZFACECOUNT as face_count,
        CASE ZDETECTIONTYPE WHEN 1 THEN 'human' WHEN 2 THEN 'pet' ELSE ZDETECTIONTYPE END as detection,
        CASE ZGENDERTYPE WHEN 0 THEN 'unknown' WHEN 1 THEN 'male' WHEN 2 THEN 'female' ELSE ZGENDERTYPE END as gender,
        CASE ZAGETYPE WHEN 0 THEN 'unknown' WHEN 1 THEN 'baby' WHEN 2 THEN 'child' WHEN 3 THEN 'teen' WHEN 4 THEN 'adult' WHEN 5 THEN 'senior' ELSE ZAGETYPE END as age_group,
        CASE ZTYPE WHEN 0 THEN 'ordinary' WHEN 1 THEN 'favorite' ELSE ZTYPE END as person_type,
        printf('%.3f', ZISMECONFIDENCE) as is_me_conf,
        printf('%.3f', ZMERGECANDIDATECONFIDENCE) as merge_conf,
        ZMDID as mdid
      FROM ZPERSON
      WHERE ZPERSONUUID = '$PH_UUID';
    " 2>/dev/null

    echo ""
    echo "--- Per-Face Attributes (every detection of this person) ---"
    sqlite3 -header -column "$PHOTOS" "
      SELECT
        a.ZFILENAME as photo,
        datetime(a.ZDATECREATED + 978307200, 'unixepoch', 'localtime') as date,
        CASE f.ZGENDERTYPE WHEN 0 THEN '-' WHEN 1 THEN 'male' WHEN 2 THEN 'female' ELSE f.ZGENDERTYPE END as gender,
        CASE f.ZAGETYPE WHEN 0 THEN '-' WHEN 1 THEN 'baby' WHEN 2 THEN 'child' WHEN 3 THEN 'teen' WHEN 4 THEN 'adult' WHEN 5 THEN 'senior' ELSE f.ZAGETYPE END as age,
        CASE f.ZETHNICITYTYPE WHEN 0 THEN '-' ELSE 'type'||f.ZETHNICITYTYPE END as ethnicity,
        f.ZSKINTONETYPE as skin_tone,
        CASE f.ZHAIRCOLORTYPE WHEN 0 THEN '-' WHEN 1 THEN 'black' WHEN 2 THEN 'brown' WHEN 3 THEN 'blonde' WHEN 4 THEN 'red' WHEN 5 THEN 'gray' ELSE f.ZHAIRCOLORTYPE END as hair_color,
        CASE f.ZFACIALHAIRTYPE WHEN 0 THEN 'none' WHEN 1 THEN 'stubble' WHEN 2 THEN 'beard' WHEN 3 THEN 'moustache' ELSE f.ZFACIALHAIRTYPE END as facial_hair,
        CASE f.ZGLASSESTYPE WHEN 0 THEN 'none' WHEN 1 THEN 'glasses' WHEN 2 THEN 'sunglasses' ELSE f.ZGLASSESTYPE END as glasses,
        CASE f.ZHASSMILE WHEN 0 THEN 'no' WHEN 1 THEN 'yes' ELSE f.ZHASSMILE END as smile,
        CASE f.ZHASFACEMASK WHEN 0 THEN 'no' WHEN 1 THEN 'yes' ELSE f.ZHASFACEMASK END as mask,
        CASE f.ZHEADGEARTYPE WHEN 0 THEN 'none' ELSE 'type'||f.ZHEADGEARTYPE END as headgear,
        CASE f.ZEYEMAKEUPTYPE WHEN 0 THEN 'none' ELSE 'type'||f.ZEYEMAKEUPTYPE END as eye_makeup,
        CASE f.ZLIPMAKEUPTYPE WHEN 0 THEN 'none' ELSE 'type'||f.ZLIPMAKEUPTYPE END as lip_makeup,
        printf('%.3f', f.ZQUALITY) as quality,
        printf('%.4f', a.ZLATITUDE) as lat,
        printf('%.4f', a.ZLONGITUDE) as lon
      FROM ZDETECTEDFACE f
      JOIN ZPERSON p ON f.ZPERSONFORFACE = p.Z_PK
      JOIN ZASSET a ON f.ZASSETFORFACE = a.Z_PK
      WHERE p.ZPERSONUUID = '$PH_UUID'
      ORDER BY a.ZDATECREATED DESC;
    " 2>/dev/null
  else
    echo "(no Photos face cluster linked)"
  fi

  echo ""
  echo "--- Feature Store Entries ---"
  sqlite3 -header -column "$DB_FT" "
    SELECT viewName, featureName, length(value) as value_bytes
    FROM kv WHERE subidentifierName LIKE '%$ID%';
  " 2>/dev/null

  echo ""
  echo "--- Interaction Histograms ---"
  sqlite3 -header -column "$DB_KG" "
    SELECT * FROM personEntityHistograms WHERE entityId = '$ID' LIMIT 10;
  " 2>/dev/null
  sqlite3 -header -column "$DB_KG" "
    SELECT * FROM personInteractionMechanisms WHERE entityId = '$ID' LIMIT 10;
  " 2>/dev/null

  echo ""
  echo "--- Associations ---"
  sqlite3 -header -column "$DB_ET" "
    SELECT subject, predicate, object
    FROM has_association_subgraph
    WHERE subject = '$ID' OR object = '$ID'
    LIMIT 20;
  " 2>/dev/null

  echo ""
}

if [ "$1" = "all" ]; then
  # Resolve every known entity
  for id in $(sqlite3 "$DB_KG" "SELECT DISTINCT subject FROM people_subgraph;" 2>/dev/null); do
    resolve_entity "$id"
  done
else
  resolve_entity "$1"
fi
