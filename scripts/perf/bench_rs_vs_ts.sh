#!/usr/bin/env bash
# Performance comparison: Rust vs TypeScript ifchange
set -euo pipefail

RUST_BIN="$(pwd)/target/release/ifchange"
TS_DIR="../ifttt-lint"
TOTAL_FILES=5000
TMP_DIR=$(mktemp -d)

echo "=== Generating $TOTAL_FILES test files ==="

LANGS=(ts js py bzl java c cpp go rs rb php swift kt scala sh toml tsx yml yaml mts cjs)
HASH_LANGS=(py bzl rb sh toml yml yaml)

is_hash() {
    local ext="$1"
    for h in "${HASH_LANGS[@]}"; do
        [[ "$ext" == "$h" ]] && return 0
    done
    return 1
}

# Generate files
for ((i=0; i<TOTAL_FILES; i++)); do
    ext="${LANGS[$((i % ${#LANGS[@]}))]}"
    if is_hash "$ext"; then
        prefix="#"
    else
        prefix="//"
    fi
    filename="file${i}.${ext}"
    filepath="$TMP_DIR/$filename"

    {
        echo "${prefix} LINT.IfChange"
        for ((j=0; j<100; j++)); do
            echo "$prefix"
        done
        echo "${prefix} LINT.ThenChange(\"${filename}\")"
    } > "$filepath"
done

echo "=== Generating diff ==="

# Generate diff
DIFF_FILE="$TMP_DIR/perf.diff"
> "$DIFF_FILE"
for ((i=0; i<TOTAL_FILES; i++)); do
    ext="${LANGS[$((i % ${#LANGS[@]}))]}"
    if is_hash "$ext"; then
        prefix="#"
    else
        prefix="//"
    fi
    filename="file${i}.${ext}"
    filepath="$TMP_DIR/$filename"

    cat >> "$DIFF_FILE" << EOF
--- a/${filepath}
+++ b/${filepath}
@@ -1,2 +1,2 @@
-${prefix} LINT.IfChange
+${prefix} LINT.IfChange // changed
 ${prefix} LINT.ThenChange("${filename}")
EOF
done

DIFF_SIZE=$(wc -c < "$DIFF_FILE")
echo "Diff size: $((DIFF_SIZE / 1024)) KB"
echo ""

# Verify Rust works
echo "=== Verifying Rust binary ==="
"$RUST_BIN" "$DIFF_FILE" > /dev/null 2>&1
echo "Rust: OK (exit $?)"

# Verify TS works
echo "=== Verifying TypeScript binary ==="
cd "$TS_DIR"
node dist/main.js "$DIFF_FILE" > /dev/null 2>&1
echo "TypeScript: OK (exit $?)"
cd - > /dev/null

echo ""
echo "=== Performance Test: Lint $TOTAL_FILES files ==="
echo ""

# Rust timing (3 runs)
echo "--- Rust (release) ---"
for run in 1 2 3; do
    start=$(date +%s%N)
    "$RUST_BIN" "$DIFF_FILE" > /dev/null 2>&1
    end=$(date +%s%N)
    elapsed=$(( (end - start) / 1000000 ))
    echo "  Run $run: ${elapsed}ms"
done

echo ""

# TypeScript timing (3 runs)
echo "--- TypeScript (Node.js) ---"
for run in 1 2 3; do
    cd "$TS_DIR"
    start=$(date +%s%N)
    node dist/main.js "$DIFF_FILE" > /dev/null 2>&1
    end=$(date +%s%N)
    elapsed=$(( (end - start) / 1000000 ))
    echo "  Run $run: ${elapsed}ms"
    cd - > /dev/null
done

echo ""
echo "=== Performance Test: Check $TOTAL_FILES files ==="
echo ""

# Rust check
echo "--- Rust check ---"
for run in 1 2 3; do
    start=$(date +%s%N)
    "$RUST_BIN" -s "$TMP_DIR" > /dev/null 2>&1
    end=$(date +%s%N)
    elapsed=$(( (end - start) / 1000000 ))
    echo "  Run $run: ${elapsed}ms"
done

echo ""

# TypeScript check
echo "--- TypeScript check ---"
for run in 1 2 3; do
    cd "$TS_DIR"
    start=$(date +%s%N)
    node dist/main.js -s "$TMP_DIR" > /dev/null 2>&1
    end=$(date +%s%N)
    elapsed=$(( (end - start) / 1000000 ))
    echo "  Run $run: ${elapsed}ms"
    cd - > /dev/null
done

# Cleanup
rm -rf "$TMP_DIR"
echo ""
echo "=== Done ==="
