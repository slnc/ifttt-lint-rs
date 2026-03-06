use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn binary_path() -> String {
    let path = env!("CARGO_BIN_EXE_lint-ifchange");
    path.to_string()
}

fn write_files(dir: &Path, files: &[(&str, &str)]) {
    for (name, content) in files {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
    }
}

fn make_diff(dir: &Path, changes: &[(&str, &str)]) -> String {
    let mut diff_lines = Vec::new();
    for (file, hunk) in changes {
        let full_path = dir.join(file);
        let full = full_path.to_string_lossy().replace('\\', "/");
        diff_lines.push(format!("--- a/{}", full));
        diff_lines.push(format!("+++ b/{}", full));
        diff_lines.push(hunk.to_string());
    }
    diff_lines.join("\n")
}

fn run_lint(diff: &str) -> (i32, String, String) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), diff).unwrap();
    let output = Command::new(binary_path())
        .arg(tmp.path())
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

fn run_lint_with_args(diff: &str, args: &[&str]) -> (i32, String, String) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), diff).unwrap();
    let output = Command::new(binary_path())
        .args(args)
        .arg(tmp.path())
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

fn run_lint_stdin(input: &str, args: &[&str]) -> (i32, String, String) {
    let mut child = Command::new(binary_path())
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

#[test]
fn empty_diff() {
    let (code, _stdout, _stderr) = run_lint("");
    assert_eq!(code, 0);
}

#[test]
fn no_error_when_target_changed() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "file1.ts",
                "// LINT.IfChange\nconst v = 1;\n// LINT.ThenChange(\"file2.ts\")\n",
            ),
            ("file2.ts", "const v = 1;\n"),
        ],
    );
    let diff = make_diff(dir.path(), &[
        ("file1.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const v = 1;\n+const v = 2;\n // LINT.ThenChange(\"file2.ts\")"),
        ("file2.ts", "@@ -1 +1 @@\n-const v = 1;\n+const v = 2;"),
    ]);
    let (code, stdout, _) = run_lint(&diff);
    assert_eq!(code, 0, "stdout: {}", stdout);
}

#[test]
fn error_when_target_not_changed() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "file1.ts",
                "// LINT.IfChange\nconst v = 1;\n// LINT.ThenChange(\"file2.ts\")\n",
            ),
            ("file2.ts", "const v = 1;\n"),
        ],
    );
    let diff = make_diff(dir.path(), &[
        ("file1.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const v = 1;\n+const v = 2;\n // LINT.ThenChange(\"file2.ts\")"),
    ]);
    let (code, stdout, _) = run_lint(&diff);
    assert_eq!(code, 1);
    assert!(stdout.contains("not changed"), "stdout: {}", stdout);
}

#[test]
fn labeled_change_ok() {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[
        ("file1.ts", "// LINT.IfChange\n// LINT.ThenChange(\"file2.ts#label1\")\n"),
        ("file2.ts", "// header\n// LINT.Label(\"label1\")\nconsole.log(1);\n// LINT.EndLabel\n// footer\n"),
    ]);
    let diff = make_diff(dir.path(), &[
        ("file1.ts", "@@ -1,2 +1,2 @@\n-// LINT.IfChange\n+// LINT.IfChange // changed\n // LINT.ThenChange(\"file2.ts#label1\")"),
        ("file2.ts", "@@ -1,5 +1,5 @@\n // header\n // LINT.Label(\"label1\")\n-console.log(1);\n+console.log(2);\n // LINT.EndLabel\n // footer"),
    ]);
    let (code, stdout, _) = run_lint(&diff);
    assert_eq!(code, 0, "stdout: {}", stdout);
}

