mod common;

use common::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn scan_mode_duplicate_labels() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "dup.ts",
            "// LINT.IfChange(\"foo\")\n// LINT.IfChange(\"bar\")\n// LINT.IfChange(\"foo\")\n",
        )],
    );
    let (code, _, _) = run_scan(dir.path(), &[]);
    assert_eq!(code, 1);
}

#[test]
fn scan_mode_unique_labels() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "ok.ts",
            "// LINT.IfChange(\"a\")\n// LINT.IfChange(\"b\")\n",
        )],
    );
    let (code, _, _) = run_scan(dir.path(), &[]);
    assert_eq!(code, 0);
}

#[test]
fn scan_mode_skips_non_lint_files() {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[("plain.ts", "const x = 1;\n")]);
    let (code, _, _) = run_scan(dir.path(), &[]);
    assert_eq!(code, 0);
}

#[cfg(unix)]
#[test]
fn scan_mode_unreadable_file_is_skipped() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("secret.ts");
    fs::write(&path, "// LINT.IfChange\n").unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&path, perms).unwrap();

    let output = Command::new(binary_path())
        .args(["-s", &dir.path().to_string_lossy()])
        .output()
        .unwrap();

    // Restore permissions for temp dir cleanup.
    let mut restore = fs::metadata(&path).unwrap().permissions();
    restore.set_mode(0o644);
    fs::set_permissions(&path, restore).unwrap();

    assert_eq!(output.status.code().unwrap(), 0);
}

#[test]
fn scan_mode_verbose_and_parse_error() {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[("bad.ts", "// LINT.ThenChange(\n")]);
    let (code, _, stderr) = run_scan(dir.path(), &["-d"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("Scanning:"), "stderr: {}", stderr);
    assert!(
        stderr.contains("Malformed LINT.ThenChange"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn scan_mode_skips_binary_files() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("binary.rs"), b"\x00\x01\x02\x03\xff\xfe").unwrap();
    let output = Command::new(binary_path())
        .arg("--scan")
        .arg(dir.path().join("binary.rs").to_string_lossy().to_string())
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    assert!(code == 0 || code == 2, "unexpected exit code: {code}");
}

#[test]
fn scan_mode_lowercase_directives() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "lower.ts",
                "// lint.ifchange(\"lbl\")\nconst v = 1;\n// lint.thenchange(\"other.ts\")\n",
            ),
            ("other.ts", "target\n"),
        ],
    );
    let (code, _, stderr) = run_scan(dir.path(), &["--no-lint", "-v"]);
    assert_eq!(code, 0);
    assert!(
        stderr.contains("1 file") || stderr.contains("1 directive"),
        "scan should detect lowercase directives, stderr: {}",
        stderr
    );
}

#[test]
fn scan_mode_lowercase_duplicate_labels() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "dup_lower.ts",
            "// lint.ifchange(\"foo\")\n// lint.ifchange(\"foo\")\n",
        )],
    );
    let (code, _, _) = run_scan(dir.path(), &["--no-lint"]);
    assert_eq!(
        code, 1,
        "duplicate lowercase labels should be detected as errors"
    );
}

#[test]
fn scan_mode_mixed_case_directives() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "upper.ts",
                "// LINT.IFCHANGE(\"a\")\ncode\n// LINT.THENCHANGE(\"other.ts\")\n",
            ),
            (
                "lower.py",
                "# lint.ifchange(\"b\")\ncode\n# lint.thenchange(\"other.py\")\n",
            ),
            (
                "mixed.rs",
                "// Lint.IfChange(\"c\")\ncode\n// Lint.ThenChange(\"other.rs\")\n",
            ),
            (
                "weird.go",
                "// lInT.iFcHaNgE(\"d\")\ncode\n// LINT.thenchange(\"other.go\")\n",
            ),
            ("other.ts", "target\n"),
            ("other.py", "target\n"),
            ("other.rs", "target\n"),
            ("other.go", "target\n"),
        ],
    );
    let (code, _, stderr) = run_scan(dir.path(), &["--no-lint", "-v"]);
    assert_eq!(code, 0, "stderr: {}", stderr);
    assert!(
        stderr.contains("4 with directives") && stderr.contains("4 directive pairs"),
        "scan should detect all 4 files with mixed-case directives, stderr: {}",
        stderr
    );
}

