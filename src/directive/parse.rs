use std::sync::OnceLock;

use regex::Regex;

use crate::comment::extract_comments;
use crate::directive::error::DirectiveParseError;
use crate::directive::patterns::patterns;
use crate::model::Directive;

/// Parse all LINT directives from a file.
///
/// Returns `Ok(vec![])` if the file doesn't exist or is a directory.
/// Returns `Err(msg)` if a malformed directive is found.
pub fn parse_file_directives(file_path: &str) -> Result<Vec<Directive>, DirectiveParseError> {
    let metadata = match std::fs::metadata(file_path) {
        Ok(m) => m,
        Err(_) => return Ok(Vec::new()),
    };
    if metadata.is_dir() {
        return Ok(Vec::new());
    }

    let content =
        std::fs::read_to_string(file_path).map_err(|source| DirectiveParseError::ReadFile {
            path: file_path.to_string(),
            source,
        })?;

    parse_directives_from_content(&content, file_path)
}

/// Parse LINT directives from content string (used by both file parsing and check mode).
pub fn parse_directives_from_content(
    content: &str,
    file_path: &str,
) -> Result<Vec<Directive>, DirectiveParseError> {
    let ext = file_path.rsplit('.').next().unwrap_or("");
    let comments = extract_comments(content, ext);
    let pats = patterns();
    let mut directives = Vec::new();

    for comment in &comments {
        let comment_lines: Vec<&str> = comment.text.lines().collect();
        let mut line_idx = 0;

        while line_idx < comment_lines.len() {
            let line_text = comment_lines[line_idx];
            let current_line = comment.start_line + line_idx;

            if !pats.lint_dot.is_match(line_text) {
                line_idx += 1;
                continue;
            }

            // IfChange with label
            if let Some(caps) = pats.if_change_labeled.captures(line_text) {
                let label = caps.get(1).unwrap().as_str().to_string();
                directives.push(Directive::IfChange {
                    line: current_line,
                    label: Some(label),
                });
                line_idx += 1;
                continue;
            }

            // Bare IfChange
            if pats.if_change_bare.is_match(line_text) {
                if line_text.contains("LINT.IfChange(") {
                    return Err(DirectiveParseError::MalformedDirective {
                        directive: "LINT.IfChange",
                        path: file_path.to_string(),
                        line: current_line,
                        expected: "LINT.IfChange or LINT.IfChange(\"label\")",
                        found: line_text.trim().to_string(),
                    });
                }
                directives.push(Directive::IfChange {
                    line: current_line,
                    label: None,
                });
                line_idx += 1;
                continue;
            }

            // ThenChange
            if line_text.contains("LINT.ThenChange") {
                let trimmed = line_text.trim();
                if trimmed.contains("LINT.ThenChange")
                    && trimmed.contains('(')
                    && !trimmed.contains(')')
                {
                    // Multi-line: accumulate until ')'
                    let mut accumulated = line_text.to_string();
                    let start = line_idx;
                    line_idx += 1;
                    while line_idx < comment_lines.len() {
                        let next_line = comment_lines[line_idx];
                        accumulated.push(' ');
                        accumulated.push_str(next_line);
                        if next_line.contains(')') {
                            line_idx += 1;
                            break;
                        }
                        line_idx += 1;
                    }
                    let directive_line = comment.start_line + start;
                    if let Some(caps) = pats.then_change_array.captures(&accumulated) {
                        let inner = caps.get(1).unwrap().as_str();
                        for target in parse_array_targets(inner) {
                            directives.push(Directive::ThenChange {
                                line: directive_line,
                                target,
                            });
                        }
                        continue;
                    }
                    if let Some(caps) = pats.then_change_single.captures(&accumulated) {
                        let target = caps.get(1).unwrap().as_str().to_string();
                        directives.push(Directive::ThenChange {
                            line: directive_line,
                            target,
                        });
                        continue;
                    }
                    return Err(DirectiveParseError::MalformedDirective {
                        directive: "LINT.ThenChange",
                        path: file_path.to_string(),
                        line: directive_line,
                        expected: "LINT.ThenChange(\"target\")",
                        found: accumulated.trim().replace('\n', " "),
                    });
                }

                // Single-line: try array first, then single
                if let Some(caps) = pats.then_change_array.captures(line_text) {
                    let inner = caps.get(1).unwrap().as_str();
                    for target in parse_array_targets(inner) {
                        directives.push(Directive::ThenChange {
                            line: current_line,
                            target,
                        });
                    }
                    line_idx += 1;
                    continue;
                }
                if let Some(caps) = pats.then_change_single.captures(line_text) {
                    let target = caps.get(1).unwrap().as_str().to_string();
                    directives.push(Directive::ThenChange {
                        line: current_line,
                        target,
                    });
                    line_idx += 1;
                    continue;
                }
                // Fallback: try to extract anything from LINT.ThenChange(...)
                if let Some(caps) = pats.then_change_fallback.captures(line_text) {
                    let raw = caps.get(1).unwrap().as_str().trim();
                    let target = raw.trim_matches(|c| c == '\'' || c == '"').to_string();
                    directives.push(Directive::ThenChange {
                        line: current_line,
                        target,
                    });
                    line_idx += 1;
                    continue;
                }
                return Err(DirectiveParseError::MalformedDirective {
                    directive: "LINT.ThenChange",
                    path: file_path.to_string(),
                    line: current_line,
                    expected: "LINT.ThenChange(\"target\")",
                    found: line_text.trim().to_string(),
                });
            }

            // Label
            if line_text.contains("LINT.Label") {
                if let Some(caps) = pats.label.captures(line_text) {
                    let name = caps.get(1).unwrap().as_str().to_string();
                    directives.push(Directive::Label {
                        line: current_line,
                        name,
                    });
                    line_idx += 1;
                    continue;
                }
                return Err(DirectiveParseError::MalformedDirective {
                    directive: "LINT.Label",
                    path: file_path.to_string(),
                    line: current_line,
                    expected: "LINT.Label(\"name\")",
                    found: line_text.trim().to_string(),
                });
            }

            // EndLabel
            if pats.end_label.is_match(line_text) {
                directives.push(Directive::EndLabel { line: current_line });
                line_idx += 1;
                continue;
            }

            // Unknown LINT directive
            if let Some(caps) = pats.lint_directive_name.captures(line_text) {
                let name = caps.get(1).unwrap().as_str();
                if name.starts_with("IfChange") {
                    return Err(DirectiveParseError::MalformedDirective {
                        directive: "LINT.IfChange",
                        path: file_path.to_string(),
                        line: current_line,
                        expected: "LINT.IfChange or LINT.IfChange(\"label\")",
                        found: line_text.trim().to_string(),
                    });
                }
                if name.starts_with("ThenChange") {
                    return Err(DirectiveParseError::MalformedDirective {
                        directive: "LINT.ThenChange",
                        path: file_path.to_string(),
                        line: current_line,
                        expected: "LINT.ThenChange(\"target\")",
                        found: line_text.trim().to_string(),
                    });
                }
                if name.starts_with("Label") {
                    return Err(DirectiveParseError::MalformedDirective {
                        directive: "LINT.Label",
                        path: file_path.to_string(),
                        line: current_line,
                        expected: "LINT.Label(\"name\")",
                        found: line_text.trim().to_string(),
                    });
                }
                return Err(DirectiveParseError::UnknownDirective {
                    name: name.to_string(),
                    path: file_path.to_string(),
                    line: current_line,
                    line_text: line_text.trim().to_string(),
                });
            }

            line_idx += 1;
        }
    }

    Ok(directives)
}

