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
    assert!(stderr.contains("unchanged"), "stderr: {}", stderr);
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
            ("file1.ts", "// LINT.IfChange\nconst x = 2;\n// LINT.ThenChange(\"file2.ts#label1\")\n"),
            ("file2.ts", "// header\n// LINT.Label(\"label1\")\nconsole.log(1);\n// LINT.EndLabel\n// footer\n"),
        ],
        &[
            ("file1.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const x = 1;\n+const x = 2;\n // LINT.ThenChange(\"file2.ts#label1\")"),
            ("file2.ts", "@@ -1,5 +1,5 @@\n // header\n // LINT.Label(\"label1\")\n console.log(1);\n-// LINT.EndLabel\n+// LINT.EndLabel // changed\n // footer"),
        ],
        &[],
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("unchanged"), "stderr: {}", stderr);
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
        &[("file1.ts", "// LINT.Label(\"label1\")\nconsole.log(1);\n// LINT.EndLabel\n// LINT.IfChange\nconst x = 2;\n// LINT.ThenChange(\"#label1\")\n")],
        &[("file1.ts", "@@ -4,3 +4,3 @@\n // LINT.IfChange\n-const x = 1;\n+const x = 2;\n // LINT.ThenChange(\"#label1\")")],
        &[],
    );
    assert_eq!(
        code, 1,
        "self-reference should require label changes, stderr: {}",
        stderr
    );
}

#[test]
fn self_reference_with_label_ok() {
    let (code, _, stderr) = lint_case(
        &[("app.yml", "# LINT.IfChange\nenv:\n  DATABASE_URL: postgres://prod:5432/myapp\n# LINT.ThenChange(\"#redis\")\n# other config\n# LINT.Label(\"redis\")\nredis:\n  host: prod\n# LINT.EndLabel\n")],
        &[("app.yml", "@@ -1,9 +1,9 @@\n # LINT.IfChange\n env:\n-  DATABASE_URL: postgres://prod:5432/myapp\n+  DATABASE_URL: postgres://prod:5432/newapp\n # LINT.ThenChange(\"#redis\")\n # other config\n # LINT.Label(\"redis\")\n redis:\n-  host: prod\n+  host: staging\n # LINT.EndLabel")],
        &[],
    );
    assert_eq!(
        code, 0,
        "self-reference should pass when label section also changes, stderr: {}",
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
    assert_eq!(code, 1); // constants.py unchanged
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
        "changes outside section should not trigger, stderr: {}",
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
            ("file1.ts", "// LINT.IfChange('g')\nconst x = 2;\n// LINT.ThenChange(\"file2.ts\")\n"),
            ("file2.ts", "// dummy\n"),
        ],
        &[("file1.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange('g')\n-const x = 1;\n+const x = 2;\n // LINT.ThenChange(\"file2.ts\")")],
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
    assert!(stderr.contains("unchanged"), "stderr: {}", stderr);
}

