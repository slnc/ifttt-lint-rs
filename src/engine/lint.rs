use std::collections::{HashMap, HashSet};
use std::path::Path;

use rayon::prelude::*;

use crate::diff::parse_changed_lines;
use crate::engine::ignore::{
    parse_ignore_list, should_ignore_file, should_ignore_if_label, should_ignore_target,
};
use crate::engine::index::{build_changed_lines_map, index_changed_file, index_target_file};
use crate::engine::path_utils::format_if_context;
use crate::engine::types::{FileIndex, Pair, TargetLoad};
use crate::model::LintResult;

pub fn lint_diff(diff_text: &str, verbose: bool, ignore_list: &[String]) -> LintResult {
    let ignore_patterns = parse_ignore_list(ignore_list);
    let mut messages: Vec<String> = Vec::new();
    let mut error_count: usize = 0;

    let changed_files_map = parse_changed_lines(diff_text);
    let changed_lines_map = build_changed_lines_map(&changed_files_map);

    let changed_files: Vec<String> = changed_files_map
        .keys()
        .filter(|f| !should_ignore_file(f, &ignore_patterns))
        .cloned()
        .collect();

    if verbose {
        for f in &changed_files {
            eprintln!("Processing changed file: {}", f);
        }
    }

    let phase1: Vec<(String, Result<_, String>)> = changed_files
        .par_iter()
        .map(|file| (file.clone(), index_changed_file(file)))
        .collect();

    let mut file_indices: HashMap<String, FileIndex> = HashMap::new();
    let mut pairs: Vec<Pair> = Vec::new();
    let mut orphan_then: Vec<(String, usize, String)> = Vec::new();
    let mut orphan_if: Vec<(String, usize, Option<String>)> = Vec::new();

    for (file, result) in phase1 {
        match result {
            Ok(outcome) => {
                for err in outcome.uniqueness_errors {
                    messages.push(err);
                    error_count += 1;
                }

                pairs.extend(outcome.index.pairs.iter().cloned());
                orphan_then.extend(
                    outcome
                        .orphan_then
                        .into_iter()
                        .map(|(line, target)| (file.clone(), line, target)),
                );
                orphan_if.extend(
                    outcome
                        .orphan_if
                        .into_iter()
                        .map(|(line, label)| (file.clone(), line, label)),
                );
                file_indices.insert(file.clone(), outcome.index);
            }
            Err(e) => {
                messages.push(e);
                error_count += 1;
            }
        }

        if verbose {
            eprintln!("Finished processing changed file: {}", file);
        }
    }

    for (file, line, target) in &orphan_then {
        if should_ignore_target(target, &ignore_patterns) {
            if verbose {
                eprintln!(
                    "[ifttt] Ignoring orphan ThenChange '{}' in {}:{}",
                    target, file, line
                );
            }
            continue;
        }
        messages.push(format!(
            "[ifttt] {}:{} -> unexpected ThenChange '{}' without preceding IfChange",
            file, line, target
        ));
        error_count += 1;
    }

    for (file, line, label) in &orphan_if {
        if let Some(lbl) = label {
            if should_ignore_if_label(file, lbl, &ignore_patterns) {
                if verbose {
                    eprintln!(
                        "[ifttt] Ignoring orphan IfChange '{}' in {}:{}",
                        lbl, file, line
                    );
                }
                continue;
            }
        }
        let lbl_str = match label {
            Some(l) => format!("('{}')", l),
            None => String::new(),
        };
        messages.push(format!(
            "[ifttt] {}:{} -> missing ThenChange after IfChange{}",
            file, line, lbl_str
        ));
        error_count += 1;
    }

    let target_files: HashSet<String> = pairs.iter().map(|p| p.then_target_path.clone()).collect();
    let uncached_targets: Vec<String> = target_files
        .iter()
        .filter(|path| !file_indices.contains_key(path.as_str()))
        .cloned()
        .collect();

    let phase2: Vec<(String, TargetLoad)> = uncached_targets
        .par_iter()
        .map(|file| (file.clone(), index_target_file(file)))
        .collect();

    let mut unavailable_targets: HashSet<String> = HashSet::new();

    for (file, load) in phase2 {
        match load {
            TargetLoad::Parsed {
                index,
                uniqueness_errors,
            } => {
                for err in uniqueness_errors {
                    messages.push(err);
                    error_count += 1;
                }
                file_indices.insert(file, index);
            }
            TargetLoad::MissingOrInvalid => {
                unavailable_targets.insert(file.clone());
                for p in &pairs {
                    if p.then_target_path != file {
                        continue;
                    }
                    if should_ignore_target(&p.then_target, &ignore_patterns) {
                        continue;
                    }
                    if let Some(ref lbl) = p.if_label {
                        if should_ignore_if_label(&p.file, lbl, &ignore_patterns) {
                            continue;
                        }
                    }
                    let if_ctx = format_if_context(&p.file, p.if_label.as_deref(), p.if_line);
                    messages.push(format!(
                        "{} -> ThenChange '{}' (line {}): target file '{}' not found.",
                        if_ctx, p.then_target, p.then_line, file
                    ));
                    error_count += 1;
                }
            }
        }
    }

    for p in &pairs {
        if let Some(ref lbl) = p.if_label {
            if should_ignore_if_label(&p.file, lbl, &ignore_patterns) {
                continue;
            }
        }
        if should_ignore_target(&p.then_target, &ignore_patterns) {
            continue;
        }
        if unavailable_targets.contains(&p.then_target_path) {
            continue;
        }

        let source_index = match file_indices.get(&p.file) {
            Some(index) => index,
            None => continue,
        };

        let source_changed = match changed_lines_map.get(&p.file) {
            Some(lines) => lines,
            None => continue,
        };

        let target_has_if_blocks = file_indices
            .get(&p.then_target_path)
            .map(|idx| idx.has_if_blocks)
            .unwrap_or(false);

        let triggered = if target_has_if_blocks {
            source_changed.iter().any(|&line| {
                source_index
                    .if_blocks
                    .iter()
                    .any(|&(start, end)| line >= start && line <= end)
            })
        } else {
            source_changed
                .iter()
                .any(|&line| line >= p.if_line && line <= p.then_line)
        };

        if !triggered {
            continue;
        }

        let target_changes = changed_lines_map.get(&p.then_target_path);
        let if_ctx = format_if_context(&p.file, p.if_label.as_deref(), p.if_line);

        if target_changes.is_none() {
            if !Path::new(&p.then_target_path).exists() {
                continue;
            }
            messages.push(format!(
                "{} -> ThenChange '{}' (line {}): target file '{}' not changed.",
                if_ctx, p.then_target, p.then_line, p.then_target_path
            ));
            error_count += 1;
            continue;
        }

        let target_changes = target_changes.expect("checked is_some above");

        if let Some(label) = p.then_target_label.as_deref() {
            let target_index = match file_indices.get(&p.then_target_path) {
                Some(idx) => idx,
                None => continue,
            };
            let available = if target_index.label_ranges.is_empty() {
                "none".to_string()
            } else {
                target_index
                    .label_ranges
                    .keys()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            match target_index.label_ranges.get(label) {
                None => {
                    messages.push(format!(
                        "{} -> ThenChange '{}' (line {}): label '{}' not found in '{}'. Available labels: {}",
                        if_ctx, p.then_target, p.then_line, label, p.then_target_path, available
                    ));
                    error_count += 1;
                }
                Some(lr) => {
                    let in_range = target_changes
                        .iter()
                        .any(|&line| line >= lr.start_line && line <= lr.end_line);
                    if !in_range {
                        let sorted = target_changes
                            .iter()
                            .map(|l| l.to_string())
                            .collect::<Vec<_>>()
                            .join(", ");
                        messages.push(format!(
                            "{} -> ThenChange '{}' (line {}): expected changes in '{}#{}' ({}-{}), but none found. Actual changes in file: [{}]",
                            if_ctx,
                            p.then_target,
                            p.then_line,
                            p.then_target_path,
                            label,
                            lr.start_line,
                            lr.end_line,
                            sorted
                        ));
                        error_count += 1;
                    }
                }
            }
        } else if target_changes.is_empty() {
            messages.push(format!(
                "{} -> ThenChange '{}' (line {}): expected changes in '{}', but none found.",
                if_ctx, p.then_target, p.then_line, p.then_target_path
            ));
            error_count += 1;
        }
    }

    LintResult {
        exit_code: if error_count > 0 { 1 } else { 0 },
        messages,
    }
}