#[test]
fn labeled_change_missing() {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[
        ("file1.ts", "// LINT.IfChange\n// LINT.ThenChange(\"file2.ts#label1\")\n"),
        ("file2.ts", "// header\n// LINT.Label(\"label1\")\nconsole.log(1);\n// LINT.EndLabel\n// footer\n"),
    ]);
    let diff = make_diff(dir.path(), &[
        ("file1.ts", "@@ -1,2 +1,2 @@\n-// LINT.IfChange\n+// LINT.IfChange // changed\n // LINT.ThenChange(\"file2.ts#label1\")"),
        ("file2.ts", "@@ -1,5 +1,5 @@\n // header\n // LINT.Label(\"label1\")\n console.log(1);\n-// LINT.EndLabel\n+// LINT.EndLabel // changed\n // footer"),
    ]);
    let (code, stdout, _) = run_lint(&diff);
    assert_eq!(code, 1);
    assert!(stdout.contains("expected changes in"), "stdout: {}", stdout);
}

#[test]
fn orphan_then_change() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[("file1.ts", "// LINT.ThenChange(\"foo.ts\")\n")],
    );
    let diff = make_diff(dir.path(), &[
        ("file1.ts", "@@ -1 +1 @@\n-// LINT.ThenChange(\"foo.ts\")\n+// LINT.ThenChange(\"foo.ts\") // changed"),
    ]);
    let (code, stdout, _) = run_lint(&diff);
    assert_eq!(code, 1);
    assert!(
        stdout.contains("unexpected ThenChange"),
        "stdout: {}",
        stdout
    );
}

#[test]
fn orphan_if_change() {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[("file1.ts", "// LINT.IfChange\n")]);
    let diff = make_diff(
        dir.path(),
        &[(
            "file1.ts",
            "@@ -1 +1 @@\n-// LINT.IfChange\n+// LINT.IfChange // changed",
        )],
    );
    let (code, stdout, _) = run_lint(&diff);
    assert_eq!(code, 1);
    assert!(stdout.contains("missing ThenChange"), "stdout: {}", stdout);
}

#[test]
fn warn_mode() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "file1.ts",
                "// LINT.IfChange\nconst v = 1;\n// LINT.ThenChange(\"file2.ts\")\n",
            ),
            ("file2.ts", "const v = 1;\n"),
        ],
    );
    let diff = make_diff(dir.path(), &[
        ("file1.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const v = 1;\n+const v = 2;\n // LINT.ThenChange(\"file2.ts\")"),
    ]);
    let (code, stdout, _) = run_lint_with_args(&diff, &["-w"]);
    assert_eq!(code, 0, "warn mode should exit 0, stdout: {}", stdout);
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
    let (code1, _, _) = run_lint(&diff);
    assert_eq!(code1, 1);
    // With ignore: should pass
    let (code2, _, _) = run_lint_with_args(&diff, &["-i", "*.json"]);
    assert_eq!(code2, 0);
}

#[test]
fn cross_reference_ignores_outside_changes() {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[
        ("source.py", "# Header\ndef helper():\n    return 1\n# LINT.IfChange\nclass Status:\n    ACTIVE = 1\n# LINT.ThenChange(\"target.py\")\ndef other():\n    return 2\n"),
        ("target.py", "# LINT.IfChange\nSTATUS = [1]\n# LINT.ThenChange(\"source.py\")\ndef target_helper():\n    return 1\n"),
    ]);
    // Change OUTSIDE the IfChange block in source
    let diff = make_diff(dir.path(), &[
        ("source.py", "@@ -1,9 +1,9 @@\n # Header\n-def helper():\n+def helper_modified():\n     return 1\n # LINT.IfChange\n class Status:\n     ACTIVE = 1\n # LINT.ThenChange(\"target.py\")\n-def other():\n+def other_modified():\n     return 2"),
    ]);
    let (code, stdout, _) = run_lint(&diff);
    assert_eq!(
        code, 0,
        "changes outside IfChange should not trigger, stdout: {}",
        stdout
    );
}