#[test]
fn scan_mode_mixed_case_label_endlabel() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[("labels.ts", "// lint.label(\"sec\")\ncode\n// LINT.EndLabel\n// Lint.Label(\"other\")\nmore\n// LINT.ENDLABEL\n")],
    );
    let (code, _, stderr) = run_scan(dir.path(), &["--no-lint", "-v"]);
    assert_eq!(
        code, 0,
        "mixed-case labels should parse without error, stderr: {}",
        stderr
    );
}

#[test]
fn scan_mode_mixed_case_duplicate_across_casing() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "case.ts",
            "// lint.ifchange(\"foo\")\n// LINT.IFCHANGE(\"foo\")\n",
        )],
    );
    let (code, _, _) = run_scan(dir.path(), &["--no-lint"]);
    assert_eq!(
        code, 1,
        "duplicate label 'foo' should be detected regardless of directive casing"
    );
}

#[test]
fn scan_mode_mixed_case_pair_within_file() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "mixed_pair.ts",
                "// lint.ifchange\ncode\n// LINT.THENCHANGE(\"other.ts\")\n",
            ),
            ("other.ts", "target\n"),
        ],
    );
    let (code, _, stderr) = run_scan(dir.path(), &["--no-lint", "-v"]);
    assert_eq!(
        code, 0,
        "mixed-case pair should be valid, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("1 pair"),
        "should detect 1 pair, stderr: {}",
        stderr
    );
}

#[test]
fn scan_prefilter_rejects_non_directive_lint_dot() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            // "lint." in a path/identifier should NOT count as "with directives"
            ("eslint.ts", "import eslint from 'eslint.config';\n"),
            ("pylint.py", "# pylint.disable=no-member\n"),
            (
                "build.json",
                "{\"files\": [\"eslint.config.js\", \"pylint.rc\"]}\n",
            ),
            // But a real directive should still be detected
            (
                "real.ts",
                "// LINT.IfChange\ncode\n// LINT.ThenChange(\"other.ts\")\n",
            ),
            ("other.ts", "target\n"),
        ],
    );
    let (code, _, stderr) = run_scan(dir.path(), &["--no-lint", "-v"]);
    assert_eq!(code, 0, "stderr: {}", stderr);
    assert!(
        stderr.contains("1 with directives"),
        "only the real directive file should count, stderr: {}",
        stderr
    );
}

#[test]
fn scan_detects_missing_target_file() {
    // a.py references b.py which doesn't exist. Scan should report an error.
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "a.py",
            "# LINT.IfChange\nvalue = 1\n# LINT.ThenChange(\"b.py\")\n",
        )],
    );
    let (code, _, stderr) = run_scan(dir.path(), &[]);
    assert_eq!(
        code, 1,
        "scan should fail when target file doesn't exist, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("b.py"),
        "error should mention the missing target, stderr: {}",
        stderr
    );
}

#[test]
fn scan_accepts_existing_target_file() {
    // a.py references b.py which exists. Scan should pass.
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "a.py",
                "# LINT.IfChange\nvalue = 1\n# LINT.ThenChange(\"b.py\")\n",
            ),
            ("b.py", "target content\n"),
        ],
    );
    let (code, _, stderr) = run_scan(dir.path(), &[]);
    assert_eq!(
        code, 0,
        "scan should pass when target file exists, stderr: {}",
        stderr
    );
}

#[test]
fn scan_detects_missing_labeled_target_file() {
    // a.py references b.py#section which doesn't exist. Scan should report an error.
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "a.py",
            "# LINT.IfChange\nvalue = 1\n# LINT.ThenChange(\"b.py#section\")\n",
        )],
    );
    let (code, _, stderr) = run_scan(dir.path(), &[]);
    assert_eq!(
        code, 1,
        "scan should fail when labeled target file doesn't exist, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("b.py"),
        "error should mention the missing target, stderr: {}",
        stderr
    );
}

#[test]
fn scan_self_referencing_target_is_ok() {
    // a.py references itself (same file). Should pass since the file exists.
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "a.py",
            "# LINT.IfChange\nvalue = 1\n# LINT.ThenChange(\"a.py\")\n",
        )],
    );
    let (code, _, stderr) = run_scan(dir.path(), &[]);
    assert_eq!(
        code, 0,
        "self-referencing target should pass, stderr: {}",
        stderr
    );
}

