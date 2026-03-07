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