#[test]
fn cross_reference_detects_inside_changes() {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[
        ("source.py", "# Header\n# LINT.IfChange\nclass Status:\n    ACTIVE = 1\n    INACTIVE = 2\n# LINT.ThenChange(\"target.py\")\ndef other():\n    return 2\n"),
        ("target.py", "# LINT.IfChange\nSTATUS = [1, 2]\n# LINT.ThenChange(\"source.py\")\n"),
    ]);
    // Change INSIDE the IfChange block
    let diff = make_diff(dir.path(), &[
        ("source.py", "@@ -2,5 +2,5 @@\n # LINT.IfChange\n class Status:\n     ACTIVE = 1\n-    INACTIVE = 2\n+    PENDING = 3\n # LINT.ThenChange(\"target.py\")"),
    ]);
    let (code, stdout, _) = run_lint(&diff);
    assert_eq!(
        code, 1,
        "changes inside IfChange should trigger, stdout: {}",
        stdout
    );
}

#[test]
fn self_reference_with_label() {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[
        ("file1.ts", "// LINT.Label(\"label1\")\nconsole.log(1);\n// LINT.EndLabel\n// LINT.IfChange\n// LINT.ThenChange(\"#label1\")\n"),
    ]);
    let diff = make_diff(
        dir.path(),
        &[(
            "file1.ts",
            "@@ -4,4 +4,4 @@\n-// LINT.IfChange\n+// LINT.IfChange // changed",
        )],
    );
    let (code, stdout, _) = run_lint(&diff);
    assert_eq!(
        code, 1,
        "self-reference should require label changes, stdout: {}",
        stdout
    );
}

#[test]
fn python_hash_comments() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "config.py",
                "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(\"constants.py\")\n",
            ),
            ("constants.py", "VALUE = 1\n"),
        ],
    );
    let diff = make_diff(dir.path(), &[
        ("config.py", "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(\"constants.py\")"),
    ]);
    let (code, _, _) = run_lint(&diff);
    assert_eq!(code, 1); // constants.py not changed
}

#[test]
fn check_mode_duplicate_labels() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "dup.ts",
            "// LINT.IfChange(\"foo\")\n// LINT.IfChange(\"bar\")\n// LINT.IfChange(\"foo\")\n",
        )],
    );
    let output = Command::new(binary_path())
        .args(["-c", &dir.path().to_string_lossy()])
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap(), 1);
}

#[test]
fn check_mode_unique_labels() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "ok.ts",
            "// LINT.IfChange(\"a\")\n// LINT.IfChange(\"b\")\n",
        )],
    );
    let output = Command::new(binary_path())
        .args(["-c", &dir.path().to_string_lossy()])
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap(), 0);
}

#[test]
fn verbose_output() {
    let (code, _, stderr) = run_lint_with_args("", &["-v"]);
    assert_eq!(code, 0);
    assert!(stderr.contains("Parallelism:"), "stderr: {}", stderr);
}

#[test]
fn deleted_files_ignored() {
    let diff = "--- a/deleted.ts\n+++ /dev/null\n@@ -1,3 +0,0 @@\n-// LINT.IfChange\n-value = 1\n-// LINT.ThenChange(\"other.ts\")\n";
    let (code, _, _) = run_lint(diff);
    assert_eq!(code, 0);
}

#[test]
fn ifchange_label_in_error_context() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "file1.ts",
                "// LINT.IfChange('g')\n// LINT.ThenChange(\"file2.ts\")\n",
            ),
            ("file2.ts", "// dummy\n"),
        ],
    );
    let diff = make_diff(dir.path(), &[
        ("file1.ts", "@@ -1,2 +1,2 @@\n-// LINT.IfChange('g')\n+// LINT.IfChange('g') // changed\n // LINT.ThenChange(\"file2.ts\")"),
    ]);
    let (code, stdout, _) = run_lint(&diff);
    assert_eq!(code, 1);
    assert!(
        stdout.contains("#g:"),
        "error should include label context, stdout: {}",
        stdout
    );
}

