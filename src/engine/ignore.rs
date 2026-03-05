use std::path::Path;

use globset::{Glob, GlobMatcher};

use crate::engine::resolve::split_target_label;

#[derive(Debug, Clone)]
enum NameMatcher {
    Glob(GlobMatcher),
    Exact(String),
}

impl NameMatcher {
    fn from_glob(pattern: &str) -> Self {
        match Glob::new(pattern) {
            Ok(glob) => Self::Glob(glob.compile_matcher()),
            Err(_) => Self::Exact(pattern.to_string()),
        }
    }

    fn is_match(&self, text: &str) -> bool {
        match self {
            Self::Glob(m) => m.is_match(text),
            Self::Exact(s) => s == text,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct IgnorePattern {
    matcher: NameMatcher,
    label: Option<String>,
}

pub(super) fn parse_ignore_list(ignore_list: &[String]) -> Vec<IgnorePattern> {
    ignore_list
        .iter()
        .map(|entry| {
            if let Some(idx) = entry.find('#') {
                IgnorePattern {
                    matcher: NameMatcher::from_glob(&entry[..idx]),
                    label: Some(entry[idx + 1..].to_string()),
                }
            } else {
                IgnorePattern {
                    matcher: NameMatcher::from_glob(entry),
                    label: None,
                }
            }
        })
        .collect()
}

pub(super) fn should_ignore_target(target: &str, patterns: &[IgnorePattern]) -> bool {
    let (target_name, target_label) = split_target_label(target);
    patterns.iter().any(|p| {
        p.matcher.is_match(target_name)
            && match (&p.label, target_label) {
                (None, _) => true,
                (Some(pl), Some(tl)) => pl == tl,
                (Some(_), None) => true,
            }
    })
}

pub(super) fn should_ignore_file(file: &str, patterns: &[IgnorePattern]) -> bool {
    let basename = Path::new(file)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    patterns
        .iter()
        .any(|p| p.label.is_none() && (p.matcher.is_match(&basename) || p.matcher.is_match(file)))
}

pub(super) fn should_ignore_if_label(file: &str, label: &str, patterns: &[IgnorePattern]) -> bool {
    let basename = Path::new(file)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    patterns.iter().any(|p| {
        p.label.as_deref() == Some(label)
            && (p.matcher.is_match(&basename) || p.matcher.is_match(file))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_star() {
        let m = NameMatcher::from_glob("*.rs");
        assert!(m.is_match("foo.rs"));
        assert!(!m.is_match("foo.ts"));
    }

    #[test]
    fn glob_question() {
        let m = NameMatcher::from_glob("?.rs");
        assert!(m.is_match("a.rs"));
        assert!(!m.is_match("ab.rs"));
    }

    #[test]
    fn parse_ignore_list_with_labels() {
        let patterns = parse_ignore_list(&["foo.rs".into(), "bar.rs#my_label".into()]);
        assert_eq!(patterns.len(), 2);
        assert!(patterns[0].matcher.is_match("foo.rs"));
        assert!(patterns[0].label.is_none());
        assert!(patterns[1].matcher.is_match("bar.rs"));
        assert_eq!(patterns[1].label.as_deref(), Some("my_label"));
    }
}
