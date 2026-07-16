#!/usr/bin/env bash
# Batch analyse STL files: score every candidate direction with every
# profile×ranker combination, storing raw metrics and rankings in SQLite.
#
# Uses `orient --all-rankings` for deterministic single-pass evaluation.
#
# Usage:  batch-analyze.sh <stl-folder> <db-path>
#
# Requires:  cargo (Rust), jq, sqlite3
set -euo pipefail

FOLDER="${1:?Usage: batch-analyze.sh <stl-folder> <db-path>}"
DB="${2:?Usage: batch-analyze.sh <stl-folder> <db-path>}"
CRITICAL_ANGLE=45

CLI="$(cd "$(dirname "$0")/.." && pwd)/core/target/release/orient"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ── 1. Build CLI if missing ──
if [ ! -x "$CLI" ]; then
  echo "Building orient CLI..."
  cargo build --features cli --release --manifest-path "$PROJECT_ROOT/core/Cargo.toml"
fi

# ── 2. Profiles (name → weights string) ──
PROFILES=(
  "overhang-only:1,0,0,0,0"
  "footprint-only:0,1,0,0,0"
  "cross-only:0,0,1,0,0"
  "surface-only:0,0,0,1,0"
  "height-only:0,0,0,0,1"
  "overhang-footprint:1,1,0,0,0"
  "equal:1,1,1,1,1"
  "resin-biased:0.5,1,2,0.5,0.5"
)

# ── 3. Create DB schema ──
sqlite3 "$DB" "
CREATE TABLE IF NOT EXISTS stls (
  id INTEGER PRIMARY KEY,
  name TEXT UNIQUE,
  triangle_count INTEGER,
  candidate_count INTEGER,
  critical_angle_deg REAL
);

CREATE TABLE IF NOT EXISTS profiles (
  id INTEGER PRIMARY KEY,
  name TEXT UNIQUE,
  w_overhang REAL,
  w_footprint REAL,
  w_cross REAL,
  w_surface REAL,
  w_height REAL
);

CREATE TABLE IF NOT EXISTS candidates (
  id INTEGER PRIMARY KEY,
  stl_id INTEGER NOT NULL,
  candidate_index INTEGER NOT NULL,
  direction_x REAL, direction_y REAL, direction_z REAL,
  quaternion_0 REAL, quaternion_1 REAL, quaternion_2 REAL, quaternion_3 REAL,
  overhang REAL, footprint REAL, max_cross REAL,
  surface REAL, height REAL, shadowed REAL,
  stable INTEGER, stability_margin REAL, contact_area REAL,
  UNIQUE(stl_id, candidate_index)
);

CREATE TABLE IF NOT EXISTS rankings (
  id INTEGER PRIMARY KEY,
  candidate_id INTEGER NOT NULL,
  profile TEXT NOT NULL,
  ranker TEXT NOT NULL,
  composite_score REAL,
  rank INTEGER,
  UNIQUE(candidate_id, profile, ranker)
);
"

# ── 4. Seed profiles table ──
for entry in "${PROFILES[@]}"; do
  name="${entry%%:*}"
  weights="${entry##*:}"
  IFS=',' read -r wo wf wc ws wh <<< "$weights"
  sqlite3 "$DB" "INSERT OR IGNORE INTO profiles(name,w_overhang,w_footprint,w_cross,w_surface,w_height) VALUES('$name',$wo,$wf,$wc,$ws,$wh);"
done

# ── 5. Process each STL ──
STL_COUNT=0
TOTAL_STLS=0
for _ in "$FOLDER"/*.stl; do [ -f "$_" ] && TOTAL_STLS=$((TOTAL_STLS + 1)); done
echo "Processing $TOTAL_STLS STL(s) into $DB"

for stl in "$FOLDER"/*.stl; do
  [ -f "$stl" ] || continue
  BASENAME=$(basename "$stl" .stl)
  STL_COUNT=$((STL_COUNT + 1))
  echo "  [$STL_COUNT/$TOTAL_STLS] $BASENAME"

  if ! "$CLI" --stl "$stl" --critical-angle "$CRITICAL_ANGLE" --all-rankings 2>/dev/null | \
       jq -r -f "$SCRIPT_DIR/ingest-candidate.jq" --arg stl_name "$BASENAME" | \
       sqlite3 "$DB"; then
    >&2 echo "      WARN: $BASENAME failed — skipping"
  fi
done

echo "Done.  Run queries with:  sqlite3 $DB"
