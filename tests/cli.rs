mod common;

use common::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn warn_mode() {
    let (code, _, stderr) = lint_case(
        &[
            ("file1.ts", "// LINT.IfChange\nconst v = 1;\n// LINT.ThenChange(\"file2.ts\")\n"),
            ("file2.ts", "const v = 1;\n"),
        ],
        &[("file1.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const v = 1;\n+const v = 2;\n // LINT.ThenChange(\"file2.ts\")")],
        &["-w"],
    );
    assert_eq!(code, 0, "warn mode should exit 0, stderr: {}", stderr);
}

#[test]
fn ignore_glob() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "file.json",
                "// LINT.IfChange\n// LINT.ThenChange(\"nochange.ts\")\n",
            ),
            ("nochange.ts", "// dummy\n"),
        ],
    );
    let diff = make_diff(dir.path(), &[
        ("file.json", "@@ -1,2 +1,2 @@\n-// LINT.IfChange\n+// LINT.IfChange // changed\n // LINT.ThenChange(\"nochange.ts\")"),
    ]);
    // Without ignore: should error
    let (code1, _, _) = run_lint(&diff, &[]);
    assert_eq!(code1, 1);
    // With ignore: should pass
    let (code2, _, _) = run_lint(&diff, &["-i", "*.json"]);
    assert_eq!(code2, 0);
}

#[test]
fn ignore_orphan_thenchange_by_target() {
    let (code, _, _) = lint_case(
        &[("a.ts", "// LINT.ThenChange(\"foo.ts\")\n")],
        &[("a.ts", "@@ -1 +1 @@\n-// LINT.ThenChange(\"foo.ts\")\n+// LINT.ThenChange(\"foo.ts\") // changed")],
        &["-i", "foo.ts"],
    );
    assert_eq!(code, 0);
}

#[test]
fn ignore_orphan_ifchange_by_label() {
    let (code, _, _) = lint_case(
        &[("a.ts", "// LINT.IfChange(\"cfg\")\n")],
        &[(
            "a.ts",
            "@@ -1 +1 @@\n-// LINT.IfChange(\"cfg\")\n+// LINT.IfChange(\"cfg\") // changed",
        )],
        &["-i", "a.ts#cfg"],
    );
    assert_eq!(code, 0);
}

#[test]
fn phase2_parse_error_ignored_by_target_ignore() {
    let (code, _, _) = lint_case(
        &[
            (
                "a.ts",
                "// LINT.IfChange\nx=1\n// LINT.ThenChange(\"b.ts\")\n",
            ),
            ("b.ts", "// LINT.IfChange(\n"),
        ],
        &[(
            "a.ts",
            "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-x=1\n+x=2\n // LINT.ThenChange(\"b.ts\")",
        )],
        &["-i", "b.ts"],
    );
    assert_eq!(code, 0);
}

#[test]
fn phase2_parse_error_ignored_by_if_label_ignore() {
    let (code, _, _) = lint_case(
        &[
            ("a.ts", "// LINT.IfChange(\"cfg\")\nx=1\n// LINT.ThenChange(\"b.ts\")\n"),
            ("b.ts", "// LINT.IfChange(\n"),
        ],
        &[("a.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange(\"cfg\")\n-x=1\n+x=2\n // LINT.ThenChange(\"b.ts\")")],
        &["-i", "a.ts#cfg"],
    );
    assert_eq!(code, 0);
}

#[test]
fn verbose_output() {
    let (code, _, stderr) = run_lint("", &["-v"]);
    assert_eq!(code, 0);
    assert!(
        stderr.contains("scan:"),
        "verbose should show scan summary, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("lint:"),
        "verbose should show lint summary, stderr: {}",
        stderr
    );
}

#[test]
fn verbose_shows_directive_pairs_and_summary() {
    let (code, _, stderr) = lint_case(
        &[
            ("a.ts", "// LINT.IfChange\nconst v = 1;\n// LINT.ThenChange(\"b.ts\")\n"),
            ("b.ts", "const v = 1;\n"),
        ],
        &[
            ("a.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const v = 1;\n+const v = 2;\n // LINT.ThenChange(\"b.ts\")"),
            ("b.ts", "@@ -1 +1 @@\n-const v = 1;\n+const v = 2;"),
        ],
        &["-v"],
    );
    assert_eq!(code, 0);
    assert!(
        stderr.contains("lint:"),
        "verbose should show lint header, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("1 pair checked"),
        "verbose should show pair count, stderr: {}",
        stderr
    );
}

#[test]
fn debug_implies_verbose() {
    let (code, _, stderr) = lint_case(
        &[
            ("a.ts", "// LINT.IfChange\nconst v = 1;\n// LINT.ThenChange(\"b.ts\")\n"),
            ("b.ts", "const v = 1;\n"),
        ],
        &[
            ("a.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const v = 1;\n+const v = 2;\n // LINT.ThenChange(\"b.ts\")"),
            ("b.ts", "@@ -1 +1 @@\n-const v = 1;\n+const v = 2;"),
        ],
        &["--debug"],
    );
    assert_eq!(code, 0);
    assert!(
        stderr.contains("lint: 1 pair checked"),
        "debug should include verbose summary, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("Parallelism:"),
        "debug should show parallelism, stderr: {}",
        stderr
    );
}