#[test]
fn missing_target_file_reports_not_found() {
    let (code, _, stderr) = lint_case(
        &[("a.ts", "// LINT.IfChange\nconst A = 1;\n// LINT.ThenChange(\"missing.ts\")\n")],
        &[("a.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n-const A = 1;\n+const A = 2;\n // LINT.ThenChange(\"missing.ts\")")],
        &[],
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("unchanged"), "stderr: {}", stderr);
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
        stderr.contains("available:") && stderr.contains("present"),
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
    assert!(stderr.contains("available: none"), "stderr: {}", stderr);
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
    assert!(stderr.contains("unchanged"), "stderr: {}", stderr);
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
fn asymmetric_multi_file_cross_reference() {
    // source.py references 3 targets; only target_a and target_b reference back
    // target_c has no back-reference. All should pass when everything co-changes.
    let (code, _, stderr) = lint_case(
        &[
            ("source.py", "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(\"target_a.py\", \"target_b.py\", \"target_c.py\")\n"),
            ("target_a.py", "# LINT.IfChange\nMIRROR_A = 1\n# LINT.ThenChange(\"source.py\")\n"),
            ("target_b.py", "# LINT.IfChange\nMIRROR_B = 1\n# LINT.ThenChange(\"source.py\")\n"),
            ("target_c.py", "MIRROR_C = 1\n"),
        ],
        &[
            ("source.py", "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(\"target_a.py\", \"target_b.py\", \"target_c.py\")"),
            ("target_a.py", "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-MIRROR_A = 1\n+MIRROR_A = 2\n # LINT.ThenChange(\"source.py\")"),
            ("target_b.py", "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-MIRROR_B = 1\n+MIRROR_B = 2\n # LINT.ThenChange(\"source.py\")"),
            ("target_c.py", "@@ -1 +1 @@\n-MIRROR_C = 1\n+MIRROR_C = 2"),
        ],
        &[],
    );
    assert_eq!(
        code, 0,
        "asymmetric cross-refs should pass when all targets change, stderr: {}",
        stderr
    );
}

#[test]
fn asymmetric_multi_file_cross_reference_missing_one_target() {
    // source.py references 3 targets but only 2 are changed, it should fail
    let (code, _, stderr) = lint_case(
        &[
            ("source.py", "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(\"target_a.py\", \"target_b.py\", \"target_c.py\")\n"),
            ("target_a.py", "# LINT.IfChange\nMIRROR_A = 1\n# LINT.ThenChange(\"source.py\")\n"),
            ("target_b.py", "# LINT.IfChange\nMIRROR_B = 1\n# LINT.ThenChange(\"source.py\")\n"),
            ("target_c.py", "MIRROR_C = 1\n"),
        ],
        &[
            ("source.py", "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n # LINT.ThenChange(\"target_a.py\", \"target_b.py\", \"target_c.py\")"),
            ("target_a.py", "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-MIRROR_A = 1\n+MIRROR_A = 2\n # LINT.ThenChange(\"source.py\")"),
            ("target_b.py", "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-MIRROR_B = 1\n+MIRROR_B = 2\n # LINT.ThenChange(\"source.py\")"),
        ],
        &[],
    );
    assert_eq!(
        code, 1,
        "should fail when target_c.py is unchanged, stderr: {}",
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
    assert!(stderr.contains("unchanged"), "stderr: {}", stderr);
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
    assert!(stderr.contains("unchanged"), "stderr: {}", stderr);
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
    assert!(stderr.contains("unchanged"), "stderr: {}", stderr);
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
fn removed_lines_before_block_do_not_false_trigger() {
    // Regression: removing lines before an IfChange block should not trigger it.
    // Old-file removed line numbers could overlap with the IfChange range in
    // new-file coordinates, causing a false positive.
    let (code, _, stderr) = lint_case(
        &[
            (
                "readme.md",
                // After deletion: the <details> line is gone, so IfChange is now line 1
                "<!-- LINT.IfChange(\"supported-languages\") -->\nList of languages\n<!-- LINT.ThenChange(\"target.py\") -->\n",
            ),
            ("target.py", "# languages\n"),
        ],
        &[(
            "readme.md",
            // Hunk removes a line before the IfChange block, but the block content is unchanged
            "@@ -1,4 +1,3 @@\n-<details>\n <!-- LINT.IfChange(\"supported-languages\") -->\n List of languages\n <!-- LINT.ThenChange(\"target.py\") -->",
        )],
        &[],
    );
    assert_eq!(
        code, 0,
        "removing lines outside IfChange block should not trigger it, stderr: {}",
        stderr
    );
}

#[test]
fn many_removals_before_block_do_not_false_trigger() {
    // All consecutive removals collapse to the same new_line (== if_line).
    // The strict `>` in the removal check must reject them all.
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange\ncontent\n# LINT.ThenChange(\"t.py\")\n"),
            ("t.py", "x\n"),
        ],
        &[(
            "src.py",
            "@@ -1,6 +1,3 @@\n-del1\n-del2\n-del3\n # LINT.IfChange\n content\n # LINT.ThenChange(\"t.py\")",
        )],
        &[],
    );
    assert_eq!(
        code, 0,
        "multiple removals before block should not trigger, stderr: {}",
        stderr
    );
}

#[test]
fn removals_between_two_blocks_do_not_false_trigger() {
    // Removal gap position falls exactly on second block's if_line.
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange(\"a\")\nalpha\n# LINT.ThenChange(\"ta.py\")\n# LINT.IfChange(\"b\")\nbeta\n# LINT.ThenChange(\"tb.py\")\n"),
            ("ta.py", "a\n"),
            ("tb.py", "b\n"),
        ],
        &[(
            "src.py",
            "@@ -1,8 +1,6 @@\n # LINT.IfChange(\"a\")\n alpha\n # LINT.ThenChange(\"ta.py\")\n-removed1\n-removed2\n # LINT.IfChange(\"b\")\n beta\n # LINT.ThenChange(\"tb.py\")",
        )],
        &[],
    );
    assert_eq!(
        code, 0,
        "removals between blocks should not trigger either block, stderr: {}",
        stderr
    );
}

