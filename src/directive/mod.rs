mod error;
mod parse;
mod patterns;
mod validate;

pub use error::DirectiveParseError;
pub use parse::{parse_directives_from_content, parse_file_directives};
pub use validate::validate_directive_uniqueness;

/// Lightweight check: does this text look like a LINT directive line?
///
/// Scans for `LINT.` followed by a known directive keyword (case-insensitive).
/// This is tighter than a bare `LINT.` check — a line like
/// `"see LINT.ThenChange docs"` in prose would still match, but
/// `"see LINT. for details"` or `"LINT.Frobulate"` would not.
pub fn looks_like_directive(text: &str) -> bool {
    let bytes = text.as_bytes();
    for window_start in 0..bytes.len().saturating_sub(4) {
        if bytes[window_start..window_start + 5].eq_ignore_ascii_case(b"LINT.") {
            let rest = &bytes[window_start + 5..];
            if rest.len() >= 8 && rest[..8].eq_ignore_ascii_case(b"IfChange") {
                return true;
            }
            if rest.len() >= 10 && rest[..10].eq_ignore_ascii_case(b"ThenChange") {
                return true;
            }
            if rest.len() >= 8 && rest[..8].eq_ignore_ascii_case(b"EndLabel") {
                return true;
            }
            if rest.len() >= 5 && rest[..5].eq_ignore_ascii_case(b"Label") {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_like_directive_matches_real_directives() {
        assert!(looks_like_directive("// LINT.IfChange"));
        assert!(looks_like_directive("# LINT.ThenChange(\"foo.py\")"));
        assert!(looks_like_directive("// LINT.Label(\"x\")"));
        assert!(looks_like_directive("// LINT.EndLabel"));
        // Case insensitive
        assert!(looks_like_directive("// lint.ifchange"));
        assert!(looks_like_directive("// Lint.ThenChange(\"x\")"));
    }

    #[test]
    fn looks_like_directive_rejects_bare_lint_dot() {
        // Content that merely mentions "LINT." without a directive keyword
        // should NOT be treated as a directive.
        assert!(!looks_like_directive("// see LINT. for details"));
        assert!(!looks_like_directive("LINT."));
        assert!(!looks_like_directive("LINT.Frobulate"));
        assert!(!looks_like_directive("refer to LINT. docs"));
    }

    #[test]
    fn looks_like_directive_rejects_content_mentioning_lint_dot() {
        // Regression: removed content lines that mention "LINT." without a
        // real directive keyword must not be classified as directives.
        assert!(!looks_like_directive("// configure LINT. settings here"));
        assert!(!looks_like_directive("# LINT. is our linting framework"));
        assert!(!looks_like_directive("LINT.Unknown(\"x\")"));
        assert!(!looks_like_directive("see LINT. docs for usage"));
        assert!(!looks_like_directive("the LINT.Check tool"));
    }
}