#[test]
fn debug_changed_file_progress() {
    let (code, _, stderr) = lint_case(
        &[
            (
                "a.ts",
                "// LINT.IfChange\nx=1\n// LINT.ThenChange(\"b.ts\")\n",
            ),
            ("b.ts", "x=1\n"),
        ],
        &[(
            "a.ts",
            "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-x=1\n+x=2\n // LINT.ThenChange(\"b.ts\")",
        )],
        &["--debug"],
    );
    assert_eq!(code, 1);
    assert!(
        stderr.contains("Processing changed file:"),
        "stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("Finished processing changed file:"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn debug_ignored_orphans_log_messages() {
    let (code, _, stderr) = lint_case(
        &[
            ("orphan_then.ts", "// LINT.ThenChange(\"foo.ts\")\n"),
            ("orphan_if.ts", "// LINT.IfChange(\"cfg\")\n"),
        ],
        &[
            ("orphan_then.ts", "@@ -1 +1 @@\n-// LINT.ThenChange(\"foo.ts\")\n+// LINT.ThenChange(\"foo.ts\") // changed"),
            ("orphan_if.ts", "@@ -1 +1 @@\n-// LINT.IfChange(\"cfg\")\n+// LINT.IfChange(\"cfg\") // changed"),
        ],
        &["--debug", "-i", "foo.ts", "-i", "orphan_if.ts#cfg"],
    );
    assert_eq!(code, 0);
    assert!(
        stderr.contains("Ignoring orphan ThenChange"),
        "stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("Ignoring orphan IfChange"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn debug_jobs_uses_explicit_value() {
    let (code, _, stderr) = run_lint("", &["--debug", "-j", "2"]);
    assert_eq!(code, 0);
    assert!(stderr.contains("Parallelism: 2"), "stderr: {}", stderr);
}

#[test]
fn jobs_flag_path() {
    let (code, _, _) = run_lint("", &["-j", "2"]);
    assert_eq!(code, 0);
}

#[test]
fn invalid_diff_input_exits_2() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), "this is not a diff").unwrap();
    let output = Command::new(binary_path())
        .arg(tmp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap(), 2);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid diff input"), "stderr: {}", stderr);
}

#[test]
fn missing_diff_file_exits_2() {
    let output = Command::new(binary_path())
        .arg("/definitely/missing/diff.patch")
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap(), 2);
}

#[test]
fn stdin_diff_mode() {
    let diff = "--- a/f.txt\n+++ b/f.txt\n@@ -1 +1 @@\n-a\n+b\n";
    let (code, _, _) = run_lint_stdin(diff, &["-"]);
    assert_eq!(code, 0);
}

#[cfg(unix)]
#[test]
fn stdin_read_error_exits_2() {
    use std::fs::File;
    use std::process::Stdio;

    let dir = TempDir::new().unwrap();
    let dir_file = File::open(dir.path()).unwrap();
    let output = Command::new(binary_path())
        .stdin(Stdio::from(dir_file))
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap(), 2);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Error:"), "stderr: {}", stderr);
    assert!(stderr.contains("reading stdin"), "stderr: {}", stderr);
}

#[test]
fn no_color_suppresses_ansi_codes() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), "this is not a diff").unwrap();
    let output = Command::new(binary_path())
        .arg(tmp.path())
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap(), 2);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains('\x1b'),
        "NO_COLOR should suppress ANSI codes, stderr: {}",
        stderr
    );
}

#[test]
fn no_scan_and_no_lint_errors() {
    let output = Command::new(binary_path())
        .args(["--no-scan", "--no-lint"])
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap(), 2);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--no-scan and --no-lint cannot both be set"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn no_scan_skips_scan_phase() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "dup.ts",
            "// LINT.IfChange(\"foo\")\n// LINT.IfChange(\"foo\")\n",
        )],
    );
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), "").unwrap();
    let output = Command::new(binary_path())
        .args(["--no-scan", &tmp.path().to_string_lossy()])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code().unwrap(),
        0,
        "should pass because scan is skipped"
    );
}

#[test]
fn default_runs_scan_on_cwd() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "dup.ts",
            "// LINT.IfChange(\"foo\")\n// LINT.IfChange(\"foo\")\n",
        )],
    );
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), "").unwrap();
    let output = Command::new(binary_path())
        .arg(tmp.path())
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code().unwrap(),
        1,
        "should fail because scan detects duplicate labels"
    );
}

#[test]
fn no_lint_with_scan_dir() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "ok.ts",
            "// LINT.IfChange(\"a\")\n// LINT.ThenChange(\"b.ts\")\n",
        )],
    );
    let output = Command::new(binary_path())
        .args(["--no-lint", "-s", &dir.path().to_string_lossy()])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code().unwrap(),
        0,
        "scan-only mode should pass with valid directives"
    );
}

#[test]
fn binary_diff_file_does_not_crash() {
    use std::process::Stdio;

    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), b"\x00\x01\x02\x03\xff\xfe\xfd\n\x80\x90\xa0\n").unwrap();
    let output = Command::new(binary_path())
        .arg(tmp.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    assert!(code == 0 || code == 2, "unexpected exit code: {code}");
}

#[test]
fn binary_stdin_does_not_crash() {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new(binary_path())
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"\x00\x01\x02\xff\xfe\xfd\n\x80\x90\xa0\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let code = output.status.code().unwrap_or(-1);
    assert!(code == 0 || code == 2, "unexpected exit code: {code}");
}

#[test]
fn binary_data_in_diff_hunks_does_not_crash() {
    let diff = "--- a/f.bin\n+++ b/f.bin\n@@ -1 +1 @@\n-\x00\x01\x02\n+\x03\x04\x05\n";
    let (code, _, _) = run_lint(diff, &[]);
    assert_eq!(code, 0);
}