#[test]
fn pure_removal_inside_block_triggers() {
    // All content between IfChange and ThenChange deleted, no additions.
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange\n# LINT.ThenChange(\"t.py\")\n"),
            ("t.py", "x\n"),
        ],
        &[(
            "src.py",
            "@@ -1,4 +1,2 @@\n # LINT.IfChange\n-line_a\n-line_b\n # LINT.ThenChange(\"t.py\")",
        )],
        &[],
    );
    assert_eq!(
        code, 1,
        "pure removal inside block should trigger, stderr: {}",
        stderr
    );
}

#[test]
fn removals_before_and_inside_block_simultaneously() {
    // Removal before block (should not trigger) + removal inside (should trigger).
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange\nkept\n# LINT.ThenChange(\"t.py\")\n"),
            ("t.py", "x\n"),
        ],
        &[(
            "src.py",
            "@@ -1,5 +1,3 @@\n-before\n # LINT.IfChange\n-inside\n kept\n # LINT.ThenChange(\"t.py\")",
        )],
        &[],
    );
    assert_eq!(
        code, 1,
        "removal inside block should trigger even with removal before block, stderr: {}",
        stderr
    );
}

#[test]
fn removals_after_block_do_not_trigger() {
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange\ncontent\n# LINT.ThenChange(\"t.py\")\nkept\n"),
            ("t.py", "x\n"),
        ],
        &[(
            "src.py",
            "@@ -1,5 +1,4 @@\n # LINT.IfChange\n content\n # LINT.ThenChange(\"t.py\")\n-after\n kept",
        )],
        &[],
    );
    assert_eq!(
        code, 0,
        "removal after block should not trigger, stderr: {}",
        stderr
    );
}

#[test]
fn removal_only_target_diff_counts_as_changed() {
    // Target has only removals (no additions). The removal should still
    // register as a change so the lint passes.
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange\ncontent\n# LINT.ThenChange(\"t.py\")\n"),
            ("t.py", "line_a\n"),
        ],
        &[
            ("src.py", "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-content\n+content_v2\n # LINT.ThenChange(\"t.py\")"),
            ("t.py", "@@ -1,2 +1,1 @@\n line_a\n-line_b"),
        ],
        &[],
    );
    assert_eq!(
        code, 0,
        "removal-only target diff should count as changed, stderr: {}",
        stderr
    );
}

#[test]
fn addition_before_block_with_removal_inside() {
    // Addition before block shifts line numbers; removal inside should still trigger.
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "new_line\n# LINT.IfChange\nkept\n# LINT.ThenChange(\"t.py\")\n"),
            ("t.py", "x\n"),
        ],
        &[(
            "src.py",
            "@@ -1,3 +1,4 @@\n+new_line\n # LINT.IfChange\n-inside\n kept\n # LINT.ThenChange(\"t.py\")",
        )],
        &[],
    );
    assert_eq!(
        code, 1,
        "removal inside block should trigger despite addition before block, stderr: {}",
        stderr
    );
}

// ── Directive-only changes should NOT trigger co-change checks ──

