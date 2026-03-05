use criterion::{criterion_group, criterion_main, Criterion};
use ifchange::lint_diff;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

const LANGS: &[&str] = &[
    "ts", "js", "py", "bzl", "java", "c", "cpp", "go", "rs", "rb", "php", "swift", "kt", "scala",
    "sh", "toml", "tsx", "yml", "yaml", "mts", "cjs",
];
const HASH_LANGS: &[&str] = &["py", "bzl", "rb", "sh", "toml", "yml", "yaml"];

fn is_hash(ext: &str) -> bool {
    HASH_LANGS.contains(&ext)
}

fn generate_perf_files(total: usize) -> (TempDir, Vec<String>, String) {
    let dir = TempDir::new().unwrap();
    let mut files = Vec::new();
    let mut diff_lines = Vec::new();

    for i in 0..total {
        let ext = LANGS[i % LANGS.len()];
        let prefix = if is_hash(ext) { "#" } else { "//" };
        let filename = format!("file{}.{}", i, ext);
        let filepath = dir.path().join(&filename);
        let full = filepath.to_string_lossy().to_string();

        let mut content = Vec::new();
        content.push(format!("{} LINT.IfChange", prefix));
        for _ in 0..100 {
            content.push(prefix.to_string());
        }
        let base = Path::new(&filename)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        content.push(format!("{} LINT.ThenChange(\"{}\")", prefix, base));

        fs::write(&filepath, content.join("\n")).unwrap();
        files.push(full.clone());

        diff_lines.push(format!("--- a/{}", full));
        diff_lines.push(format!("+++ b/{}", full));
        diff_lines.push("@@ -1,2 +1,2 @@".to_string());
        diff_lines.push(format!("-{} LINT.IfChange", prefix));
        diff_lines.push(format!("+{} LINT.IfChange // changed", prefix));
        diff_lines.push(format!(" {} LINT.ThenChange(\"{}\")", prefix, base));
    }

    let diff = diff_lines.join("\n");
    (dir, files, diff)
}

fn bench_lint_5000(c: &mut Criterion) {
    let (_dir, _files, diff) = generate_perf_files(5000);
    let ignore: Vec<String> = Vec::new();

    c.bench_function("lint_5000_files", |b| {
        b.iter(|| {
            let result = lint_diff(&diff, false, false, &ignore);
            assert_eq!(result.exit_code, 0);
        });
    });
}

fn bench_lint_1000(c: &mut Criterion) {
    let (_dir, _files, diff) = generate_perf_files(1000);
    let ignore: Vec<String> = Vec::new();

    c.bench_function("lint_1000_files", |b| {
        b.iter(|| {
            let result = lint_diff(&diff, false, false, &ignore);
            assert_eq!(result.exit_code, 0);
        });
    });
}

criterion_group!(benches, bench_lint_1000, bench_lint_5000);
criterion_main!(benches);