#[test]
fn no_change_outside_block() {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[
        ("file1.ts", "const other = 0;\n// LINT.IfChange\nconst v = 1;\n// LINT.ThenChange(\"file2.ts\")\nconst more = 2;\n"),
        ("file2.ts", "const v = 1;\n"),
    ]);
    // Only change lines outside the IfChange block
    let diff = make_diff(dir.path(), &[
        ("file1.ts", "@@ -1,5 +1,5 @@\n-const other = 0;\n+const other = 99;\n // LINT.IfChange\n const v = 1;\n // LINT.ThenChange(\"file2.ts\")\n const more = 2;"),
    ]);
    let (code, stdout, _) = run_lint(&diff);
    assert_eq!(
        code, 0,
        "changes outside block should not trigger, stdout: {}",
        stdout
    );
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
    let (code, _stdout, _stderr) = run_lint_stdin(diff, &["-"]);
    assert_eq!(code, 0);
}

#[test]
fn parallelism_flag_path() {
    let (code, _stdout, _stderr) = run_lint_with_args("", &["-p", "2"]);
    assert_eq!(code, 0);
}

#[test]
fn verbose_parallelism_uses_explicit_value() {
    let (code, _stdout, stderr) = run_lint_with_args("", &["-v", "-p", "2"]);
    assert_eq!(code, 0);
    assert!(stderr.contains("Parallelism: 2"), "stderr: {}", stderr);
}

#[test]
fn check_mode_verbose_and_parse_error() {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[("bad.ts", "// LINT.ThenChange(\n")]);
    let output = Command::new(binary_path())
        .args(["-c", &dir.path().to_string_lossy(), "-v"])
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap(), 1);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Validating file:"), "stderr: {}", stderr);
    assert!(
        stderr.contains("Malformed LINT.ThenChange"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn check_mode_skips_non_lint_files() {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[("plain.ts", "const x = 1;\n")]);
    let output = Command::new(binary_path())
        .args(["-c", &dir.path().to_string_lossy()])
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap(), 0);
}

#[cfg(unix)]
#[test]
fn check_mode_unreadable_file_is_skipped() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("secret.ts");
    fs::write(&path, "// LINT.IfChange\n").unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&path, perms).unwrap();

    let output = Command::new(binary_path())
        .args(["-c", &dir.path().to_string_lossy()])
        .output()
        .unwrap();

    // Restore permissions for temp dir cleanup.
    let mut restore = fs::metadata(&path).unwrap().permissions();
    restore.set_mode(0o644);
    fs::set_permissions(&path, restore).unwrap();

    assert_eq!(output.status.code().unwrap(), 0);
}

#[test]
fn changed_file_parse_error_reported() {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[("bad.ts", "// LINT.IfChange(\n")]);
    let diff = make_diff(
        dir.path(),
        &[(
            "bad.ts",
            "@@ -1 +1 @@\n-// LINT.IfChange(\n+// LINT.IfChange(",
        )],
    );
    let (code, stdout, _stderr) = run_lint(&diff);
    assert_eq!(code, 1);
    assert!(
        stdout.contains("Malformed LINT.IfChange"),
        "stdout: {}",
        stdout
    );
}

#[test]
fn duplicate_labels_in_changed_file_reported() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "dup.ts",
            "// LINT.IfChange(\"x\")\n// LINT.Label(\"x\")\n// LINT.ThenChange(\"other.ts\")\n",
        )],
    );
    let diff = make_diff(dir.path(), &[(
        "dup.ts",
        "@@ -1,3 +1,3 @@\n // LINT.IfChange(\"x\")\n // LINT.Label(\"x\")\n-// LINT.ThenChange(\"other.ts\")\n+// LINT.ThenChange(\"other.ts\") // changed",
    )]);
    let (code, stdout, _stderr) = run_lint(&diff);
    assert_eq!(code, 1);
    assert!(
        stdout.contains("duplicate directive label"),
        "stdout: {}",
        stdout
    );
}

