pub mod cli;
mod comment;
mod diff;
mod directive;
mod engine;
mod model;

pub use diff::parse_changed_lines;
pub use directive::{
    parse_directives_from_content, parse_file_directives, validate_directive_uniqueness,
    DirectiveParseError,
};
pub use engine::{find_repo_root, lint_diff};
pub use model::{Directive, FileChanges, LineRange, LintResult};
