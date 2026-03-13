use std::collections::HashMap;
use std::path::Path;

use super::{ChangedFileOutcome, FileIndex, Pair, TargetLoad};
use crate::directive::{
    looks_like_directive, parse_file_directives, validate_directive_uniqueness,
};
use crate::engine::resolve::{resolve_target_path, split_target_label};
use crate::model::{Directive, FileChanges, LineRange};

/// Resolve a repo-relative path to a filesystem path for reading.
fn fs_path(repo_root: &Path, relative: &str) -> String {
    repo_root.join(relative).to_string_lossy().to_string()
}

/// Per-file changed line data using new-file coordinates only.
#[derive(Debug)]
pub(super) struct ChangedLines {
    /// New-file lines where additions occurred (exact positions).
    pub(super) addition_lines: Vec<usize>,
    /// New-file lines where removals occurred (gap positions), sorted and deduped.
    pub(super) removal_lines: Vec<usize>,
    /// Ordered removed-line contents grouped by new-file gap position.
    /// Each entry maps a gap position to the list of removed line texts in
    /// old-file order.  Combined with a directive-detection function this lets
    /// boundary checks determine which side of a directive rewrite a deletion
    /// came from (before the block vs inside the block).
    pub(super) removal_line_details: HashMap<usize, Vec<String>>,
    /// Union of addition_lines and removal_lines, sorted and deduped.
    pub(super) all_lines: Vec<usize>,
}

impl ChangedLines {
    /// How many raw removal lines map to the given gap position.
    pub(super) fn removal_count_at(&self, pos: usize) -> usize {
        self.removal_line_details.get(&pos).map_or(0, |v| v.len())
    }

    /// Were any non-directive lines removed on the **inside** of the block
    /// at the `if_line` boundary?
    ///
    /// Removals at `if_line` are in old-file order.  The IfChange directive
    /// is among them.  Lines removed *after* the last directive in the
    /// sequence were below the IfChange in the old file — i.e. inside the
    /// block.  Lines before it were above (outside the block).
    pub(super) fn has_content_removal_after_directive(&self, pos: usize) -> bool {
        let details = match self.removal_line_details.get(&pos) {
            Some(d) => d,
            None => return false,
        };
        // Find the last directive in the sequence.
        let last_directive_idx = details.iter().rposition(|text| looks_like_directive(text));
        match last_directive_idx {
            Some(idx) => {
                // Any non-directive line after the last directive is content
                // from inside the block.
                details[idx + 1..]
                    .iter()
                    .any(|text| !looks_like_directive(text))
            }
            None => {
                // No directive found among removals — all are content.
                // This shouldn't happen when a directive is being rewritten,
                // but treat it conservatively as content.
                !details.is_empty()
            }
        }
    }

    /// Were any non-directive lines removed on the **inside** of the block
    /// at the `then_line` boundary?
    ///
    /// Symmetric to `has_content_removal_after_directive` but for the bottom
    /// of the block.  Content inside the block is *above* the ThenChange
    /// directive, so we look for non-directive removals *before* the first
    /// directive in the sequence.
    pub(super) fn has_content_removal_before_directive(&self, pos: usize) -> bool {
        let details = match self.removal_line_details.get(&pos) {
            Some(d) => d,
            None => return false,
        };
        let first_directive_idx = details.iter().position(|text| looks_like_directive(text));
        match first_directive_idx {
            Some(idx) => {
                // Any non-directive line before the first directive is content
                // from inside the block.
                details[..idx]
                    .iter()
                    .any(|text| !looks_like_directive(text))
            }
            None => {
                // No directive found — all are content.
                !details.is_empty()
            }
        }
    }
}

pub(super) fn build_changed_lines_map(
    changed_files_map: &HashMap<String, FileChanges>,
) -> HashMap<String, ChangedLines> {
    changed_files_map
        .iter()
        .map(|(file, changes)| {
            let mut addition_lines: Vec<usize> =
                changes.addition_new_lines.iter().copied().collect();
            addition_lines.sort_unstable();
            addition_lines.dedup();

            let mut removal_line_details: HashMap<usize, Vec<String>> = HashMap::new();
            for (line, content) in &changes.removal_new_lines {
                removal_line_details
                    .entry(*line)
                    .or_default()
                    .push(content.clone());
            }

            let mut removal_lines: Vec<usize> = removal_line_details.keys().copied().collect();
            removal_lines.sort_unstable();

            let mut all_lines = Vec::with_capacity(addition_lines.len() + removal_lines.len());
            all_lines.extend(&addition_lines);
            all_lines.extend(&removal_lines);
            all_lines.sort_unstable();
            all_lines.dedup();

            (
                file.clone(),
                ChangedLines {
                    addition_lines,
                    removal_lines,
                    removal_line_details,
                    all_lines,
                },
            )
        })
        .collect()
}

