mod ignore;
mod index;
pub mod lint;
pub(crate) mod resolve;

pub use lint::lint_diff;
pub use resolve::find_repo_root;

use std::collections::HashMap;

use crate::model::LineRange;

#[derive(Debug, Clone)]
pub(super) struct Pair {
    pub(super) file: String,
    pub(super) if_line: usize,
    pub(super) if_label: Option<String>,
    pub(super) then_target: String,
    pub(super) then_target_path: String,
    pub(super) then_target_label: Option<String>,
    pub(super) then_line: usize,
}

#[derive(Debug, Clone)]
pub(super) struct FileIndex {
    pub(super) pairs: Vec<Pair>,
    pub(super) label_ranges: HashMap<String, LineRange>,
}

#[derive(Debug)]
pub(super) struct ChangedFileOutcome {
    pub(super) index: FileIndex,
    pub(super) orphan_then: Vec<(usize, String)>,
    pub(super) orphan_if: Vec<(usize, Option<String>)>,
    pub(super) uniqueness_errors: Vec<String>,
}

#[derive(Debug)]
pub(super) enum TargetLoad {
    Parsed {
        index: FileIndex,
        uniqueness_errors: Vec<String>,
    },
    MissingOrInvalid,
}
