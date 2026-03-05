use thiserror::Error;

#[derive(Debug, Error)]
pub enum DirectiveParseError {
    #[error("Failed to read {path}: {source}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "Malformed {directive} directive at {path}:{line}: expected {expected}, saw '{found}'"
    )]
    MalformedDirective {
        directive: &'static str,
        path: String,
        line: usize,
        expected: &'static str,
        found: String,
    },
    #[error("Unknown LINT directive '{name}' at {path}:{line}: '{line_text}'")]
    UnknownDirective {
        name: String,
        path: String,
        line: usize,
        line_text: String,
    },
}