#[test]
fn malformed_target_file_reports_not_found() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "a.ts",
                "// LINT.IfChange\nconst A = 1;\n// LINT.ThenChange(\"b.ts\")\n",
            ),
            ("b.ts", "// LINT.IfChange(\n"),
        ],
    );
    let diff = make_diff(dir.path(), &[(
        "a.ts",
        "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const A = 1;\n+const A = 2;\n // LINT.ThenChange(\"b.ts\")",
    )]);
    let (code, stdout, _stderr) = run_lint(&diff);
    assert_eq!(code, 1);
    assert!(stdout.contains("not found"), "stdout: {}", stdout);
}

#[test]
fn missing_target_file_reports_not_found() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "a.ts",
            "// LINT.IfChange\nconst A = 1;\n// LINT.ThenChange(\"missing.ts\")\n",
        )],
    );
    let diff = make_diff(dir.path(), &[(
        "a.ts",
        "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const A = 1;\n+const A = 2;\n // LINT.ThenChange(\"missing.ts\")",
    )]);
    let (code, stdout, _stderr) = run_lint(&diff);
    assert_eq!(code, 1);
    assert!(stdout.contains("not found"), "stdout: {}", stdout);
}

#[test]
fn missing_target_label_reports_available_labels() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "a.ts",
                "// LINT.IfChange\nconst A = 1;\n// LINT.ThenChange(\"b.ts#missing\")\n",
            ),
            (
                "b.ts",
                "// LINT.Label(\"present\")\nlet x = 1;\n// LINT.EndLabel\n",
            ),
        ],
    );
    let diff = make_diff(dir.path(), &[
        ("a.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const A = 1;\n+const A = 2;\n // LINT.ThenChange(\"b.ts#missing\")"),
        ("b.ts", "@@ -1,3 +1,3 @@\n // LINT.Label(\"present\")\n-let x = 1;\n+let x = 2;\n // LINT.EndLabel"),
    ]);
    let (code, stdout, _stderr) = run_lint(&diff);
    assert_eq!(code, 1);
    assert!(
        stdout.contains("label 'missing' not found"),
        "stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("Available labels: present"),
        "stdout: {}",
        stdout
    );
}

#[test]
fn ignore_orphan_thenchange_by_target() {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[("a.ts", "// LINT.ThenChange(\"foo.ts\")\n")]);
    let diff = make_diff(dir.path(), &[("a.ts", "@@ -1 +1 @@\n-// LINT.ThenChange(\"foo.ts\")\n+// LINT.ThenChange(\"foo.ts\") // changed")]);
    let (code, _stdout, _stderr) = run_lint_with_args(&diff, &["-i", "foo.ts"]);
    assert_eq!(code, 0);
}

#[test]
fn ignore_orphan_ifchange_by_label() {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[("a.ts", "// LINT.IfChange(\"cfg\")\n")]);
    let diff = make_diff(
        dir.path(),
        &[(
            "a.ts",
            "@@ -1 +1 @@\n-// LINT.IfChange(\"cfg\")\n+// LINT.IfChange(\"cfg\") // changed",
        )],
    );
    let (code, _stdout, _stderr) = run_lint_with_args(&diff, &["-i", "a.ts#cfg"]);
    assert_eq!(code, 0);
}

#[test]
fn multiple_ifchange_marks_first_orphan() {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[(
        "a.ts",
        "// LINT.IfChange(\"first\")\n// LINT.IfChange(\"second\")\nconst x = 1;\n// LINT.ThenChange(\"b.ts\")\n",
    )]);
    let diff = make_diff(dir.path(), &[(
        "a.ts",
        "@@ -1,4 +1,4 @@\n // LINT.IfChange(\"first\")\n // LINT.IfChange(\"second\")\n-const x = 1;\n+const x = 2;\n // LINT.ThenChange(\"b.ts\")",
    )]);
    let (code, stdout, _stderr) = run_lint(&diff);
    assert_eq!(code, 1);
    assert!(
        stdout.contains("missing ThenChange after IfChange('first')"),
        "stdout: {}",
        stdout
    );
}

