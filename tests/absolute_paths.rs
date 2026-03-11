mod common;

use common::*;
use tempfile::TempDir;

#[test]
fn bare_slash_thenchange() {
    let (code, _, stderr) = lint_case_repo(
        &[(
            "src/a.py",
            "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(/)\n",
        )],
        &[(
            "src/a.py",
            "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(/)",
        )],
        &[],
    );
    assert_ne!(
        code, 0,
        "bare slash ThenChange(/) should not silently pass, stderr: {}",
        stderr
    );
}

#[test]
fn path_traversal_escape_attempt() {
    let (code, _, stderr) = lint_case_repo(
        &[(
            "src/a.py",
            "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(/../../etc/passwd)\n",
        )],
        &[(
            "src/a.py",
            "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(/../../etc/passwd)",
        )],
        &[],
    );
    // Should report an error (target unchanged / missing), not silently pass.
    assert_eq!(
        code, 1,
        "path traversal should be contained and report error, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("etc/passwd"),
        "error should reference the resolved path, stderr: {}",
        stderr
    );
}

#[test]
fn absolute_and_relative_resolve_to_same_target() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "src/lib/a.py",
                "# LINT.IfChange(\"block_rel\")\nVALUE_A = 1\n# LINT.ThenChange(\"../api.py\")\n# LINT.IfChange(\"block_abs\")\nVALUE_B = 1\n# LINT.ThenChange(/src/api.py)\n",
            ),
            ("src/api.py", "API = 1\n"),
        ],
    );
    let diff = make_diff_relative(&[(
        "src/lib/a.py",
        "@@ -1,6 +1,6 @@\n # LINT.IfChange(\"block_rel\")\n-VALUE_A = 1\n+VALUE_A = 2\n # LINT.ThenChange(\"../api.py\")\n # LINT.IfChange(\"block_abs\")\n-VALUE_B = 1\n+VALUE_B = 2\n # LINT.ThenChange(/src/api.py)",
    )]);
    let (code, _, stderr) = run_lint_in_repo(dir.path(), &diff, &[]);
    // Both blocks point to the same file; both should fail since api.py is unchanged.
    assert_eq!(
        code, 1,
        "both absolute and relative should resolve to src/api.py, stderr: {}",
        stderr
    );
    let error_count = stderr.matches("unchanged").count();
    assert_eq!(
        error_count, 2,
        "expected 2 'unchanged' errors (one per block), got {}, stderr: {}",
        error_count, stderr
    );
}

#[test]
fn self_reference_absolute_path_with_label() {
    let (code, _, stderr) = lint_case_repo(
        &[(
            "src/lib/widget.py",
            "# LINT.Label(\"config\")\nHOST = 'localhost'\n# LINT.EndLabel\n# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(/src/lib/widget.py#config)\n",
        )],
        &[(
            "src/lib/widget.py",
            "@@ -1,6 +1,6 @@\n # LINT.Label(\"config\")\n-HOST = 'localhost'\n+HOST = 'production'\n # LINT.EndLabel\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(/src/lib/widget.py#config)",
        )],
        &[],
    );
    assert_eq!(
        code, 0,
        "self-reference via absolute path should work when label section changed, stderr: {}",
        stderr
    );
}

#[test]
fn self_reference_absolute_path_with_label_unchanged() {
    let (code, _, stderr) = lint_case_repo(
        &[(
            "src/lib/widget.py",
            "# LINT.Label(\"config\")\nHOST = 'localhost'\n# LINT.EndLabel\n# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(/src/lib/widget.py#config)\n",
        )],
        &[(
            "src/lib/widget.py",
            "@@ -4,3 +4,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(/src/lib/widget.py#config)",
        )],
        &[],
    );
    assert_eq!(
        code, 1,
        "self-reference via absolute path should fail when label section unchanged, stderr: {}",
        stderr
    );
    assert!(stderr.contains("unchanged"), "stderr: {}", stderr);
}

#[test]
fn scan_mode_accepts_mixed_absolute_and_relative() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[(
            "a.py",
            "# LINT.IfChange(\"rel\")\nA = 1\n# LINT.ThenChange(\"../b.py\")\n# LINT.IfChange(\"abs\")\nB = 1\n# LINT.ThenChange(/src/b.py)\n",
        )],
    );
    let (code, _, stderr) = run_scan(dir.path(), &["--no-lint", "-v"]);
    assert_eq!(
        code, 0,
        "scan should accept both absolute and relative paths, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("2 pairs"),
        "should detect 2 pairs, stderr: {}",
        stderr
    );
}

#[test]
fn unicode_in_absolute_path() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "src/a.py",
                "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(/src/caf\u{00E9}.py)\n",
            ),
            ("src/caf\u{00E9}.py", "DATA = 1\n"),
        ],
    );
    // Simulate git's octal encoding for the unicode filename in diff headers
    let diff = "--- a/src/a.py\n+++ b/src/a.py\n@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(/src/caf\u{00E9}.py)\n--- \"a/src/caf\\303\\251.py\"\n+++ \"b/src/caf\\303\\251.py\"\n@@ -1 +1 @@\n-DATA = 1\n+DATA = 2\n".to_string();
    let (code, _, stderr) = run_lint_in_repo(dir.path(), &diff, &[]);
    assert_eq!(
        code, 0,
        "unicode absolute path should match octal-decoded diff path, stderr: {}",
        stderr
    );
}

