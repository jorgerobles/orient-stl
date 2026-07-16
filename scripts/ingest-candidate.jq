# jq filter: transform orient CLI JSON output into SQL INSERTs for sqlite3
# --all-rankings mode: rankings come from the flat .rankings[] array
# Usage: jq -f ingest-candidate.jq --arg stl_name NAME

# STL metadata
"INSERT OR IGNORE INTO stls(name, triangle_count, candidate_count, critical_angle_deg) VALUES(" +
  ($stl_name | @sh) + "," +
  (.meta.triangle_count | tostring) + "," +
  (.meta.candidate_count | tostring) + "," +
  (.meta.critical_angle_deg | tostring) +
  ");",

# Candidate raw metrics
(.candidates[] |
  "INSERT OR IGNORE INTO candidates(stl_id,candidate_index,direction_x,direction_y,direction_z,quaternion_0,quaternion_1,quaternion_2,quaternion_3,overhang,footprint,max_cross,surface,height,shadowed,stable,stability_margin,contact_area) VALUES(" +
    "(SELECT id FROM stls WHERE name=" + ($stl_name | @sh) + ")," +
    (.index | tostring) + "," +
    (.direction[0] | tostring) + "," + (.direction[1] | tostring) + "," + (.direction[2] | tostring) + "," +
    (.quaternion[0] | tostring) + "," + (.quaternion[1] | tostring) + "," + (.quaternion[2] | tostring) + "," + (.quaternion[3] | tostring) + "," +
    (.overhang | tostring) + "," + (.footprint | tostring) + "," + (.max_cross | tostring) + "," +
    (.surface | tostring) + "," + (.height | tostring) + "," + (.shadowed | tostring) + "," +
    (if .stable then "1" else "0" end) + "," +
    (.stability_margin | tostring) + "," + (.contact_area | tostring) +
  ");"),

# Rankings from flat array
(.rankings[] |
  "INSERT INTO rankings(candidate_id,profile,ranker,composite_score,rank) VALUES(" +
    "(SELECT c.id FROM candidates c JOIN stls s ON c.stl_id=s.id WHERE s.name=" + ($stl_name | @sh) + " AND c.candidate_index=" + (.candidate | tostring) + ")," +
    (.profile | @sh) + "," + (.ranker | @sh) + "," + (.composite_score | tostring) + "," + (.rank | tostring) +
  ");")