#[test]
fn verbose_changed_file_progress() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "a.ts",
                "// LINT.IfChange\nx=1\n// LINT.ThenChange(\"b.ts\")\n",
            ),
            ("b.ts", "x=1\n"),
        ],
    );
    let diff = make_diff(
        dir.path(),
        &[(
            "a.ts",
            "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-x=1\n+x=2\n // LINT.ThenChange(\"b.ts\")",
        )],
    );
    let (code, _stdout, stderr) = run_lint_with_args(&diff, &["-v"]);
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
fn verbose_ignored_orphans_log_messages() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            ("orphan_then.ts", "// LINT.ThenChange(\"foo.ts\")\n"),
            ("orphan_if.ts", "// LINT.IfChange(\"cfg\")\n"),
        ],
    );
    let diff = make_diff(dir.path(), &[
        ("orphan_then.ts", "@@ -1 +1 @@\n-// LINT.ThenChange(\"foo.ts\")\n+// LINT.ThenChange(\"foo.ts\") // changed"),
        ("orphan_if.ts", "@@ -1 +1 @@\n-// LINT.IfChange(\"cfg\")\n+// LINT.IfChange(\"cfg\") // changed"),
    ]);
    let (code, _stdout, stderr) =
        run_lint_with_args(&diff, &["-v", "-i", "foo.ts", "-i", "orphan_if.ts#cfg"]);
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
fn phase2_duplicate_labels_in_target_file_reported() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "a.ts",
                "// LINT.IfChange\nx=1\n// LINT.ThenChange(\"b.ts\")\n",
            ),
            (
                "b.ts",
                "// LINT.Label(\"dup\")\nlet x = 1;\n// LINT.Label(\"dup\")\n",
            ),
        ],
    );
    let diff = make_diff(
        dir.path(),
        &[(
            "a.ts",
            "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-x=1\n+x=2\n // LINT.ThenChange(\"b.ts\")",
        )],
    );
    let (code, stdout, _stderr) = run_lint(&diff);
    assert_eq!(code, 1);
    assert!(
        stdout.contains("duplicate directive label 'dup'"),
        "stdout: {}",
        stdout
    );
}

#[test]
fn phase2_parse_error_ignored_by_target_ignore() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "a.ts",
                "// LINT.IfChange\nx=1\n// LINT.ThenChange(\"b.ts\")\n",
            ),
            ("b.ts", "// LINT.IfChange(\n"),
        ],
    );
    let diff = make_diff(
        dir.path(),
        &[(
            "a.ts",
            "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-x=1\n+x=2\n // LINT.ThenChange(\"b.ts\")",
        )],
    );
    let (code, _stdout, _stderr) = run_lint_with_args(&diff, &["-i", "b.ts"]);
    assert_eq!(code, 0);
}

#[test]
fn phase2_parse_error_ignored_by_if_label_ignore() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "a.ts",
                "// LINT.IfChange(\"cfg\")\nx=1\n// LINT.ThenChange(\"b.ts\")\n",
            ),
            ("b.ts", "// LINT.IfChange(\n"),
        ],
    );
    let diff = make_diff(dir.path(), &[(
        "a.ts",
        "@@ -1,3 +1,3 @@\n // LINT.IfChange(\"cfg\")\n-x=1\n+x=2\n // LINT.ThenChange(\"b.ts\")",
    )]);
    let (code, _stdout, _stderr) = run_lint_with_args(&diff, &["-i", "a.ts#cfg"]);
    assert_eq!(code, 0);
}

