use std::collections::HashSet;

/// Represents the added and removed line numbers for a file in a diff.
#[derive(Debug, Clone)]
pub struct FileChanges {
    pub added_lines: HashSet<usize>,
    pub removed_lines: HashSet<usize>,
}
