use std::collections::HashMap;
use std::path::Path;

use crate::directive::{parse_file_directives, validate_directive_uniqueness};
use crate::engine::path_utils::{resolve_target_path, split_target_label};
use crate::engine::types::{ChangedFileOutcome, FileIndex, Pair, TargetLoad};
use crate::model::{Directive, FileChanges, LineRange};

pub(super) fn build_changed_lines_map(
    changed_files_map: &HashMap<String, FileChanges>,
) -> HashMap<String, Vec<usize>> {
    changed_files_map
        .iter()
        .map(|(file, changes)| {
            let mut merged =
                Vec::with_capacity(changes.added_lines.len() + changes.removed_lines.len());
            merged.extend(changes.added_lines.iter().copied());
            merged.extend(changes.removed_lines.iter().copied());
            merged.sort_unstable();
            merged.dedup();
            (file.clone(), merged)
        })
        .collect()
}

pub(super) fn index_changed_file(file: &str) -> Result<ChangedFileOutcome, String> {
    let directives = parse_file_directives(file).map_err(|e| e.to_string())?;
    let uniqueness_errors = validate_directive_uniqueness(&directives, file);
    let (pairs, orphan_then, orphan_if) = build_pairs(file, &directives);

    Ok(ChangedFileOutcome {
        index: FileIndex {
            pairs,
            if_blocks: get_if_change_blocks(&directives),
            label_ranges: build_label_ranges(&directives),
            has_if_blocks: directives
                .iter()
                .any(|d| matches!(d, Directive::IfChange { .. })),
        },
        orphan_then,
        orphan_if,
        uniqueness_errors,
    })
}

pub(super) fn index_target_file(file: &str) -> TargetLoad {
    if !Path::new(file).exists() {
        return TargetLoad::MissingOrInvalid;
    }

    let directives = match parse_file_directives(file) {
        Ok(ds) => ds,
        Err(_) => return TargetLoad::MissingOrInvalid,
    };

    let uniqueness_errors = validate_directive_uniqueness(&directives, file);
    TargetLoad::Parsed {
        index: FileIndex {
            pairs: Vec::new(),
            if_blocks: get_if_change_blocks(&directives),
            label_ranges: build_label_ranges(&directives),
            has_if_blocks: directives
                .iter()
                .any(|d| matches!(d, Directive::IfChange { .. })),
        },
        uniqueness_errors,
    }
}

fn build_pairs(
    file: &str,
    directives: &[Directive],
) -> (
    Vec<Pair>,
    Vec<(usize, String)>,
    Vec<(usize, Option<String>)>,
) {
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

fn get_if_change_blocks(directives: &[Directive]) -> Vec<(usize, usize)> {
    let mut blocks = Vec::new();
    let mut current_if: Option<usize> = None;

    for d in directives {
        match d {
            Directive::IfChange { line, .. } => {
                current_if = Some(*line);
            }
            Directive::ThenChange { line, .. } => {
                if let Some(if_line) = current_if.take() {
                    blocks.push((if_line, *line));
                }
            }
            _ => {}
        }
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_build_changed_lines_map_dedups() {
        let mut map = HashMap::new();
        let mut fc = FileChanges {
            added_lines: HashSet::new(),
            removed_lines: HashSet::new(),
        };
        fc.added_lines.insert(3);
        fc.added_lines.insert(2);
        fc.removed_lines.insert(3);
        map.insert("a.ts".to_string(), fc);

        let merged = build_changed_lines_map(&map);
        assert_eq!(merged.get("a.ts").cloned().unwrap_or_default(), vec![2, 3]);
    }
}
