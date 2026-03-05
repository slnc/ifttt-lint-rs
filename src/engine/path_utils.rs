use std::path::Path;

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
    if Path::new(target_name).is_absolute() {
        return target_name.to_string();
    }
    let source_dir = Path::new(source_file)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let joined = source_dir.join(target_name);
    normalize_path_str(&joined.to_string_lossy())
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
        Some(lbl) => format!("[ifttt] {}#{}:{}", file, lbl, line),
        None => format!("[ifttt] {}:{}", file, line),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_target_path_self() {
        assert_eq!(resolve_target_path("src/foo.rs", ""), "src/foo.rs");
    }

    #[test]
    fn test_resolve_target_path_relative() {
        assert_eq!(resolve_target_path("src/foo.rs", "bar.rs"), "src/bar.rs");
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path_str("src/sub/../bar.rs"), "src/bar.rs");
    }

    #[test]
    fn test_format_if_context() {
        assert_eq!(format_if_context("f.rs", None, 42), "[ifttt] f.rs:42");
        assert_eq!(
            format_if_context("f.rs", Some("lbl"), 42),
            "[ifttt] f.rs#lbl:42"
        );
    }
}
