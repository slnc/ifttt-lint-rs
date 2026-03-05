/// Inclusive range of line numbers in a target file label region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineRange {
    pub start_line: usize,
    pub end_line: usize,
}
