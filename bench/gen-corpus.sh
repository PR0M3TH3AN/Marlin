#!/usr/bin/env bash
#
# bench/gen-corpus.sh
#
# Generate a synthetic corpus of N files in nested directories.
# Defaults to 1 000 files so it stays laptop-friendly.
#

set -euo pipefail
IFS=$'\n\t'

# How many files? (default: 1 000)
COUNT=${COUNT:-100000}
# Where to put them
TARGET=${TARGET:-bench/corpus}

# Wipe any old corpus
rm -rf "$TARGET"
mkdir -p "$TARGET"

echo "🚀 Generating $COUNT files under $TARGET…"
for i in $(seq 1 "$COUNT"); do
  # bucket into 100 sub-dirs so walkdir has some structure
  dir_index=$(( (i - 1) / (COUNT / 100 + 1) ))
  subdir="$TARGET/dir$(printf "%03d" "$dir_index")"
  mkdir -p "$subdir"
  echo "This is file #$i" > "$subdir/file_$i.txt"
done

echo "✅ Done: $(find "$TARGET" -type f | wc -l) files created."
