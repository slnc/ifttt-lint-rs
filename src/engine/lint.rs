use std::collections::{HashMap, HashSet};
use std::path::Path;

use rayon::prelude::*;

use super::{FileIndex, Pair, TargetLoad};
use crate::diff::parse_changed_lines;
use crate::engine::ignore::{
    parse_ignore_list, should_ignore_file, should_ignore_if_label, should_ignore_target,
};
use crate::engine::index::{build_changed_lines_map, index_changed_file, index_target_file};
use crate::engine::resolve::format_if_context;
use crate::model::LintResult;

pub fn lint_diff(
    diff_text: &str,
    verbose: bool,
    debug: bool,
    ignore_list: &[String],
    repo_root: &Path,
) -> LintResult {
    let ignore_patterns = parse_ignore_list(ignore_list);
    let mut messages: Vec<String> = Vec::new();
    let mut verbose_messages: Vec<String> = Vec::new();
    let mut error_count: usize = 0;
    let mut pairs_checked: usize = 0;

    let changed_files_map = parse_changed_lines(diff_text);
    let changed_lines_map = build_changed_lines_map(&changed_files_map);

    let changed_files: Vec<String> = changed_files_map
        .keys()
        .filter(|f| !should_ignore_file(f, &ignore_patterns))
        .cloned()
        .collect();

    if debug {
        for f in &changed_files {
            eprintln!("Linting: {}", f);
        }
    }

    let phase1: Vec<(String, Result<_, String>)> = changed_files
        .par_iter()
        .map(|file| (file.clone(), index_changed_file(repo_root, file)))
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

                let FileIndex {
                    pairs: file_pairs,
                    label_ranges,
                } = outcome.index;
                pairs.extend(file_pairs);
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
                file_indices.insert(
                    file.clone(),
                    FileIndex {
                        pairs: Vec::new(),
                        label_ranges,
                    },
                );
            }
            Err(e) => {
                messages.push(e);
                error_count += 1;
            }
        }
    }

    for (file, line, target) in &orphan_then {
        if should_ignore_target(target, &ignore_patterns) {
            if verbose {
                eprintln!(
                    "Ignoring orphan ThenChange '{}' in {}:{}",
                    target, file, line
                );
            }
            continue;
        }
        messages.push(format!(
            "error: {}:{}: unexpected ThenChange '{}' without preceding IfChange",
            file, line, target
        ));
        error_count += 1;
    }

    for (file, line, label) in &orphan_if {
        if let Some(lbl) = label {
            if should_ignore_if_label(file, lbl, &ignore_patterns) {
                if verbose {
                    eprintln!("Ignoring orphan IfChange '{}' in {}:{}", lbl, file, line);
                }
                continue;
            }
        }
        let lbl_str = match label {
            Some(l) => format!("('{}')", l),
            None => String::new(),
        };
        messages.push(format!(
            "error: {}:{}: missing ThenChange after IfChange{}",
            file, line, lbl_str
        ));
        error_count += 1;
    }

    let target_files: HashSet<&str> = pairs.iter().map(|p| p.then_target_path.as_str()).collect();
    let uncached_targets: Vec<&str> = target_files
        .iter()
        .filter(|path| !file_indices.contains_key(**path))
        .copied()
        .collect();

    let phase2: Vec<(String, TargetLoad)> = uncached_targets
        .par_iter()
        .map(|file| (file.to_string(), index_target_file(repo_root, file)))
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
                    let kind = if p.then_target_label.is_some() {
                        "section"
                    } else {
                        "file"
                    };
                    messages.push(format!(
                        "error: {} -> {}: target {} unchanged",
                        if_ctx, p.then_target, kind
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

        let source_changed = match changed_lines_map.get(&p.file) {
            Some(cl) => cl,
            None => continue,
        };

        // Added lines use inclusive range (they have exact new-file positions).
        // Removal-only lines use a tighter range (> if_line) because a removal
        // that maps to if_line was actually before the block, not inside it.
        let triggered = source_changed
            .addition_lines
            .iter()
            .any(|&line| line >= p.if_line && line <= p.then_line)
            || source_changed
                .removal_lines
                .iter()
                .any(|&line| line > p.if_line && line <= p.then_line);

        if !triggered {
            continue;
        }

        pairs_checked += 1;

        if verbose {
            verbose_messages.push(format!(
                "  {}:{} IfChange -> ThenChange({})",
                p.file, p.if_line, p.then_target
            ));
        }

        let target_cl = changed_lines_map.get(&p.then_target_path);
        let if_ctx = format_if_context(&p.file, p.if_label.as_deref(), p.if_line);

        if target_cl.is_none() {
            if !repo_root.join(&p.then_target_path).exists() {
                continue;
            }
            messages.push(format!(
                "error: {} -> {}: target file unchanged",
                if_ctx, p.then_target
            ));
            error_count += 1;
            continue;
        }

        let target_cl = target_cl.expect("checked is_some above");

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
                    .map(|k| format!("'{}'", k))
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            match target_index.label_ranges.get(label) {
                None => {
                    messages.push(format!(
                        "error: {} -> {}: label '{}' not found (available: {})",
                        if_ctx, p.then_target, label, available
                    ));
                    error_count += 1;
                }
                Some(lr) => {
                    let in_range = target_cl
                        .all_lines
                        .iter()
                        .any(|&line| line >= lr.start_line && line <= lr.end_line);
                    if !in_range {
                        let range = if lr.start_line == lr.end_line {
                            format!(":{}", lr.start_line)
                        } else {
                            format!(":{}..{}", lr.start_line, lr.end_line)
                        };
                        messages.push(format!(
                            "error: {} -> {}{} unchanged",
                            if_ctx, p.then_target, range,
                        ));
                        error_count += 1;
                    }
                }
            }
        } else if target_cl.all_lines.is_empty() {
            messages.push(format!(
                "error: {} -> {}: target file unchanged",
                if_ctx, p.then_target
            ));
            error_count += 1;
        }
    }

    LintResult {
        exit_code: if error_count > 0 { 1 } else { 0 },
        messages,
        verbose_messages,
        pairs_checked,
        files_checked: changed_files.len(),
    }
}
