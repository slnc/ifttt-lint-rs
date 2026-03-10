use std::collections::HashMap;
use std::path::Path;

use super::{ChangedFileOutcome, FileIndex, Pair, TargetLoad};
use crate::directive::{parse_file_directives, validate_directive_uniqueness};
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
    /// New-file lines where removals occurred (gap positions).
    pub(super) removal_lines: Vec<usize>,
    /// Union of addition_lines and removal_lines, sorted and deduped.
    pub(super) all_lines: Vec<usize>,
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

            let mut removal_lines: Vec<usize> = changes.removal_new_lines.iter().copied().collect();
            removal_lines.sort_unstable();
            removal_lines.dedup();

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
                removal_new_lines: HashSet::from([2]),
            },
        )]);
        let result = build_changed_lines_map(&map);
        assert_eq!(result["a.ts"].all_lines, vec![2, 3]);
        assert_eq!(result["a.ts"].addition_lines, vec![2, 3]);
        assert_eq!(result["a.ts"].removal_lines, vec![2]);
    }
}
