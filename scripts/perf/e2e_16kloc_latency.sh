#!/usr/bin/env bash
set -euo pipefail

TMP_DIR=$(mktemp -d)
FILE="$TMP_DIR/big.ts"
DIFF="$TMP_DIR/big.diff"
TARGET_DIFF_LOC=16000
REPEATS=10

# Diff line count is: 5 + 2 * replacements. Use ceil division to meet/exceed target.
replacements=$(( (TARGET_DIFF_LOC - 5 + 1) / 2 ))

{
  echo "// LINT.IfChange"
  for ((i=0; i<replacements; i++)); do
    echo "const value_${i} = $((i+1));"
  done
  echo "// LINT.ThenChange(\"big.ts\")"
} > "$FILE"

{
  echo "--- a/$FILE"
  echo "+++ b/$FILE"
  echo "@@ -1,$((replacements+2)) +1,$((replacements+2)) @@"
  echo " // LINT.IfChange"
  for ((i=0; i<replacements; i++)); do
    echo "-const value_${i} = ${i};"
    echo "+const value_${i} = $((i+1));"
  done
  echo " // LINT.ThenChange(\"big.ts\")"
} > "$DIFF"

BIN="target/release/ifchange"
cargo build --release -q
"$BIN" "$DIFF" >/dev/null 2>&1

echo "diff_lines=$(wc -l < "$DIFF")"
for run in $(seq 1 "$REPEATS"); do
  start=$(date +%s%N)
  "$BIN" "$DIFF" >/dev/null 2>&1
  end=$(date +%s%N)
  echo "run_${run}_ms=$(( (end - start) / 1000000 ))"
done

rm -rf "$TMP_DIR"
