#![allow(dead_code)]

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

pub fn binary_path() -> String {
    let path = env!("CARGO_BIN_EXE_ifchange");
    path.to_string()
}

pub fn write_files(dir: &Path, files: &[(&str, &str)]) {
    for (name, content) in files {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
    }
}

pub fn make_diff(dir: &Path, changes: &[(&str, &str)]) -> String {
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

pub fn run_lint(diff: &str, args: &[&str]) -> (i32, String, String) {
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

pub fn run_lint_stdin(input: &str, args: &[&str]) -> (i32, String, String) {
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

pub fn lint_case(
    files: &[(&str, &str)],
    changes: &[(&str, &str)],
    args: &[&str],
) -> (i32, String, String) {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), files);
    let diff = make_diff(dir.path(), changes);
    run_lint(&diff, args)
}

/// Create a diff using repo-root-relative paths (no dir prefix).
/// Use with `run_lint_in_repo` for testing absolute path resolution.
pub fn make_diff_relative(changes: &[(&str, &str)]) -> String {
    let mut diff_lines = Vec::new();
    for (file, hunk) in changes {
        diff_lines.push(format!("--- a/{}", file));
        diff_lines.push(format!("+++ b/{}", file));
        diff_lines.push(hunk.to_string());
    }
    diff_lines.join("\n")
}

/// Run the lint binary from within a fake git repo directory.
/// Creates `.git` dir in the temp dir so repo-root detection works,
/// then runs the binary with CWD set to that directory.
pub fn run_lint_in_repo(dir: &Path, diff: &str, args: &[&str]) -> (i32, String, String) {
    // Ensure .git directory exists so repo root detection works
    let git_dir = dir.join(".git");
    if !git_dir.exists() {
        fs::create_dir_all(&git_dir).unwrap();
    }
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), diff).unwrap();
    let output = Command::new(binary_path())
        .args(args)
        .arg(tmp.path())
        .current_dir(dir)
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

/// Like `lint_case` but runs inside a fake git repo with repo-root-relative diff paths.
/// Use this for testing absolute path resolution in ThenChange directives.
pub fn lint_case_repo(
    files: &[(&str, &str)],
    changes: &[(&str, &str)],
    args: &[&str],
) -> (i32, String, String) {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), files);
    let diff = make_diff_relative(changes);
    run_lint_in_repo(dir.path(), &diff, args)
}

pub fn run_scan(dir: &Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(binary_path())
        .args(args)
        .args(["-s", &dir.to_string_lossy()])
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}