#[test]
fn only_thenchange_path_changed_no_error() {
    // The original bug: updating ThenChange target path should not trigger
    let (code, _, stderr) = lint_case(
        &[
            ("src.ts", "// LINT.IfChange\nconst v = 1;\n// LINT.ThenChange(\"target.ts\")\n"),
            ("target.ts", "const v = 1;\n"),
        ],
        &[("src.ts", "@@ -1,3 +1,3 @@\n // LINT.IfChange\n const v = 1;\n-// LINT.ThenChange(\"old-target.ts\")\n+// LINT.ThenChange(\"target.ts\")")],
        &[],
    );
    assert_eq!(
        code, 0,
        "changing only ThenChange target path should not trigger, stderr: {}",
        stderr
    );
}

#[test]
fn only_ifchange_label_changed_no_error() {
    // Adding a label to IfChange should not trigger
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange('new-label')\nVALUE = 1\n# LINT.ThenChange(\"target.py\")\n"),
            ("target.py", "VALUE = 1\n"),
        ],
        &[("src.py", "@@ -1,3 +1,3 @@\n-# LINT.IfChange\n+# LINT.IfChange('new-label')\n VALUE = 1\n # LINT.ThenChange(\"target.py\")")],
        &[],
    );
    assert_eq!(
        code, 0,
        "adding label to IfChange should not trigger, stderr: {}",
        stderr
    );
}

#[test]
fn both_directives_changed_no_content_no_error() {
    // Both IfChange and ThenChange changed, but content between them is untouched
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange('renamed')\nVALUE = 1\n# LINT.ThenChange(\"target.py\")\n"),
            ("target.py", "VALUE = 1\n"),
        ],
        &[("src.py", "@@ -1,3 +1,3 @@\n-# LINT.IfChange('old')\n+# LINT.IfChange('renamed')\n VALUE = 1\n-# LINT.ThenChange(\"old-target.py\")\n+# LINT.ThenChange(\"target.py\")")],
        &[],
    );
    assert_eq!(
        code, 0,
        "changing both directives without content should not trigger, stderr: {}",
        stderr
    );
}

#[test]
fn content_and_directive_changed_still_errors() {
    // Content changed AND directive changed — should still enforce co-change
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange('label')\nVALUE = 2\n# LINT.ThenChange(\"target.py\")\n"),
            ("target.py", "VALUE = 1\n"),
        ],
        &[("src.py", "@@ -1,3 +1,3 @@\n-# LINT.IfChange\n+# LINT.IfChange('label')\n-VALUE = 1\n+VALUE = 2\n-# LINT.ThenChange(\"old.py\")\n+# LINT.ThenChange(\"target.py\")")],
        &[],
    );
    assert_eq!(
        code, 1,
        "content change with directive change should still trigger, stderr: {}",
        stderr
    );
}

#[test]
fn empty_block_directive_change_no_error() {
    // Adjacent IfChange/ThenChange with nothing between them
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange('x')\n# LINT.ThenChange(\"target.py\")\n"),
            ("target.py", "VALUE = 1\n"),
        ],
        &[("src.py", "@@ -1,2 +1,2 @@\n-# LINT.IfChange\n+# LINT.IfChange('x')\n-# LINT.ThenChange(\"old.py\")\n+# LINT.ThenChange(\"target.py\")")],
        &[],
    );
    assert_eq!(
        code, 0,
        "empty block with only directive changes should not trigger, stderr: {}",
        stderr
    );
}

#[test]
fn directive_change_does_not_affect_adjacent_pair() {
    // Two pairs in same file; changing directive of first should not affect second
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange('a')\nalpha = 1\n# LINT.ThenChange(\"ta.py\")\n# LINT.IfChange('b')\nbeta = 1\n# LINT.ThenChange(\"tb.py\")\n"),
            ("ta.py", "a = 1\n"),
            ("tb.py", "b = 1\n"),
        ],
        &[("src.py", "@@ -1,6 +1,6 @@\n # LINT.IfChange('a')\n alpha = 1\n-# LINT.ThenChange(\"old-ta.py\")\n+# LINT.ThenChange(\"ta.py\")\n # LINT.IfChange('b')\n beta = 1\n # LINT.ThenChange(\"tb.py\")")],
        &[],
    );
    assert_eq!(
        code, 0,
        "directive change in pair A should not trigger pair B, stderr: {}",
        stderr
    );
}