fn parse_array_targets(inner: &str) -> Vec<String> {
    static QUOTED: OnceLock<Regex> = OnceLock::new();
    let re = QUOTED.get_or_init(|| Regex::new(r#"['\"]([^'\"]+)['\"]"#).unwrap());
    re.captures_iter(inner)
        .map(|c| c.get(1).unwrap().as_str().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_array_targets() {
        let targets = parse_array_targets(r#"'foo.ts', "bar.ts""#);
        assert_eq!(targets, vec!["foo.ts", "bar.ts"]);
    }

    #[test]
    fn test_parse_file_directives_missing_file_returns_empty() {
        let result = parse_file_directives("/definitely/not/found/file.ts").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_file_directives_directory_returns_empty() {
        let dir = TempDir::new().unwrap();
        let result = parse_file_directives(dir.path().to_str().unwrap()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_file_directives_reads_content() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("a.ts");
        fs::write(&file, "// LINT.IfChange\n// LINT.ThenChange(\"b.ts\")\n").unwrap();
        let directives = parse_file_directives(file.to_str().unwrap()).unwrap();
        assert!(matches!(directives[0], Directive::IfChange { .. }));
        assert!(matches!(directives[1], Directive::ThenChange { .. }));
    }

    #[test]
    fn test_parse_malformed_ifchange_error() {
        let err = parse_directives_from_content("// LINT.IfChange(\n", "x.ts").unwrap_err();
        assert!(err.to_string().contains("Malformed LINT.IfChange"));
    }

    #[test]
    fn test_parse_thenchange_multiline_array() {
        let content = "/*\nLINT.ThenChange([\n\"a.ts\",\n\"b.ts\",\n])\n*/\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        let targets: Vec<String> = directives
            .into_iter()
            .filter_map(|d| match d {
                Directive::ThenChange { target, .. } => Some(target),
                _ => None,
            })
            .collect();
        assert_eq!(targets, vec!["a.ts".to_string(), "b.ts".to_string()]);
    }

    #[test]
    fn test_parse_thenchange_multiline_malformed_error() {
        let content = "/*\nLINT.ThenChange(\n\"a.ts\"\n*/\n";
        let err = parse_directives_from_content(content, "x.ts").unwrap_err();
        assert!(err.to_string().contains("Malformed LINT.ThenChange"));
    }

    #[test]
    fn test_parse_thenchange_singleline_array() {
        let directives =
            parse_directives_from_content("// LINT.ThenChange([\"a.ts\", \"b.ts\"])\n", "x.ts")
                .unwrap();
        assert_eq!(
            directives
                .into_iter()
                .filter(|d| matches!(d, Directive::ThenChange { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn test_parse_thenchange_fallback_target() {
        let directives =
            parse_directives_from_content("// LINT.ThenChange(foo.ts)\n", "x.ts").unwrap();
        let target = directives
            .into_iter()
            .find_map(|d| match d {
                Directive::ThenChange { target, .. } => Some(target),
                _ => None,
            })
            .unwrap();
        assert_eq!(target, "foo.ts");
    }

    #[test]
    fn test_parse_thenchange_multiline_single_target() {
        let content = "/*\nLINT.ThenChange(\n\"one.ts\"\n)\n*/\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        let target = directives
            .into_iter()
            .find_map(|d| match d {
                Directive::ThenChange { target, .. } => Some(target),
                _ => None,
            })
            .unwrap();
        assert_eq!(target, "one.ts");
    }

    #[test]
    fn test_parse_thenchange_without_parens_errors() {
        let err = parse_directives_from_content("// LINT.ThenChange nope\n", "x.ts").unwrap_err();
        assert!(err.to_string().contains("Malformed LINT.ThenChange"));
    }

    #[test]
    fn test_parse_unknown_ifchange_like_directive_error() {
        let err = parse_directives_from_content("// LINT.IfChanges\n", "x.ts").unwrap_err();
        assert!(err.to_string().contains("Malformed LINT.IfChange"));
    }

    #[test]
    fn test_parse_unknown_thenchange_like_directive_error() {
        let err = parse_directives_from_content("// LINT.ThenChanges\n", "x.ts").unwrap_err();
        assert!(err.to_string().contains("Malformed LINT.ThenChange"));
    }

    #[test]
    fn test_parse_unknown_label_like_directive_error() {
        let err = parse_directives_from_content("// LINT.Labels\n", "x.ts").unwrap_err();
        assert!(err.to_string().contains("Malformed LINT.Label"));
    }

    #[test]
    fn test_parse_malformed_label_error() {
        let err = parse_directives_from_content("// LINT.Label(\n", "x.ts").unwrap_err();
        assert!(err.to_string().contains("Malformed LINT.Label"));
    }

    #[test]
    fn test_parse_unknown_directive_error() {
        let err = parse_directives_from_content("// LINT.Frobulate(\"x\")\n", "x.ts").unwrap_err();
        assert!(err.to_string().contains("Unknown LINT directive"));
    }

    #[test]
    fn test_parse_lint_dot_only_ignored() {
        let directives = parse_directives_from_content("// LINT.\n", "x.ts").unwrap();
        assert!(directives.is_empty());
    }

    #[test]
    fn test_find_map_none_branch_for_non_thenchange() {
        let directives = parse_directives_from_content(
            "// LINT.IfChange\n// LINT.ThenChange(\"x.ts\")\n",
            "x.ts",
        )
        .unwrap();
        let first_then = directives.into_iter().find_map(|d| match d {
            Directive::ThenChange { target, .. } => Some(target),
            _ => None,
        });
        assert_eq!(first_then.as_deref(), Some("x.ts"));
    }
}
