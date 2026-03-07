mod common;

use common::*;
use tempfile::TempDir;

#[test]
fn empty_diff() {
    let (code, _stdout, _stderr) = run_lint("", &[]);
    assert_eq!(code, 0);
}

#[test]
fn no_error_when_target_changed() {
    let (code, _, stderr) = lint_case(
        &[
            ("file1.ts", "// LINT.IfChange\nconst v = 1;\n// LINT.ThenChange(\"file2.ts\")\n"),
            ("file2.ts", "const v = 1;\n"),
        ],
        &[
            ("file1.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const v = 1;\n+const v = 2;\n // LINT.ThenChange(\"file2.ts\")"),
            ("file2.ts", "@@ -1 +1 @@\n-const v = 1;\n+const v = 2;"),
        ],
        &[],
    );
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn error_when_target_not_changed() {
    let (code, _, stderr) = lint_case(
        &[
            ("file1.ts", "// LINT.IfChange\nconst v = 1;\n// LINT.ThenChange(\"file2.ts\")\n"),
            ("file2.ts", "const v = 1;\n"),
        ],
        &[("file1.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const v = 1;\n+const v = 2;\n // LINT.ThenChange(\"file2.ts\")")],
        &[],
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("not changed"), "stderr: {}", stderr);
}

#[test]
fn labeled_change_ok() {
    let (code, _, stderr) = lint_case(
        &[
            ("file1.ts", "// LINT.IfChange\n// LINT.ThenChange(\"file2.ts#label1\")\n"),
            ("file2.ts", "// header\n// LINT.Label(\"label1\")\nconsole.log(1);\n// LINT.EndLabel\n// footer\n"),
        ],
        &[
            ("file1.ts", "@@ -1,2 +1,2 @@\n-// LINT.IfChange\n+// LINT.IfChange // changed\n // LINT.ThenChange(\"file2.ts#label1\")"),
            ("file2.ts", "@@ -1,5 +1,5 @@\n // header\n // LINT.Label(\"label1\")\n-console.log(1);\n+console.log(2);\n // LINT.EndLabel\n // footer"),
        ],
        &[],
    );
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn labeled_change_missing() {
    let (code, _, stderr) = lint_case(
        &[
            ("file1.ts", "// LINT.IfChange\n// LINT.ThenChange(\"file2.ts#label1\")\n"),
            ("file2.ts", "// header\n// LINT.Label(\"label1\")\nconsole.log(1);\n// LINT.EndLabel\n// footer\n"),
        ],
        &[
            ("file1.ts", "@@ -1,2 +1,2 @@\n-// LINT.IfChange\n+// LINT.IfChange // changed\n // LINT.ThenChange(\"file2.ts#label1\")"),
            ("file2.ts", "@@ -1,5 +1,5 @@\n // header\n // LINT.Label(\"label1\")\n console.log(1);\n-// LINT.EndLabel\n+// LINT.EndLabel // changed\n // footer"),
        ],
        &[],
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("expected changes"), "stderr: {}", stderr);
}

#[test]
fn orphan_then_change() {
    let (code, _, stderr) = lint_case(
        &[("file1.ts", "// LINT.ThenChange(\"foo.ts\")\n")],
        &[("file1.ts", "@@ -1 +1 @@\n-// LINT.ThenChange(\"foo.ts\")\n+// LINT.ThenChange(\"foo.ts\") // changed")],
        &[],
    );
    assert_eq!(code, 1);
    assert!(
        stderr.contains("unexpected ThenChange"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn orphan_if_change() {
    let (code, _, stderr) = lint_case(
        &[("file1.ts", "// LINT.IfChange\n")],
        &[(
            "file1.ts",
            "@@ -1 +1 @@\n-// LINT.IfChange\n+// LINT.IfChange // changed",
        )],
        &[],
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("missing ThenChange"), "stderr: {}", stderr);
}

#[test]
fn cross_reference_ignores_outside_changes() {
    let (code, _, stderr) = lint_case(
        &[
            ("source.py", "# Header\ndef helper():\n    return 1\n# LINT.IfChange\nclass Status:\n    ACTIVE = 1\n# LINT.ThenChange(\"target.py\")\ndef other():\n    return 2\n"),
            ("target.py", "# LINT.IfChange\nSTATUS = [1]\n# LINT.ThenChange(\"source.py\")\ndef target_helper():\n    return 1\n"),
        ],
        &[("source.py", "@@ -1,9 +1,9 @@\n # Header\n-def helper():\n+def helper_modified():\n     return 1\n # LINT.IfChange\n class Status:\n     ACTIVE = 1\n # LINT.ThenChange(\"target.py\")\n-def other():\n+def other_modified():\n     return 2")],
        &[],
    );
    assert_eq!(
        code, 0,
        "changes outside IfChange should not trigger, stderr: {}",
        stderr
    );
}

#[test]
fn cross_reference_detects_inside_changes() {
    let (code, _, stderr) = lint_case(
        &[
            ("source.py", "# Header\n# LINT.IfChange\nclass Status:\n    ACTIVE = 1\n    INACTIVE = 2\n# LINT.ThenChange(\"target.py\")\ndef other():\n    return 2\n"),
            ("target.py", "# LINT.IfChange\nSTATUS = [1, 2]\n# LINT.ThenChange(\"source.py\")\n"),
        ],
        &[("source.py", "@@ -2,5 +2,5 @@\n # LINT.IfChange\n class Status:\n     ACTIVE = 1\n-    INACTIVE = 2\n+    PENDING = 3\n # LINT.ThenChange(\"target.py\")")],
        &[],
    );
    assert_eq!(
        code, 1,
        "changes inside IfChange should trigger, stderr: {}",
        stderr
    );
}

#[test]
fn self_reference_with_label() {
    let (code, _, stderr) = lint_case(
        &[("file1.ts", "// LINT.Label(\"label1\")\nconsole.log(1);\n// LINT.EndLabel\n// LINT.IfChange\n// LINT.ThenChange(\"#label1\")\n")],
        &[("file1.ts", "@@ -4,4 +4,4 @@\n-// LINT.IfChange\n+// LINT.IfChange // changed")],
        &[],
    );
    assert_eq!(
        code, 1,
        "self-reference should require label changes, stderr: {}",
        stderr
    );
}

#[test]
fn python_hash_comments() {
    let (code, _, _) = lint_case(
        &[
            ("config.py", "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(\"constants.py\")\n"),
            ("constants.py", "VALUE = 1\n"),
        ],
        &[("config.py", "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(\"constants.py\")")],
        &[],
    );
    assert_eq!(code, 1); // constants.py not changed
}

#[test]
fn no_change_outside_block() {
    let (code, _, stderr) = lint_case(
        &[
            ("file1.ts", "const other = 0;\n// LINT.IfChange\nconst v = 1;\n// LINT.ThenChange(\"file2.ts\")\nconst more = 2;\n"),
            ("file2.ts", "const v = 1;\n"),
        ],
        &[("file1.ts", "@@ -1,5 +1,5 @@\n-const other = 0;\n+const other = 99;\n // LINT.IfChange\n const v = 1;\n // LINT.ThenChange(\"file2.ts\")\n const more = 2;")],
        &[],
    );
    assert_eq!(
        code, 0,
        "changes outside block should not trigger, stderr: {}",
        stderr
    );
}

#[test]
fn deleted_files_ignored() {
    let diff = "--- a/deleted.ts\n+++ /dev/null\n@@ -1,3 +0,0 @@\n-// LINT.IfChange\n-value = 1\n-// LINT.ThenChange(\"other.ts\")\n";
    let (code, _, _) = run_lint(diff, &[]);
    assert_eq!(code, 0);
}

#[test]
fn ifchange_label_in_error_context() {
    let (code, _, stderr) = lint_case(
        &[
            ("file1.ts", "// LINT.IfChange('g')\n// LINT.ThenChange(\"file2.ts\")\n"),
            ("file2.ts", "// dummy\n"),
        ],
        &[("file1.ts", "@@ -1,2 +1,2 @@\n-// LINT.IfChange('g')\n+// LINT.IfChange('g') // changed\n // LINT.ThenChange(\"file2.ts\")")],
        &[],
    );
    assert_eq!(code, 1);
    assert!(
        stderr.contains("#g:"),
        "error should include label context, stderr: {}",
        stderr
    );
}

#[test]
fn changed_file_parse_error_reported() {
    let (code, _, stderr) = lint_case(
        &[("bad.ts", "// LINT.IfChange(\n")],
        &[(
            "bad.ts",
            "@@ -1 +1 @@\n-// LINT.IfChange(\n+// LINT.IfChange(",
        )],
        &[],
    );
    assert_eq!(code, 1);
    assert!(
        stderr.contains("Malformed LINT.IfChange"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn duplicate_labels_in_changed_file_reported() {
    let (code, _, stderr) = lint_case(
        &[("dup.ts", "// LINT.IfChange(\"x\")\n// LINT.Label(\"x\")\n// LINT.ThenChange(\"other.ts\")\n")],
        &[("dup.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange(\"x\")\n // LINT.Label(\"x\")\n-// LINT.ThenChange(\"other.ts\")\n+// LINT.ThenChange(\"other.ts\") // changed")],
        &[],
    );
    assert_eq!(code, 1);
    assert!(
        stderr.contains("duplicate directive label"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn malformed_target_file_reports_not_found() {
    let (code, _, stderr) = lint_case(
        &[
            ("a.ts", "// LINT.IfChange\nconst A = 1;\n// LINT.ThenChange(\"b.ts\")\n"),
            ("b.ts", "// LINT.IfChange(\n"),
        ],
        &[("a.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const A = 1;\n+const A = 2;\n // LINT.ThenChange(\"b.ts\")")],
        &[],
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("not found"), "stderr: {}", stderr);
}

#[test]
fn missing_target_file_reports_not_found() {
    let (code, _, stderr) = lint_case(
        &[("a.ts", "// LINT.IfChange\nconst A = 1;\n// LINT.ThenChange(\"missing.ts\")\n")],
        &[("a.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const A = 1;\n+const A = 2;\n // LINT.ThenChange(\"missing.ts\")")],
        &[],
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("not found"), "stderr: {}", stderr);
}

#[test]
fn missing_target_label_reports_available_labels() {
    let (code, _, stderr) = lint_case(
        &[
            ("a.ts", "// LINT.IfChange\nconst A = 1;\n// LINT.ThenChange(\"b.ts#missing\")\n"),
            ("b.ts", "// LINT.Label(\"present\")\nlet x = 1;\n// LINT.EndLabel\n"),
        ],
        &[
            ("a.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const A = 1;\n+const A = 2;\n // LINT.ThenChange(\"b.ts#missing\")"),
            ("b.ts", "@@ -1,3 +1,3 @@\n // LINT.Label(\"present\")\n-let x = 1;\n+let x = 2;\n // LINT.EndLabel"),
        ],
        &[],
    );
    assert_eq!(code, 1);
    assert!(
        stderr.contains("label 'missing' not found"),
        "stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("Available labels: present"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn multiple_ifchange_marks_first_orphan() {
    let (code, _, stderr) = lint_case(
        &[("a.ts", "// LINT.IfChange(\"first\")\n// LINT.IfChange(\"second\")\nconst x = 1;\n// LINT.ThenChange(\"b.ts\")\n")],
        &[("a.ts", "@@ -1,4 +1,4 @@\n // LINT.IfChange(\"first\")\n // LINT.IfChange(\"second\")\n-const x = 1;\n+const x = 2;\n // LINT.ThenChange(\"b.ts\")")],
        &[],
    );
    assert_eq!(code, 1);
    assert!(
        stderr.contains("missing ThenChange after IfChange('first')"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn phase2_duplicate_labels_in_target_file_reported() {
    let (code, _, stderr) = lint_case(
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
        &[(
            "a.ts",
            "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-x=1\n+x=2\n // LINT.ThenChange(\"b.ts\")",
        )],
        &[],
    );
    assert_eq!(code, 1);
    assert!(
        stderr.contains("duplicate directive label 'dup'"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn missing_label_with_no_available_labels_reports_none() {
    let (code, _, stderr) = lint_case(
        &[
            ("a.ts", "// LINT.IfChange\nconst A = 1;\n// LINT.ThenChange(\"b.ts#x\")\n"),
            ("b.ts", "const B = 1;\n"),
        ],
        &[
            ("a.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const A = 1;\n+const A = 2;\n // LINT.ThenChange(\"b.ts#x\")"),
            ("b.ts", "@@ -1 +1 @@\n-const B = 1;\n+const B = 2;"),
        ],
        &[],
    );
    assert_eq!(code, 1);
    assert!(
        stderr.contains("Available labels: none"),
        "stderr: {}",
        stderr
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
    let (code, _, stderr) = run_lint(&diff, &[]);
    assert_eq!(code, 1);
    assert!(stderr.contains("expected changes"), "stderr: {}", stderr);
}

#[test]
fn cross_ref_trigger_scoped_to_specific_block() {
    let (code, _, stderr) = lint_case(
        &[
            ("source.py", "# LINT.IfChange(\"block_a\")\nVALUE_A = 1\n# LINT.ThenChange(\"target_a.py\")\n# other\n# LINT.IfChange(\"block_b\")\nVALUE_B = 1\n# LINT.ThenChange(\"target_b.py\")\n"),
            ("target_a.py", "# LINT.IfChange\nMIRROR_A = 1\n# LINT.ThenChange(\"source.py\")\n"),
            ("target_b.py", "# LINT.IfChange\nMIRROR_B = 1\n# LINT.ThenChange(\"source.py\")\n"),
        ],
        &[
            ("source.py", "@@ -5,3 +5,3 @@\n # LINT.IfChange(\"block_b\")\n-VALUE_B = 1\n+VALUE_B = 2\n # LINT.ThenChange(\"target_b.py\")"),
            ("target_b.py", "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-MIRROR_B = 1\n+MIRROR_B = 2\n # LINT.ThenChange(\"source.py\")"),
        ],
        &[],
    );
    assert_eq!(
        code, 0,
        "changing block_b should not trigger block_a, stderr: {}",
        stderr
    );
}

#[test]
fn filename_with_spaces_trailing_tab() {
    let dir = TempDir::new().unwrap();
    write_files(
        dir.path(),
        &[
            (
                "my dir/my file.py",
                "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(\"other file.py\")\n",
            ),
            ("my dir/other file.py", "VALUE = 1\n"),
        ],
    );
    let dir_str = dir.path().to_string_lossy().replace('\\', "/");
    let diff = format!(
        "--- a/{0}/my dir/my file.py\t\n+++ b/{0}/my dir/my file.py\t\n@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(\"other file.py\")\n",
        dir_str
    );
    let (code, _, stderr) = run_lint(&diff, &[]);
    assert_eq!(
        code, 1,
        "should detect violation despite trailing tab, stderr: {}",
        stderr
    );
    assert!(stderr.contains("not changed"), "stderr: {}", stderr);
}

#[test]
fn bom_does_not_break_directives() {
    let (code, _, stderr) = lint_case(
        &[
            ("bom.py", "\u{FEFF}# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(\"other.py\")\n"),
            ("other.py", "VALUE = 1\n"),
        ],
        &[("bom.py", "@@ -1,3 +1,3 @@\n \u{FEFF}# LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(\"other.py\")")],
        &[],
    );
    assert_eq!(
        code, 1,
        "BOM file should still detect directives, stderr: {}",
        stderr
    );
    assert!(stderr.contains("not changed"), "stderr: {}", stderr);
}

#[test]
fn lint_mixed_case_directives() {
    let (code, _, stderr) = lint_case(
        &[
            ("source.py", "# lint.ifchange\nVALUE = 1\n# LINT.ThenChange(\"target.py\")\n"),
            ("target.py", "VALUE = 1\n"),
        ],
        &[
            ("source.py", "@@ -1,3 +1,3 @@\n # lint.ifchange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(\"target.py\")"),
            ("target.py", "@@ -1 +1 @@\n-VALUE = 1\n+VALUE = 2"),
        ],
        &[],
    );
    assert_eq!(
        code, 0,
        "mixed-case directives should work in lint mode, stderr: {}",
        stderr
    );
}

#[test]
fn lint_fully_lowercase_directives() {
    let (code, _, stderr) = lint_case(
        &[
            ("source.ts", "// lint.ifchange(\"block\")\nconst v = 1;\n// lint.thenchange(\"target.ts\")\n"),
            ("target.ts", "const v = 1;\n"),
        ],
        &[("source.ts", "@@ -1,3 +1,3 @@\n // lint.ifchange(\"block\")\n-const v = 1;\n+const v = 2;\n // lint.thenchange(\"target.ts\")")],
        &[],
    );
    assert_eq!(
        code, 1,
        "lowercase directives should still trigger lint, stderr: {}",
        stderr
    );
    assert!(stderr.contains("not changed"), "stderr: {}", stderr);
}

#[test]
fn lint_mixed_case_label_reference() {
    let (code, _, stderr) = lint_case(
        &[
            ("source.ts", "// LINT.IfChange\nconst v = 1;\n// LINT.ThenChange(\"target.ts#sec\")\n"),
            ("target.ts", "// lint.label(\"sec\")\nconst t = 1;\n// lint.endlabel\n"),
        ],
        &[
            ("source.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const v = 1;\n+const v = 2;\n // LINT.ThenChange(\"target.ts#sec\")"),
            ("target.ts", "@@ -1,3 +1,3 @@\n // lint.label(\"sec\")\n-const t = 1;\n+const t = 2;\n // lint.endlabel"),
        ],
        &[],
    );
    assert_eq!(
        code, 0,
        "lowercase label/endlabel in target should work, stderr: {}",
        stderr
    );
}

#[test]
fn thenchange_path_is_case_sensitive() {
    let (code, _, stderr) = lint_case(
        &[
            ("source.py", "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(\"FOO.txt\")\n"),
            ("foo.txt", "# data\n"),
        ],
        &[
            ("source.py", "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(\"FOO.txt\")"),
            ("foo.txt", "@@ -1 +1 @@\n-# data\n+# updated data"),
        ],
        &[],
    );
    assert_eq!(
        code, 1,
        "case-different path should not match, stderr: {}",
        stderr
    );
}

#[test]
fn errors_go_to_stderr_not_stdout() {
    let (code, stdout, stderr) = lint_case(
        &[
            ("a.ts", "// LINT.IfChange\nconst v = 1;\n// LINT.ThenChange(\"b.ts\")\n"),
            ("b.ts", "const v = 1;\n"),
        ],
        &[("a.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const v = 1;\n+const v = 2;\n // LINT.ThenChange(\"b.ts\")")],
        &[],
    );
    assert_eq!(code, 1);
    assert!(stdout.is_empty(), "stdout should be empty, got: {}", stdout);
    assert!(stderr.contains("not changed"), "stderr: {}", stderr);
}