#[test]
fn absolute_path_with_dot_and_dotdot() {
    let (code, _, stderr) = lint_case_repo(
        &[
            (
                "deep/nested/a.py",
                "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(/src/./sub/../api.py)\n",
            ),
            ("src/api.py", "API = 1\n"),
        ],
        &[
            (
                "deep/nested/a.py",
                "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(/src/./sub/../api.py)",
            ),
            ("src/api.py", "@@ -1 +1 @@\n-API = 1\n+API = 2"),
        ],
        &[],
    );
    assert_eq!(
        code, 0,
        "/src/./sub/../api.py should normalize to src/api.py, stderr: {}",
        stderr
    );
}

#[test]
fn multiple_absolute_targets_in_array() {
    let (code, _, stderr) = lint_case_repo(
        &[
            (
                "main.py",
                "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(/lib/a.py, /lib/b.py)\n",
            ),
            ("lib/a.py", "A = 1\n"),
            ("lib/b.py", "B = 1\n"),
        ],
        &[
            (
                "main.py",
                "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(/lib/a.py, /lib/b.py)",
            ),
            ("lib/a.py", "@@ -1 +1 @@\n-A = 1\n+A = 2"),
            ("lib/b.py", "@@ -1 +1 @@\n-B = 1\n+B = 2"),
        ],
        &[],
    );
    assert_eq!(
        code, 0,
        "multiple absolute targets should all resolve correctly, stderr: {}",
        stderr
    );
}

#[test]
fn multiple_absolute_targets_one_missing() {
    let (code, _, stderr) = lint_case_repo(
        &[
            (
                "main.py",
                "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(/lib/a.py, /lib/b.py)\n",
            ),
            ("lib/a.py", "A = 1\n"),
            ("lib/b.py", "B = 1\n"),
        ],
        &[
            (
                "main.py",
                "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(/lib/a.py, /lib/b.py)",
            ),
            ("lib/a.py", "@@ -1 +1 @@\n-A = 1\n+A = 2"),
            // lib/b.py intentionally NOT in diff
        ],
        &[],
    );
    assert_eq!(
        code, 1,
        "should fail when one of multiple absolute targets is unchanged, stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("lib/b.py"),
        "error should mention the unchanged target, stderr: {}",
        stderr
    );
}

#[test]
fn run_from_subdirectory_with_absolute_target() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "src/a.py",
                "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(/lib/b.py)\n",
            ),
            ("lib/b.py", "B = 1\n"),
        ],
    );
    // Create .git at root
    std::fs::create_dir_all(dir.path().join(".git")).unwrap();
    // Create the subdirectory we'll run from
    std::fs::create_dir_all(dir.path().join("src")).unwrap();

    let diff = make_diff_relative(&[
        (
            "src/a.py",
            "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(/lib/b.py)",
        ),
        ("lib/b.py", "@@ -1 +1 @@\n-B = 1\n+B = 2"),
    ]);
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), &diff).unwrap();
    // Run from src/ subdirectory
    let output = std::process::Command::new(binary_path())
        .arg(tmp.path())
        .current_dir(dir.path().join("src"))
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert_eq!(
        code, 0,
        "absolute target should resolve correctly when running from subdirectory, stderr: {}",
        stderr
    );
}

// Regression: relative paths must still work when running from a subdirectory.
#[test]
fn run_from_subdirectory_relative_target_still_works() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "src/a.py",
                "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(\"b.py\")\n",
            ),
            ("src/b.py", "B = 1\n"),
        ],
    );
    std::fs::create_dir_all(dir.path().join(".git")).unwrap();

    let diff = make_diff_relative(&[
        (
            "src/a.py",
            "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(\"b.py\")",
        ),
        ("src/b.py", "@@ -1 +1 @@\n-B = 1\n+B = 2"),
    ]);
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), &diff).unwrap();
    // Run from src/ subdirectory
    let output = std::process::Command::new(binary_path())
        .arg(tmp.path())
        .current_dir(dir.path().join("src"))
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert_eq!(
        code, 0,
        "relative target should still work from subdirectory, stderr: {}",
        stderr
    );
}

#[test]
fn no_git_dir_absolute_path_from_correct_cwd() {
    let dir = TempDir::new().unwrap();
    // No .git directory
    write_files(
        dir.path(),
        &[
            (
                "src/a.py",
                "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(/lib/b.py)\n",
            ),
            ("lib/b.py", "B = 1\n"),
        ],
    );
    let diff = make_diff_relative(&[
        (
            "src/a.py",
            "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(/lib/b.py)",
        ),
        ("lib/b.py", "@@ -1 +1 @@\n-B = 1\n+B = 2"),
    ]);
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), &diff).unwrap();
    // Run from the dir itself (where files are relative to)
    let output = std::process::Command::new(binary_path())
        .arg(tmp.path())
        .current_dir(dir.path())
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    // Without .git, the tool stays in CWD. Absolute path /lib/b.py resolves to lib/b.py
    // relative to CWD, which IS the temp dir. So this should work.
    assert_eq!(
        code, 0,
        "absolute path should work from correct CWD even without .git, stderr: {}",
        stderr
    );
}
