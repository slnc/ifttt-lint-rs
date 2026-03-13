use std::collections::HashSet;

/// The kinds of lint directives supported in source files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Directive {
    IfChange { line: usize, label: Option<String> },
    ThenChange { line: usize, target: String },
    Label { line: usize, name: String },
    EndLabel { line: usize },
}

/// Represents the added and removed line numbers for a file in a diff.
///
/// A unified diff uses two coordinate systems: old-file line numbers for
/// removed lines and new-file line numbers for added lines.  Directive
/// positions (`if_line`, `then_line`) come from parsing the current file on
/// disk, so they are in new-file coordinates.  To compare removals against
/// directive ranges we also need removals in new-file coordinates, that's
/// what `removal_new_lines` provides.
///
/// When the diff parser encounters a `-` line it records the current
/// `new_line` counter (which does not advance for removals).  This value
/// represents the position of the next surviving line in the new file, the
/// "gap" where the deletion occurred.  Consecutive removals all map to the
/// same `new_line`.  A removal immediately before an IfChange block maps to
/// `new_line == if_line`, so the trigger check must use `> if_line` (strict)
/// for removals to avoid false positives.  See `engine/lint.rs`.
#[derive(Debug, Clone)]
pub struct FileChanges {
    pub added_lines: HashSet<usize>,
    pub removed_lines: HashSet<usize>,
    /// New-file line numbers where additions occurred (same as added_lines).
    pub addition_new_lines: HashSet<usize>,
    /// New-file line numbers and content for removals.  See struct-level docs.
    /// Stored as a `Vec` of `(new_line, removed_text)` in hunk order so that
    /// consecutive removals mapping to the same new-line position are preserved
    /// as separate, ordered entries.  The ordering lets boundary checks
    /// determine which side of a directive rewrite a deletion came from.
    pub removal_new_lines: Vec<(usize, String)>,
}

/// Inclusive range of line numbers in a target file's labeled section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineRange {
    pub start_line: usize,
    pub end_line: usize,
}

/// Result of linting: collected errors and output messages.
pub struct LintResult {
    pub exit_code: i32,
    pub messages: Vec<String>,
    pub verbose_messages: Vec<String>,
    pub pairs_checked: usize,
    pub files_checked: usize,
}