pub(super) fn index_changed_file(
    repo_root: &Path,
    file: &str,
) -> Result<ChangedFileOutcome, String> {
    let directives = parse_file_directives(&fs_path(repo_root, file)).map_err(|e| e.to_string())?;
    let uniqueness_errors = validate_directive_uniqueness(&directives, file);
    let (pairs, orphan_then, orphan_if) = build_pairs(file, &directives);

    Ok(ChangedFileOutcome {
        index: FileIndex {
            pairs,
            label_ranges: build_label_ranges(&directives),
        },
        orphan_then,
        orphan_if,
        uniqueness_errors,
    })
}

pub(super) fn index_target_file(repo_root: &Path, file: &str) -> TargetLoad {
    let full_path = fs_path(repo_root, file);
    if !Path::new(&full_path).exists() {
        return TargetLoad::MissingOrInvalid;
    }

    let directives = match parse_file_directives(&full_path) {
        Ok(ds) => ds,
        Err(_) => return TargetLoad::MissingOrInvalid,
    };

    let uniqueness_errors = validate_directive_uniqueness(&directives, file);
    TargetLoad::Parsed {
        index: FileIndex {
            pairs: Vec::new(),
            label_ranges: build_label_ranges(&directives),
        },
        uniqueness_errors,
    }
}

type OrphanThen = Vec<(usize, String)>;
type OrphanIf = Vec<(usize, Option<String>)>;

fn build_pairs(file: &str, directives: &[Directive]) -> (Vec<Pair>, OrphanThen, OrphanIf) {
    let mut pairs = Vec::new();
    let mut orphan_then = Vec::new();
    let mut orphan_if = Vec::new();

    let mut current_if: Option<(usize, Option<String>)> = None;
    let mut saw_then = false;

    for d in directives {
        match d {
            Directive::IfChange { line, label } => {
                if let Some((prev_line, prev_label)) = current_if.take() {
                    if !saw_then {
                        orphan_if.push((prev_line, prev_label));
                    }
                }
                current_if = Some((*line, label.clone()));
                saw_then = false;
            }
            Directive::ThenChange { line, target } => {
                if let Some((if_line, ref if_label)) = current_if {
                    let (target_name, target_label) = split_target_label(target);
                    let then_target_path = resolve_target_path(file, target_name);
                    pairs.push(Pair {
                        file: file.to_string(),
                        if_line,
                        if_label: if_label.clone(),
                        then_target: target.clone(),
                        then_target_path,
                        then_target_label: target_label.map(str::to_string),
                        then_line: *line,
                    });
                    saw_then = true;
                } else {
                    orphan_then.push((*line, target.clone()));
                }
            }
            _ => {}
        }
    }

    if let Some((prev_line, prev_label)) = current_if.take() {
        if !saw_then {
            orphan_if.push((prev_line, prev_label));
        }
    }

    (pairs, orphan_then, orphan_if)
}