#[test]
fn scan_verbose_shows_summary() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "a.ts",
                "// LINT.IfChange\nconst v = 1;\n// LINT.ThenChange(\"b.ts\")\n",
            ),
            ("b.ts", "target\n"),
        ],
    );
    let (code, _, stderr) = run_scan(dir.path(), &["--no-lint", "-v"]);
    assert_eq!(code, 0);
    assert!(
        stderr.contains("Scan summary:"),
        "scan verbose should show summary, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("pair"),
        "scan verbose should mention pairs, stderr: {}",
        stderr
    );
}

// ── Multi-line ThenChange without brackets (scan) ──

#[test]
fn scan_multiline_thenchange_no_brackets_valid() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "src.ts",
                "// LINT.IfChange\nconst v = 1;\n// LINT.ThenChange(\n//   \"a.ts\",\n//   \"b.ts\",\n// )\n",
            ),
            ("a.ts", ""),
            ("b.ts", ""),
        ],
    );
    let (code, _, stderr) = run_scan(dir.path(), &["--no-lint", "-v"]);
    assert_eq!(
        code, 0,
        "valid multi-line no-brackets should pass scan, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("1 directive pair"),
        "should detect 1 pair, stderr: {}",
        stderr
    );
}

#[test]
fn scan_multiline_thenchange_no_brackets_unclosed_error() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "bad.ts",
            "// LINT.ThenChange(\n//   \"a.ts\",\n//   \"b.ts\",\n",
        )],
    );
    let (code, _, stderr) = run_scan(dir.path(), &[]);
    assert_eq!(
        code, 1,
        "unclosed multi-line should fail scan, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("Malformed LINT.ThenChange"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn scan_multiline_thenchange_no_brackets_multiple_pairs() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "multi.ts",
                "// LINT.IfChange\nconst a = 1;\n// LINT.ThenChange(\n//   \"x.ts\",\n//   \"y.ts\",\n// )\n// LINT.IfChange\nconst b = 2;\n// LINT.ThenChange(\n//   \"z.ts\",\n// )\n",
            ),
            ("x.ts", ""),
            ("y.ts", ""),
            ("z.ts", ""),
        ],
    );
    let (code, _, stderr) = run_scan(dir.path(), &["--no-lint", "-v"]);
    assert_eq!(
        code, 0,
        "multiple pairs should pass scan, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("2 directive pairs"),
        "should detect 2 pairs, stderr: {}",
        stderr
    );
}

// ── Directory target scan tests ──

#[test]
fn scan_directory_target_exists() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "a.py",
                "# LINT.IfChange\nvalue = 1\n# LINT.ThenChange(\"subdir/\")\n",
            ),
            ("subdir/file.py", "content\n"),
        ],
    );
    let (code, _, stderr) = run_scan(dir.path(), &["--no-lint"]);
    assert_eq!(
        code, 0,
        "scan should pass when directory target exists, stderr: {}",
        stderr
    );
}

#[test]
fn scan_directory_target_does_not_exist() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "a.py",
            "# LINT.IfChange\nvalue = 1\n# LINT.ThenChange(\"missing_dir/\")\n",
        )],
    );
    let (code, _, stderr) = run_scan(dir.path(), &["--no-lint"]);
    assert_eq!(
        code, 1,
        "scan should fail when directory target doesn't exist, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("missing_dir/"),
        "error should mention the missing target, stderr: {}",
        stderr
    );
}

#[test]
fn scan_bare_directory_without_trailing_slash() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "a.py",
                "# LINT.IfChange\nvalue = 1\n# LINT.ThenChange(\"subdir\")\n",
            ),
            ("subdir/file.py", "content\n"),
        ],
    );
    let (code, _, stderr) = run_scan(dir.path(), &["--no-lint"]);
    assert_eq!(
        code, 1,
        "scan should fail when target is a directory without trailing slash, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("is a directory") && stderr.contains("trailing '/'"),
        "error should suggest adding trailing slash, stderr: {}",
        stderr
    );
}

#[test]
fn scan_directory_target_with_label_rejected() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "a.py",
                "# LINT.IfChange\nvalue = 1\n# LINT.ThenChange(\"subdir/#label\")\n",
            ),
            ("subdir/file.py", "content\n"),
        ],
    );
    let (code, _, stderr) = run_scan(dir.path(), &["--no-lint"]);
    assert_eq!(
        code, 1,
        "scan should fail when directory target has a label, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("labels are not supported for directory targets"),
        "error should mention labels not supported for dir targets, stderr: {}",
        stderr
    );
}