#[test]
fn ifchange_line1_directive_only_change_no_error() {
    // IfChange on line 1 of file, only directive changed
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange('added-label')\nVALUE = 1\n# LINT.ThenChange(\"t.py\")\n"),
            ("t.py", "x = 1\n"),
        ],
        &[("src.py", "@@ -1,3 +1,3 @@\n-# LINT.IfChange\n+# LINT.IfChange('added-label')\n VALUE = 1\n # LINT.ThenChange(\"t.py\")")],
        &[],
    );
    assert_eq!(
        code, 0,
        "IfChange at line 1 directive-only change should not trigger, stderr: {}",
        stderr
    );
}

#[test]
fn thenchange_last_line_directive_only_change_no_error() {
    // ThenChange is the last line of the file
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(\"t.py\")\n"),
            ("t.py", "x = 1\n"),
        ],
        &[("src.py", "@@ -1,3 +1,3 @@\n # LINT.IfChange\n VALUE = 1\n-# LINT.ThenChange(\"old.py\")\n+# LINT.ThenChange(\"t.py\")")],
        &[],
    );
    assert_eq!(
        code, 0,
        "ThenChange at last line directive-only change should not trigger, stderr: {}",
        stderr
    );
}

#[test]
fn content_and_thenchange_changed_new_target_also_changed() {
    // Content changed AND ThenChange target updated, new target also changed → PASS
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange\nVALUE = 2\n# LINT.ThenChange(\"new-target.py\")\n"),
            ("new-target.py", "MIRROR = 2\n"),
        ],
        &[
            ("src.py", "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n-# LINT.ThenChange(\"old-target.py\")\n+# LINT.ThenChange(\"new-target.py\")"),
            ("new-target.py", "@@ -1 +1 @@\n-MIRROR = 1\n+MIRROR = 2"),
        ],
        &[],
    );
    assert_eq!(
        code, 0,
        "content + directive change with new target also changed should pass, stderr: {}",
        stderr
    );
}

#[test]
fn content_and_thenchange_changed_new_target_not_changed() {
    // Content changed AND ThenChange target updated, but new target NOT changed → ERROR
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange\nVALUE = 2\n# LINT.ThenChange(\"new-target.py\")\n"),
            ("new-target.py", "MIRROR = 1\n"),
        ],
        &[("src.py", "@@ -1,3 +1,3 @@\n # LINT.IfChange\n-VALUE = 1\n+VALUE = 2\n-# LINT.ThenChange(\"old-target.py\")\n+# LINT.ThenChange(\"new-target.py\")")],
        &[],
    );
    assert_eq!(
        code, 1,
        "content change with new target not changed should error, stderr: {}",
        stderr
    );
}

#[test]
fn content_added_to_previously_empty_block_triggers() {
    // Adding content to a previously empty block (IfChange immediately above ThenChange)
    let (code, _, stderr) = lint_case(
        &[
            (
                "src.py",
                "# LINT.IfChange\nnew_content = 1\n# LINT.ThenChange(\"t.py\")\n",
            ),
            ("t.py", "x = 1\n"),
        ],
        &[(
            "src.py",
            "@@ -1,2 +1,3 @@\n # LINT.IfChange\n+new_content = 1\n # LINT.ThenChange(\"t.py\")",
        )],
        &[],
    );
    assert_eq!(
        code, 1,
        "adding content to empty block should trigger, stderr: {}",
        stderr
    );
}

#[test]
fn thenchange_replaced_with_removal_at_same_position_no_error() {
    // When ThenChange is replaced, both a removal and addition land on
    // then_line. The removal should not falsely trigger the co-change check.
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange(\"t.py\")\n"),
            ("t.py", "x = 1\n"),
        ],
        &[("src.py", "@@ -1,3 +1,3 @@\n # LINT.IfChange\n VALUE = 1\n-# LINT.ThenChange(\"old.py\")\n+# LINT.ThenChange(\"t.py\")")],
        &[],
    );
    assert_eq!(
        code, 0,
        "ThenChange replacement should not trigger, stderr: {}",
        stderr
    );
}

// ── Boundary collapse regression tests ──
// When content lines are deleted alongside a directive rewrite, all removals
// map to the same new-file line number.  The trigger check must still detect
// the content deletion even though it shares a position with the directive.

