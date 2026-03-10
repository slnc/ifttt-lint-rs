use std::path::Path;

/// SCM marker directories/files to detect repo root.
const SCM_MARKERS: &[&str] = &[
    ".git",
    ".hg",
    ".jj",
    ".svn",
    ".pijul",
    ".fslckout",
    "_FOSSIL_",
];

/// Walk up from `start` looking for a known SCM marker (directory or file).
/// Returns the repo root directory, or `None` if not found.
pub fn find_repo_root(start: &Path) -> Option<std::path::PathBuf> {
    let mut dir = if start.is_absolute() {
        start.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(start)
    };
    // If start is a file, begin from its parent directory.
    if dir.is_file() {
        dir = dir.parent()?.to_path_buf();
    }
    loop {
        for marker in SCM_MARKERS {
            if dir.join(marker).exists() {
                return Some(dir);
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

pub(super) fn split_target_label(target: &str) -> (&str, Option<&str>) {
    if let Some(idx) = target.find('#') {
        (&target[..idx], Some(&target[idx + 1..]))
    } else {
        (target, None)
    }
}

pub(super) fn resolve_target_path(source_file: &str, target_name: &str) -> String {
    if target_name.is_empty() {
        return source_file.to_string();
    }
    if let Some(stripped) = target_name.strip_prefix('/') {
        // Treat leading slash as repo-root-relative, not filesystem-absolute.
        // Strip any additional leading slashes (e.g. "//src/api.py").
        let stripped = stripped.trim_start_matches('/');
        return normalize_path_str(stripped);
    }
    let source_dir = Path::new(source_file)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let joined = source_dir.join(target_name);
    normalize_path_str(&joined.to_string_lossy().replace('\\', "/"))
}

pub(super) fn normalize_path_str(path: &str) -> String {
    let is_absolute = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for component in path.split('/') {
        match component {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    let joined = parts.join("/");
    if is_absolute {
        format!("/{}", joined)
    } else {
        joined
    }
}

pub(super) fn format_if_context(file: &str, label: Option<&str>, line: usize) -> String {
    match label {
        Some(lbl) => format!("{}#{}:{}", file, lbl, line),
        None => format!("{}:{}", file, line),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_target_path_self() {
        assert_eq!(resolve_target_path("src/foo.rs", ""), "src/foo.rs");
    }

    #[test]
    fn resolve_target_path_relative() {
        assert_eq!(resolve_target_path("src/foo.rs", "bar.rs"), "src/bar.rs");
    }

    #[test]
    fn normalize_path() {
        assert_eq!(normalize_path_str("src/sub/../bar.rs"), "src/bar.rs");
    }

    // BUG 3: Leading slash should be treated as repo-root-relative.
    #[test]
    fn resolve_target_path_leading_slash_is_repo_root() {
        // From sub/a.py, ThenChange("/lib/b.py") should resolve to "lib/b.py"
        assert_eq!(resolve_target_path("sub/a.py", "/lib/b.py"), "lib/b.py");
    }

    #[test]
    fn resolve_target_path_leading_slash_normalized() {
        assert_eq!(
            resolve_target_path("deep/nested/a.py", "/src/./b.py"),
            "src/b.py"
        );
    }

    #[test]
    fn format_if_context_variants() {
        assert_eq!(format_if_context("f.rs", None, 42), "f.rs:42");
        assert_eq!(format_if_context("f.rs", Some("lbl"), 42), "f.rs#lbl:42");
    }

    #[test]
    fn resolve_absolute_path_with_dotdot_clamped() {
        // /../../etc/passwd should normalize to etc/passwd (can't escape root)
        assert_eq!(
            resolve_target_path("sub/a.py", "/../../etc/passwd"),
            "etc/passwd"
        );
    }

    #[test]
    fn resolve_absolute_double_slash() {
        // "//src//api.py" should strip all leading slashes and normalize.
        assert_eq!(
            resolve_target_path("sub/a.py", "//src//api.py"),
            "src/api.py"
        );
    }

    #[test]
    fn resolve_absolute_just_slash() {
        // ThenChange(/) with empty path after slash -> resolves to source file
        // Actually, strip_prefix('/') gives "" which is empty, so it returns ""
        assert_eq!(resolve_target_path("sub/a.py", "/"), "");
    }

    #[test]
    fn resolve_absolute_with_label_splitting() {
        let (name, label) = split_target_label("/src/api.py#fields");
        assert_eq!(name, "/src/api.py");
        assert_eq!(label, Some("fields"));
        assert_eq!(resolve_target_path("any/file.py", name), "src/api.py");
    }

    #[test]
    fn find_repo_root_finds_git_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = tmp.path().join("myrepo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::create_dir_all(repo.join("src/deep")).unwrap();
        assert_eq!(find_repo_root(&repo.join("src/deep")), Some(repo.clone()));
        assert_eq!(find_repo_root(&repo), Some(repo.clone()));
    }

    #[test]
    fn find_repo_root_finds_git_file_worktree() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = tmp.path().join("myrepo");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        // Worktrees use a .git file instead of a directory
        std::fs::write(repo.join(".git"), "gitdir: /some/path").unwrap();
        assert_eq!(find_repo_root(&repo.join("src")), Some(repo.clone()));
    }

    #[test]
    fn find_repo_root_from_file_path_uses_parent_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::create_dir_all(repo.join("src")).unwrap();
        let file = repo.join("src/main.py");
        std::fs::write(&file, "x = 1\n").unwrap();
        assert_eq!(find_repo_root(&file), Some(repo));
    }

    #[test]
    fn find_repo_root_accepts_relative_start() {
        let root = find_repo_root(std::path::Path::new("."));
        assert!(
            root.is_some(),
            "expected to discover repo root from relative path start"
        );
    }

    #[test]
    fn find_repo_root_returns_none_when_no_scm() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().join("norepo/deep");
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(find_repo_root(&dir), None);
    }

    #[test]
    fn find_repo_root_finds_hg_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = tmp.path().join("hgrepo");
        std::fs::create_dir_all(repo.join(".hg")).unwrap();
        std::fs::create_dir_all(repo.join("src")).unwrap();
        assert_eq!(find_repo_root(&repo.join("src")), Some(repo.clone()));
    }

    #[test]
    fn find_repo_root_finds_jj_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = tmp.path().join("jjrepo");
        std::fs::create_dir_all(repo.join(".jj")).unwrap();
        std::fs::create_dir_all(repo.join("src")).unwrap();
        assert_eq!(find_repo_root(&repo.join("src")), Some(repo.clone()));
    }

    #[test]
    fn find_repo_root_finds_svn_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = tmp.path().join("svnrepo");
        std::fs::create_dir_all(repo.join(".svn")).unwrap();
        std::fs::create_dir_all(repo.join("src")).unwrap();
        assert_eq!(find_repo_root(&repo.join("src")), Some(repo.clone()));
    }

    #[test]
    fn find_repo_root_finds_pijul_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = tmp.path().join("pijulrepo");
        std::fs::create_dir_all(repo.join(".pijul")).unwrap();
        std::fs::create_dir_all(repo.join("src")).unwrap();
        assert_eq!(find_repo_root(&repo.join("src")), Some(repo.clone()));
    }

    #[test]
    fn find_repo_root_finds_fslckout_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = tmp.path().join("fossilrepo");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join(".fslckout"), "checkout db").unwrap();
        assert_eq!(find_repo_root(&repo.join("src")), Some(repo.clone()));
    }

    #[test]
    fn find_repo_root_finds_fossil_underscore_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = tmp.path().join("fossilrepo2");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("_FOSSIL_"), "checkout db").unwrap();
        assert_eq!(find_repo_root(&repo.join("src")), Some(repo.clone()));
    }

    #[test]
    fn find_repo_root_nearest_wins_nested() {
        let tmp = tempfile::TempDir::new().unwrap();
        let outer = tmp.path().join("outer");
        let inner = outer.join("inner");
        std::fs::create_dir_all(outer.join(".git")).unwrap();
        std::fs::create_dir_all(inner.join(".git")).unwrap();
        std::fs::create_dir_all(inner.join("src")).unwrap();
        // From inner/src, should find inner, not outer
        assert_eq!(find_repo_root(&inner.join("src")), Some(inner.clone()));
    }
}