fn build_label_ranges(directives: &[Directive]) -> HashMap<String, LineRange> {
    let mut ranges = HashMap::new();
    let mut pending: Vec<(String, usize)> = Vec::new();
    let mut active_if_label: Option<(String, usize)> = None;

    for d in directives {
        match d {
            Directive::Label { line, name } => {
                pending.push((name.clone(), line + 1));
            }
            Directive::EndLabel { line } => {
                if let Some((name, start)) = pending.pop() {
                    ranges.insert(
                        name,
                        LineRange {
                            start_line: start,
                            end_line: line.saturating_sub(1),
                        },
                    );
                }
            }
            Directive::IfChange { line, label } => {
                if let Some((name, start)) = active_if_label.take() {
                    ranges.entry(name).or_insert_with(|| {
                        let end = start.max(line.saturating_sub(1));
                        LineRange {
                            start_line: start,
                            end_line: end,
                        }
                    });
                }
                active_if_label = label.as_ref().map(|l| (l.clone(), line + 1));
            }
            Directive::ThenChange { line, .. } => {
                if let Some((name, start)) = active_if_label.take() {
                    ranges.entry(name).or_insert_with(|| {
                        let end = start.max(line.saturating_sub(1));
                        LineRange {
                            start_line: start,
                            end_line: end,
                        }
                    });
                }
            }
        }
    }

    if let Some((name, start)) = active_if_label.take() {
        ranges.entry(name).or_insert(LineRange {
            start_line: start,
            end_line: start,
        });
    }

    ranges
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn build_changed_lines_map_dedups() {
        let map = HashMap::from([(
            "a.ts".to_string(),
            FileChanges {
                added_lines: HashSet::from([2, 3]),
                removed_lines: HashSet::from([3]),
                addition_new_lines: HashSet::from([2, 3]),
                removal_new_lines: vec![(2, "old content".to_string())],
            },
        )]);
        let result = build_changed_lines_map(&map);
        assert_eq!(result["a.ts"].all_lines, vec![2, 3]);
        assert_eq!(result["a.ts"].addition_lines, vec![2, 3]);
        assert_eq!(result["a.ts"].removal_lines, vec![2]);
    }

    #[test]
    fn has_content_removal_after_directive_basic() {
        let map = HashMap::from([(
            "a.py".to_string(),
            FileChanges {
                added_lines: HashSet::from([1]),
                removed_lines: HashSet::from([1, 2]),
                addition_new_lines: HashSet::from([1]),
                removal_new_lines: vec![
                    (1, "# LINT.IfChange".to_string()),
                    (1, "inside content".to_string()),
                ],
            },
        )]);
        let result = build_changed_lines_map(&map);
        let cl = &result["a.py"];
        // Content after the directive -> inside block
        assert!(cl.has_content_removal_after_directive(1));
        // No content before the directive -> not inside block (for then_line check)
        assert!(!cl.has_content_removal_before_directive(1));
    }

    #[test]
    fn has_content_removal_before_directive_basic() {
        let map = HashMap::from([(
            "a.py".to_string(),
            FileChanges {
                added_lines: HashSet::from([3]),
                removed_lines: HashSet::from([2, 3]),
                addition_new_lines: HashSet::from([3]),
                removal_new_lines: vec![
                    (3, "inside content".to_string()),
                    (3, "# LINT.ThenChange(\"old.py\")".to_string()),
                ],
            },
        )]);
        let result = build_changed_lines_map(&map);
        let cl = &result["a.py"];
        // Content before the directive -> inside block
        assert!(cl.has_content_removal_before_directive(3));
        // No content after the directive -> not inside block (for if_line check)
        assert!(!cl.has_content_removal_after_directive(3));
    }

    #[test]
    fn outside_deletion_at_if_boundary_not_detected_as_inside() {
        // "-before / -# LINT.IfChange / +# LINT.IfChange(x)"
        // The "before" line is outside the block, so has_content_removal_after_directive
        // should return false (content is before the directive, not after).
        let map = HashMap::from([(
            "a.py".to_string(),
            FileChanges {
                added_lines: HashSet::from([1]),
                removed_lines: HashSet::from([1, 2]),
                addition_new_lines: HashSet::from([1]),
                removal_new_lines: vec![
                    (1, "before".to_string()),
                    (1, "# LINT.IfChange".to_string()),
                ],
            },
        )]);
        let result = build_changed_lines_map(&map);
        let cl = &result["a.py"];
        assert!(
            !cl.has_content_removal_after_directive(1),
            "deletion before IfChange should not be detected as inside-block content"
        );
    }

    #[test]
    fn outside_deletion_at_then_boundary_not_detected_as_inside() {
        // "-# LINT.ThenChange / -after / +# LINT.ThenChange(new)"
        // The "after" line is outside the block, so has_content_removal_before_directive
        // should return false (content is after the directive, not before).
        let map = HashMap::from([(
            "a.py".to_string(),
            FileChanges {
                added_lines: HashSet::from([3]),
                removed_lines: HashSet::from([3, 4]),
                addition_new_lines: HashSet::from([3]),
                removal_new_lines: vec![
                    (3, "# LINT.ThenChange(\"old.py\")".to_string()),
                    (3, "after".to_string()),
                ],
            },
        )]);
        let result = build_changed_lines_map(&map);
        let cl = &result["a.py"];
        assert!(
            !cl.has_content_removal_before_directive(3),
            "deletion after ThenChange should not be detected as inside-block content"
        );
    }
}