#[test]
fn content_deleted_with_thenchange_rewrite_triggers() {
    // Delete the only content line while rewriting ThenChange.
    // Both removals collapse onto then_line; must still trigger.
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange\n# LINT.ThenChange(\"new.py\")\n"),
            ("new.py", "x = 1\n"),
        ],
        &[(
            "src.py",
            "@@ -1,3 +1,2 @@\n # LINT.IfChange\n-old_value\n-# LINT.ThenChange(\"old.py\")\n+# LINT.ThenChange(\"new.py\")",
        )],
        &[],
    );
    assert_eq!(
        code, 1,
        "content deletion collapsing onto ThenChange rewrite should trigger, stderr: {}",
        stderr
    );
}

#[test]
fn content_deleted_with_ifchange_rewrite_triggers() {
    // Delete the first content line while rewriting IfChange.
    // Both removals collapse onto if_line; must still trigger.
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange(label)\n# LINT.ThenChange(\"t.py\")\n"),
            ("t.py", "x = 1\n"),
        ],
        &[(
            "src.py",
            "@@ -1,3 +1,2 @@\n-# LINT.IfChange\n-old_value\n+# LINT.IfChange(label)\n # LINT.ThenChange(\"t.py\")",
        )],
        &[],
    );
    assert_eq!(
        code, 1,
        "content deletion collapsing onto IfChange rewrite should trigger, stderr: {}",
        stderr
    );
}

#[test]
fn multiple_content_deleted_with_thenchange_rewrite_triggers() {
    // Delete multiple content lines while rewriting ThenChange.
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange\n# LINT.ThenChange(\"new.py\")\n"),
            ("new.py", "x = 1\n"),
        ],
        &[(
            "src.py",
            "@@ -1,4 +1,2 @@\n # LINT.IfChange\n-line_a\n-line_b\n-# LINT.ThenChange(\"old.py\")\n+# LINT.ThenChange(\"new.py\")",
        )],
        &[],
    );
    assert_eq!(
        code, 1,
        "multiple content deletions with ThenChange rewrite should trigger, stderr: {}",
        stderr
    );
}

// ── Regression: outside-of-block deletion + directive rewrite ──
// CodeRabbit flagged that deletion before/after a block combined with a
// directive rewrite could false-trigger because removal_line_counts
// collapses both sides into one count.

#[test]
fn delete_before_block_with_ifchange_rewrite_no_trigger() {
    // Delete a line BEFORE the block while rewriting IfChange.
    // Both removals collapse onto if_line with count=2, but the
    // deletions are outside the block — must NOT trigger.
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange(\"x\")\ncontent\n# LINT.ThenChange(\"t.py\")\n"),
            ("t.py", "x = 1\n"),
        ],
        &[(
            "src.py",
            "@@ -1,4 +1,3 @@\n-before\n-# LINT.IfChange\n+# LINT.IfChange(\"x\")\n content\n # LINT.ThenChange(\"t.py\")",
        )],
        &[],
    );
    assert_eq!(
        code, 0,
        "delete before block + IfChange rewrite should not trigger, stderr: {}",
        stderr
    );
}

#[test]
fn delete_after_block_with_thenchange_rewrite_no_trigger() {
    // Delete a line AFTER the block while rewriting ThenChange.
    // Both removals collapse onto then_line with count=2, but the
    // deletions are outside the block — must NOT trigger.
    let (code, _, stderr) = lint_case(
        &[
            ("src.py", "# LINT.IfChange\ncontent\n# LINT.ThenChange(\"new.py\")\n"),
            ("new.py", "x = 1\n"),
        ],
        &[(
            "src.py",
            "@@ -1,4 +1,3 @@\n # LINT.IfChange\n content\n-# LINT.ThenChange(\"old.py\")\n-after\n+# LINT.ThenChange(\"new.py\")",
        )],
        &[],
    );
    assert_eq!(
        code, 0,
        "delete after block + ThenChange rewrite should not trigger, stderr: {}",
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
    assert!(stderr.contains("unchanged"), "stderr: {}", stderr);
}
