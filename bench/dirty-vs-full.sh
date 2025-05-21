#!/usr/bin/env bash
#
# bench/dirty-vs-full.sh
#
# Compare full-scan vs dirty-scan performance on a large corpus,
# simulating a random set of file modifications before each dirty scan,
# and reporting corpus size, number of dirty files, and speedup.
#

set -euo pipefail
IFS=$'\n\t'

# Path to the marlin binary (adjust if you build elsewhere)
MARLIN_BIN=${MARLIN_BIN:-target/release/marlin}

# Directory containing your test corpus (100k+ files)
CORPUS_DIR=${CORPUS_DIR:-bench/corpus}

# Where to put the ephemeral DB
DB_PATH=${DB_PATH:-bench/index.db}

# How many files to mark dirty before each dirty‐scan run
DIRTY_COUNT=${DIRTY_COUNT:-100}

# Number of warm‐up runs
WARMUPS=${WARMUPS:-3}

# Tell Marlin where to write its DB
export MARLIN_DB_PATH="$DB_PATH"

# Ensure hyperfine is installed
if ! command -v hyperfine &>/dev/null; then
  echo "Error: hyperfine not found. Please install it and try again." >&2
  exit 1
fi

# Ensure our corpus exists
if [ ! -d "$CORPUS_DIR" ]; then
  echo "Error: corpus directory '$CORPUS_DIR' not found." >&2
  exit 1
fi

# Count corpus size
CORPUS_SIZE=$(find "$CORPUS_DIR" -type f | wc -l | tr -d ' ')
echo "→ Corpus size: $CORPUS_SIZE files"
echo "→ Will mark $DIRTY_COUNT files dirty per dirty‐scan run"
echo

# Clean up any old database
rm -f "$DB_PATH"

# First, populate the DB once so that dirty-scan has something to do
echo "→ Initial full scan to populate DB"
"$MARLIN_BIN" scan "$CORPUS_DIR" >/dev/null 2>&1

echo
echo "→ Benchmarking full vs dirty scan with hyperfine"
hyperfine \
  --warmup "$WARMUPS" \
  --prepare "
    # wipe and re-populate
    rm -f '$DB_PATH'
    mkdir -p bench
    export MARLIN_DB_PATH='$DB_PATH'
    $MARLIN_BIN scan '$CORPUS_DIR' >/dev/null 2>&1

    # seed $DIRTY_COUNT random files as 'dirty' in the DB
    sqlite3 '$DB_PATH' \"INSERT OR IGNORE INTO file_changes(file_id, marked_at)
      SELECT id, strftime('%s','now') FROM files
      ORDER BY RANDOM()
      LIMIT $DIRTY_COUNT;\"
  " \
  --command-name "full-scan"  "MARLIN_DB_PATH='$DB_PATH' $MARLIN_BIN scan       '$CORPUS_DIR' >/dev/null 2>&1" \
  --command-name "dirty-scan" "MARLIN_DB_PATH='$DB_PATH' $MARLIN_BIN scan --dirty '$CORPUS_DIR' >/dev/null 2>&1" \
  --export-markdown bench/dirty-vs-full.md

echo
echo "Results written to bench/dirty-vs-full.md"

# Extract the speedup factor from the markdown table:
#   the "Relative" column on the full-scan row tells us how many times
#   slower full-scan is relative to dirty-scan (baseline = 1.00).
SPEEDUP=$(grep '\`full-scan\`' bench/dirty-vs-full.md \
         | awk -F'|' '{print $5}' \
         | xargs || echo "N/A")

echo
echo "→ Summary:"
echo "   Corpus size:        $CORPUS_SIZE files"
echo "   Dirty files seeded: $DIRTY_COUNT"
echo "   Dirty‐scan speedup: dirty-scan ran $SPEEDUP times faster than full-scan"
