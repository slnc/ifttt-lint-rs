use std::sync::OnceLock;

use regex::Regex;

pub(super) struct Patterns {
    pub(super) if_change_labeled: Regex,
    pub(super) if_change_labeled_unquoted: Regex,
    pub(super) if_change_bare: Regex,
    pub(super) then_change_single: Regex,
    pub(super) then_change_array: Regex,
    pub(super) then_change_fallback: Regex,
    pub(super) label: Regex,
    pub(super) label_unquoted: Regex,
    pub(super) end_label: Regex,
    pub(super) lint_dot: Regex,
    pub(super) lint_directive_name: Regex,
}

pub(super) fn patterns() -> &'static Patterns {
    static INSTANCE: OnceLock<Patterns> = OnceLock::new();
    INSTANCE.get_or_init(|| Patterns {
        if_change_labeled: Regex::new(r#"(?i)LINT\.IfChange\s*\(\s*['\"]([^'\"]+)['\"]\s*\)"#)
            .unwrap(),
        if_change_labeled_unquoted: Regex::new(
            r#"(?i)LINT\.IfChange\s*\(\s*([^'\")\s][^)]*?)\s*\)"#,
        )
        .unwrap(),
        if_change_bare: Regex::new(r"(?i)LINT\.IfChange\b").unwrap(),
        then_change_single: Regex::new(r#"(?i)LINT\.ThenChange\s*\(\s*['\"]([^'\"]+)['\"]\s*\)"#)
            .unwrap(),
        then_change_array: Regex::new(r#"(?i)LINT\.ThenChange\s*\(\s*\[([^\]]*?)\]\s*,?\s*\)"#)
            .unwrap(),
        then_change_fallback: Regex::new(r"(?i)LINT\.ThenChange\(([^)]*)\)").unwrap(),
        label: Regex::new(r#"(?i)LINT\.Label\(\s*['\"]([^'\"]+)['\"]\s*\)"#).unwrap(),
        label_unquoted: Regex::new(r#"(?i)LINT\.Label\(\s*([^'\")\s][^)]*?)\s*\)"#).unwrap(),
        end_label: Regex::new(r"(?i)LINT\.EndLabel\b").unwrap(),
        lint_dot: Regex::new(r"(?i)^\s*LINT\.").unwrap(),
        lint_directive_name: Regex::new(r"(?i)LINT\.(\w+)").unwrap(),
    })
}
