default:
    @just --list

build:
    cargo build --release

check:
    @cargo fmt --all -- --check
    @cargo clippy --quiet --all-targets --all-features -- -D warnings

check_audit:
    cargo fmt --all -- --check
    cargo clippy --quiet --all-targets --all-features -- -D warnings
    cargo machete
    cargo +nightly udeps --all-targets --all-features

check_fix:
    @cargo fmt --all
    @cargo clippy --quiet --all-targets --all-features -- -D warnings

clean:
    cargo clean

perf:
    @echo "── lint: 5000 files, 21 lang types ──"
    @cargo bench --bench lint_bench 2>&1 | grep -E "^[a-z_]|time:"
    @echo ""
    @echo "── check: 5000 files, directive validation ──"
    @cargo bench --bench scan_bench 2>&1 | grep -E "^[a-z_]|time:"
    @echo ""
    @echo "── parser: single 16k-line diff ──"
    @cargo bench --bench latency_16kloc_bench 2>&1 | grep -E "^[a-z_]|time:"

perf_history:
    @gh api 'repos/slnc/ifchange/contents/bench/data.js?ref=gh-pages' --jq '.content' \
        | base64 -d \
        | sed 's/^window.BENCHMARK_DATA = //' \
        | jq -r '.entries.Benchmark[-100:] | reverse | .[] | (.commit.timestamp | split("T")[0]) as $d | (.commit.id[:7]) as $s | (.benches | map({(.name): (.value / 1e6 | . * 100 | round / 100 | tostring + " ms")}) | add) as $b | [$d, $s, $b["lint_latency_16kloc_diff"] // "-", $b["lint_1000_files"] // "-", $b["lint_5000_files"] // "-", $b["scan_5000_files"] // "-"] | @tsv' \
        | (echo "DATE\tCOMMIT\tLINT_16K\tLINT_1K\tLINT_5K\tSCAN_5K" && cat) \
        | column -t -s '	'

setup:
    ./scripts/setup.sh

test:
    #!/usr/bin/env bash
    set -eu
    out="$(mktemp)"
    trap 'rm -f "$out"' EXIT
    if cargo test --quiet >"$out" 2>&1; then
        awk 'BEGIN { passed=0; failed=0; ignored=0; measured=0; filtered=0; suites=0 }
            /test result:/ {
                suites++;
                for (i = 1; i <= NF; i++) {
                    if ($i == "passed;") passed += $(i - 1);
                    else if ($i == "failed;") failed += $(i - 1);
                    else if ($i == "ignored;") ignored += $(i - 1);
                    else if ($i == "measured;") measured += $(i - 1);
                    else if ($i == "filtered") filtered += $(i - 1);
                }
            }
            END {
                if (suites == 0) {
                    print "Test totals: 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out";
                } else {
                    printf "Test totals: %d passed; %d failed; %d ignored; %d measured; %d filtered out\n", passed, failed, ignored, measured, filtered;
                }
            }' "$out"
    else
        cat "$out"
        exit 1
    fi

test_coverage:
    cargo llvm-cov --workspace --all-features --html
    cargo llvm-cov report --json --summary-only --output-path target/llvm-cov/summary.json
    @jq -r '.data[0].totals as $t | "Coverage: lines \($t.lines.covered)/\($t.lines.count) (\(($t.lines.percent*100|round)/100)%) | regions \($t.regions.covered)/\($t.regions.count) (\(($t.regions.percent*100|round)/100)%) | functions \($t.functions.covered)/\($t.functions.count) (\(($t.functions.percent*100|round)/100)%)"' target/llvm-cov/summary.json
