use criterion::{criterion_group, criterion_main, Criterion};
use ifchange::{parse_directives_from_content, validate_directive_uniqueness};
use ignore::WalkBuilder;
use std::fs;
use tempfile::TempDir;

const LANGS: &[&str] = &[
    "ts", "js", "py", "bzl", "java", "c", "cpp", "go", "rs", "rb", "php", "swift", "kt", "scala",
    "sh", "toml", "tsx", "yml", "yaml", "mts", "cjs",
];
const HASH_LANGS: &[&str] = &["py", "bzl", "rb", "sh", "toml", "yml", "yaml"];

fn is_hash(ext: &str) -> bool {
    HASH_LANGS.contains(&ext)
}

fn generate_scan_files(total: usize) -> TempDir {
    let dir = TempDir::new().unwrap();
    for i in 0..total {
        let ext = LANGS[i % LANGS.len()];
        let prefix = if is_hash(ext) { "#" } else { "//" };
        let filename = format!("file{}.{}", i, ext);
        let filepath = dir.path().join(&filename);

        let mut content = Vec::new();
        content.push(format!("{} LINT.IfChange", prefix));
        for _ in 0..100 {
            content.push(prefix.to_string());
        }
        content.push(format!("{} LINT.ThenChange(\"{}\")", prefix, filename));
        fs::write(&filepath, content.join("\n")).unwrap();
    }
    dir
}

fn bench_scan_5000(c: &mut Criterion) {
    let dir = generate_scan_files(5000);
    let root = dir.path().to_path_buf();

    c.bench_function("scan_5000_files", |b| {
        b.iter(|| {
            let mut errors = Vec::new();
            for entry in WalkBuilder::new(&root)
                .build()
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_type().is_some_and(|ft| ft.is_file()))
            {
                let path = entry.path();
                let file_path = path.to_string_lossy().to_string();
                let content = match fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                if !content.contains("LINT.") {
                    continue;
                }
                match parse_directives_from_content(&content, &file_path) {
                    Ok(directives) => {
                        errors.extend(validate_directive_uniqueness(&directives, &file_path));
                    }
                    Err(e) => errors.push(e.to_string()),
                }
            }
            assert!(errors.is_empty());
        });
    });
}

criterion_group!(benches, bench_scan_5000);
criterion_main!(benches);
