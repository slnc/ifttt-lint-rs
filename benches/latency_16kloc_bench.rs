use criterion::{criterion_group, criterion_main, Criterion};
use ifchange::lint_diff;
use std::fs;
use tempfile::TempDir;

const TARGET_DIFF_LOC: usize = 16_000;

fn generate_16kloc_diff_case() -> (TempDir, String) {
    let dir = TempDir::new().expect("failed to create temp dir");
    let file_path = dir.path().join("big.ts");
    let file_name = "big.ts";

    // Keep the generated diff near 16k lines:
    // 4 metadata lines + 2 context lines + (2 * replacements) ~= 16_000
    // Diff line count is: 5 + 2 * replacements. Use ceil division so we hit/exceed target.
    let replacements = TARGET_DIFF_LOC.saturating_sub(5).div_ceil(2);

    let mut new_content = String::new();
    new_content.push_str("// LINT.IfChange\n");
    for i in 0..replacements {
        new_content.push_str(&format!("const value_{} = {};\n", i, i + 1));
    }
    new_content.push_str(&format!("// LINT.ThenChange(\"{}\")\n", file_name));
    fs::write(&file_path, new_content).expect("failed to write source file");

    let mut diff = String::new();
    diff.push_str(&format!("--- a/{}\n", file_path.to_string_lossy()));
    diff.push_str(&format!("+++ b/{}\n", file_path.to_string_lossy()));
    diff.push_str(&format!(
        "@@ -1,{} +1,{} @@\n",
        replacements + 2,
        replacements + 2
    ));
    diff.push_str(" // LINT.IfChange\n");
    for i in 0..replacements {
        diff.push_str(&format!("-const value_{} = {};\n", i, i));
        diff.push_str(&format!("+const value_{} = {};\n", i, i + 1));
    }
    diff.push_str(&format!(" // LINT.ThenChange(\"{}\")\n", file_name));

    assert!(
        diff.lines().count() >= TARGET_DIFF_LOC,
        "generated diff is smaller than expected"
    );

    (dir, diff)
}

fn bench_lint_latency_16kloc(c: &mut Criterion) {
    let (dir, diff) = generate_16kloc_diff_case();
    let ignore: Vec<String> = Vec::new();
    let root = dir.path().to_path_buf();

    c.bench_function("lint_latency_16kloc_diff", |b| {
        b.iter(|| {
            let result = lint_diff(&diff, false, false, &ignore, &root);
            assert_eq!(result.exit_code, 0);
        });
    });
}

criterion_group!(benches, bench_lint_latency_16kloc);
criterion_main!(benches);
