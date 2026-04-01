#!/bin/bash
# dump-relationships.sh — who Apple thinks your contacts are
# Joins contact names to relationship probability scores

DB_ET="$HOME/Library/IntelligencePlatform/Artifacts/entityTagging/entityTagging.db"
DB_KG="$HOME/Library/IntelligencePlatform/Artifacts/internal/views.db"

sqlite3 -header -column "" "
ATTACH '$DB_KG' AS kg;
ATTACH '$DB_ET' AS et;

SELECT
  coalesce(ea.alias, t.mdid) as name,
  printf('%.3f',t.mother) as mother,
  printf('%.3f',t.father) as father,
  printf('%.3f',t.parent) as parent,
  printf('%.3f',t.sister) as sister,
  printf('%.3f',t.brother) as brother,
  printf('%.3f',t.sibling) as sibling,
  printf('%.3f',t.family) as family,
  printf('%.3f',t.friend) as friend,
  printf('%.3f',t.familyAndFriends) as fam_friend,
  printf('%.3f',t.partner) as partner,
  printf('%.3f',t.coworker) as coworker,
  printf('%.3f',t.alumni) as alumni,
  printf('%.3f',t.child) as child,
  printf('%.3f',t.son) as son,
  printf('%.3f',t.daughter) as daughter,
  printf('%.3f',t.myself) as myself,
  printf('%.3f',t.unknown) as unknown
FROM et.entity_tagging t
LEFT JOIN kg.entity_alias ea ON ea.MD_ID = t.mdid AND ea.signal_type = 'first_provided_alias'
ORDER BY t.family DESC, t.familyAndFriends DESC;
"
