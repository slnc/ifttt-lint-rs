mod error;
mod parse;
mod patterns;
mod validate;

pub use error::DirectiveParseError;
pub use parse::{parse_directives_from_content, parse_file_directives};
pub use validate::validate_directive_uniqueness;

/// Lightweight check: does this text look like a LINT directive line?
///
/// This performs a simple case-insensitive scan for `LINT.` — it is intentionally
/// broad so that boundary-removal logic errs on the side of treating ambiguous
/// lines as directives (which avoids false-positive co-change triggers).
pub fn looks_like_directive(text: &str) -> bool {
    text.as_bytes()
        .windows(5)
        .any(|w| w.eq_ignore_ascii_case(b"LINT."))
}