#[test]
fn missing_label_with_no_available_labels_reports_none() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "a.ts",
                "// LINT.IfChange\nconst A = 1;\n// LINT.ThenChange(\"b.ts#x\")\n",
            ),
            ("b.ts", "const B = 1;\n"),
        ],
    );
    let diff = make_diff(dir.path(), &[
        ("a.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const A = 1;\n+const A = 2;\n // LINT.ThenChange(\"b.ts#x\")"),
        ("b.ts", "@@ -1 +1 @@\n-const B = 1;\n+const B = 2;"),
    ]);
    let (code, stdout, _stderr) = run_lint(&diff);
    assert_eq!(code, 1);
    assert!(
        stdout.contains("Available labels: none"),
        "stdout: {}",
        stdout
    );
}

#[test]
fn target_in_diff_with_no_changed_lines_reports_expected_changes() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "a.ts",
                "// LINT.IfChange\nx=1\n// LINT.ThenChange(\"b.ts\")\n",
            ),
            ("b.ts", "one\n"),
        ],
    );
    let dir_str = dir.path().to_string_lossy().replace('\\', "/");
    let diff = format!(
        "--- a/{0}/a.ts\n+++ b/{0}/a.ts\n@@ -1,3 +1,3 @@\n // LINT.IfChange\n-x=1\n+x=2\n // LINT.ThenChange(\"b.ts\")\n--- a/{0}/b.ts\n+++ b/{0}/b.ts\n@@ -1 +1 @@\n one\n",
        dir_str
    );
    let (code, stdout, _stderr) = run_lint(&diff);
    assert_eq!(code, 1);
    assert!(stdout.contains("expected changes in"), "stdout: {}", stdout);
}

#[test]
fn binary_diff_file_does_not_crash() {
    use std::process::Stdio;

    // Write raw binary bytes to a temp file and pass as diff input
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), b"\x00\x01\x02\x03\xff\xfe\xfd\n\x80\x90\xa0\n").unwrap();
    let output = Command::new(binary_path())
        .arg(tmp.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    // Should not crash — exit 0 (no directives) or 2 (read error on invalid UTF-8)
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
    // Write raw binary bytes via stdin
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
    let (code, _stdout, _stderr) = run_lint(diff);
    assert_eq!(code, 0);
}

#[test]
fn check_mode_skips_binary_files() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("binary.rs"), b"\x00\x01\x02\x03\xff\xfe").unwrap();
    let output = Command::new(binary_path())
        .arg("--check")
        .arg(dir.path().join("binary.rs").to_string_lossy().to_string())
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    // Should not crash — may exit 0 (no directives) or 2 (read error on invalid UTF-8)
    assert!(code == 0 || code == 2, "unexpected exit code: {code}");
}

#[test]
fn oversized_diff_file_rejected() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    // Write 65 MB of text (just over 64 MB limit)
    let chunk = "a".repeat(1024 * 1024); // 1 MB
    let mut f = std::io::BufWriter::new(tmp.as_file());
    use std::io::Write;
    for _ in 0..65 {
        f.write_all(chunk.as_bytes()).unwrap();
    }
    f.flush().unwrap();
    drop(f);

    let output = Command::new(binary_path())
        .arg(tmp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap(), 2);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("too large"), "stderr: {}", stderr);
}

#[test]
fn oversized_stdin_rejected() {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new(binary_path())
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // Write 65 MB via stdin
    let chunk = vec![b'a'; 1024 * 1024]; // 1 MB
    let mut stdin = child.stdin.take().unwrap();
    for _ in 0..65 {
        if stdin.write_all(&chunk).is_err() {
            break; // process may close pipe early
        }
    }
    drop(stdin);

    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code().unwrap(), 2);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("too large"), "stderr: {}", stderr);
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
    assert!(stderr.contains("Error reading stdin"), "stderr: {}", stderr);
}
