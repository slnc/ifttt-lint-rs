use std::sync::OnceLock;

use regex::Regex;

use crate::comment::extract_comments;
use crate::directive::error::DirectiveParseError;
use crate::directive::patterns::patterns;
use crate::model::Directive;

fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack
        .as_bytes()
        .windows(needle.len())
        .any(|w| w.eq_ignore_ascii_case(needle.as_bytes()))
}

fn starts_with_ci(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

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

    let mut comment_idx = 0;

    while comment_idx < comments.len() {
        let comment = &comments[comment_idx];
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
                if contains_ci(line_text, "LINT.IfChange(") {
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
            if contains_ci(line_text, "LINT.ThenChange") {
                let trimmed = line_text.trim();
                if contains_ci(trimmed, "LINT.ThenChange")
                    && trimmed.contains('(')
                    && !trimmed.contains(')')
                {
                    // Multi-line: accumulate until ')'
                    let mut accumulated = line_text.to_string();
                    let directive_line = current_line;

                    // First try within this comment's remaining lines
                    let start = line_idx;
                    line_idx += 1;
                    let mut found_close = false;
                    while line_idx < comment_lines.len() {
                        let next_line = comment_lines[line_idx];
                        accumulated.push(' ');
                        accumulated.push_str(next_line);
                        if next_line.contains(')') {
                            line_idx += 1;
                            found_close = true;
                            break;
                        }
                        line_idx += 1;
                    }

                    // If not found and this is a single-line comment, look at subsequent comments
                    if !found_close && comment_lines.len() == 1 {
                        let mut next_ci = comment_idx + 1;
                        while next_ci < comments.len() {
                            let next_comment = &comments[next_ci];
                            // Only consume consecutive single-line comments
                            if next_comment.text.lines().count() != 1 {
                                break;
                            }
                            accumulated.push(' ');
                            accumulated.push_str(&next_comment.text);
                            if next_comment.text.contains(')') {
                                next_ci += 1;
                                found_close = true;
                                break;
                            }
                            next_ci += 1;
                        }
                        if found_close {
                            comment_idx = next_ci;
                        }
                    }

                    let _ = start;
                    if let Some(caps) = pats.then_change_array.captures(&accumulated) {
                        let inner = caps.get(1).unwrap().as_str();
                        for target in parse_array_targets(inner) {
                            directives.push(Directive::ThenChange {
                                line: directive_line,
                                target,
                            });
                        }
                        if found_close && comment_lines.len() == 1 {
                            // comment_idx already advanced, skip to outer loop
                            break;
                        }
                        continue;
                    }
                    if let Some(caps) = pats.then_change_single.captures(&accumulated) {
                        let target = caps.get(1).unwrap().as_str().to_string();
                        directives.push(Directive::ThenChange {
                            line: directive_line,
                            target,
                        });
                        if found_close && comment_lines.len() == 1 {
                            break;
                        }
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
            if contains_ci(line_text, "LINT.Label") {
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
                if starts_with_ci(name, "IfChange") {
                    return Err(DirectiveParseError::MalformedDirective {
                        directive: "LINT.IfChange",
                        path: file_path.to_string(),
                        line: current_line,
                        expected: "LINT.IfChange or LINT.IfChange(\"label\")",
                        found: line_text.trim().to_string(),
                    });
                }
                if starts_with_ci(name, "ThenChange") {
                    return Err(DirectiveParseError::MalformedDirective {
                        directive: "LINT.ThenChange",
                        path: file_path.to_string(),
                        line: current_line,
                        expected: "LINT.ThenChange(\"target\")",
                        found: line_text.trim().to_string(),
                    });
                }
                if starts_with_ci(name, "Label") {
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
        comment_idx += 1;
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

    fn then_targets(directives: Vec<Directive>) -> Vec<String> {
        directives
            .into_iter()
            .filter_map(|d| match d {
                Directive::ThenChange { target, .. } => Some(target),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn parse_array_targets_mixed_quotes() {
        assert_eq!(
            parse_array_targets(r#"'foo.ts', "bar.ts""#),
            vec!["foo.ts", "bar.ts"]
        );
    }

    #[test]
    fn file_directives_missing_file_returns_empty() {
        assert!(parse_file_directives("/definitely/not/found/file.ts")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn file_directives_directory_returns_empty() {
        let dir = TempDir::new().unwrap();
        assert!(parse_file_directives(dir.path().to_str().unwrap())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn file_directives_reads_content() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("a.ts");
        fs::write(&file, "// LINT.IfChange\n// LINT.ThenChange(\"b.ts\")\n").unwrap();
        let directives = parse_file_directives(file.to_str().unwrap()).unwrap();
        assert_eq!(
            directives,
            vec![
                Directive::IfChange {
                    line: 1,
                    label: None
                },
                Directive::ThenChange {
                    line: 2,
                    target: "b.ts".into()
                },
            ]
        );
    }

    #[test]
    fn malformed_ifchange_error() {
        let err = parse_directives_from_content("// LINT.IfChange(\n", "x.ts").unwrap_err();
        assert!(err.to_string().contains("Malformed LINT.IfChange"));
    }

    #[test]
    fn thenchange_multiline_array() {
        let content = "/*\nLINT.ThenChange([\n\"a.ts\",\n\"b.ts\",\n])\n*/\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert_eq!(then_targets(directives), vec!["a.ts", "b.ts"]);
    }

    #[test]
    fn thenchange_multiline_malformed_error() {
        let err = parse_directives_from_content("/*\nLINT.ThenChange(\n\"a.ts\"\n*/\n", "x.ts")
            .unwrap_err();
        assert!(err.to_string().contains("Malformed LINT.ThenChange"));
    }

    #[test]
    fn thenchange_singleline_array() {
        let directives =
            parse_directives_from_content("// LINT.ThenChange([\"a.ts\", \"b.ts\"])\n", "x.ts")
                .unwrap();
        assert_eq!(then_targets(directives), vec!["a.ts", "b.ts"]);
    }

    #[test]
    fn thenchange_fallback_target() {
        let directives =
            parse_directives_from_content("// LINT.ThenChange(foo.ts)\n", "x.ts").unwrap();
        assert_eq!(then_targets(directives), vec!["foo.ts"]);
    }

    #[test]
    fn thenchange_multiline_single_target() {
        let directives =
            parse_directives_from_content("/*\nLINT.ThenChange(\n\"one.ts\"\n)\n*/\n", "x.ts")
                .unwrap();
        assert_eq!(then_targets(directives), vec!["one.ts"]);
    }

    #[test]
    fn thenchange_without_parens_errors() {
        let err = parse_directives_from_content("// LINT.ThenChange nope\n", "x.ts").unwrap_err();
        assert!(err.to_string().contains("Malformed LINT.ThenChange"));
    }

    #[test]
    fn malformed_directive_errors() {
        for (input, expected) in [
            ("// LINT.IfChanges\n", "Malformed LINT.IfChange"),
            ("// LINT.ThenChanges\n", "Malformed LINT.ThenChange"),
            ("// LINT.Labels\n", "Malformed LINT.Label"),
            ("// LINT.Label(\n", "Malformed LINT.Label"),
        ] {
            let err = parse_directives_from_content(input, "x.ts").unwrap_err();
            assert!(err.to_string().contains(expected), "input: {input}");
        }
    }

    #[test]
    fn unknown_directive_error() {
        let err = parse_directives_from_content("// LINT.Frobulate(\"x\")\n", "x.ts").unwrap_err();
        assert!(err.to_string().contains("Unknown LINT directive"));
    }

    #[test]
    fn case_insensitive_ifchange_bare() {
        for variant in [
            "lint.ifchange",
            "Lint.Ifchange",
            "LINT.IFCHANGE",
            "Lint.IfChange",
        ] {
            let content = format!("// {variant}\n// LINT.ThenChange(\"b.ts\")\n");
            let directives = parse_directives_from_content(&content, "x.ts").unwrap();
            assert!(
                directives
                    .iter()
                    .any(|d| matches!(d, Directive::IfChange { label: None, .. })),
                "failed for variant: {variant}"
            );
        }
    }

    #[test]
    fn case_insensitive_ifchange_labeled() {
        for variant in [
            r#"lint.ifchange("lbl")"#,
            r#"LINT.IFCHANGE("lbl")"#,
            r#"Lint.Ifchange("lbl")"#,
        ] {
            let content = format!("// {variant}\n// LINT.ThenChange(\"b.ts\")\n");
            let directives = parse_directives_from_content(&content, "x.ts").unwrap();
            assert!(
                directives
                    .iter()
                    .any(|d| matches!(d, Directive::IfChange { label: Some(l), .. } if l == "lbl")),
                "failed for variant: {variant}"
            );
        }
    }

    #[test]
    fn case_insensitive_thenchange() {
        for variant in [
            r#"lint.thenchange("b.ts")"#,
            r#"LINT.THENCHANGE("b.ts")"#,
            r#"Lint.ThenChange("b.ts")"#,
        ] {
            let content = format!("// LINT.IfChange\n// {variant}\n");
            let directives = parse_directives_from_content(&content, "x.ts").unwrap();
            assert_eq!(
                then_targets(directives),
                vec!["b.ts"],
                "failed for variant: {variant}"
            );
        }
    }

    #[test]
    fn case_insensitive_thenchange_array() {
        let content = "// LINT.IfChange\n// lint.thenchange([\"a.ts\", \"b.ts\"])\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert_eq!(then_targets(directives), vec!["a.ts", "b.ts"]);
    }

    #[test]
    fn case_insensitive_thenchange_fallback() {
        let content = "// LINT.IfChange\n// lint.thenchange(foo.ts)\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert_eq!(then_targets(directives), vec!["foo.ts"]);
    }

    #[test]
    fn case_insensitive_label_and_endlabel() {
        let content = "// lint.label(\"sec\")\n// lint.endlabel\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert!(directives
            .iter()
            .any(|d| matches!(d, Directive::Label { name, .. } if name == "sec")),);
        assert!(directives
            .iter()
            .any(|d| matches!(d, Directive::EndLabel { .. })),);
    }

    #[test]
    fn case_insensitive_thenchange_multiline() {
        let content = "/*\nlint.thenchange([\n\"a.ts\",\n\"b.ts\",\n])\n*/\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert_eq!(then_targets(directives), vec!["a.ts", "b.ts"]);
    }

    #[test]
    fn case_insensitive_malformed_ifchange_error() {
        let err = parse_directives_from_content("// lint.ifchange(\n", "x.ts").unwrap_err();
        assert!(err.to_string().contains("Malformed"));
    }

    #[test]
    fn case_insensitive_unknown_directive_error() {
        let err = parse_directives_from_content("// lint.frobulate(\"x\")\n", "x.ts").unwrap_err();
        assert!(err.to_string().contains("Unknown LINT directive"));
    }

    #[test]
    fn lint_dot_only_ignored() {
        assert!(parse_directives_from_content("// LINT.\n", "x.ts")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn ifchange_then_thenchange_pair() {
        let directives = parse_directives_from_content(
            "// LINT.IfChange\n// LINT.ThenChange(\"x.ts\")\n",
            "x.ts",
        )
        .unwrap();
        assert_eq!(then_targets(directives), vec!["x.ts"]);
    }

    #[test]
    fn thenchange_multiline_array_line_comments_slash() {
        let content = "// LINT.IfChange\nconst x = 1;\n// LINT.ThenChange([\n//   \"a.ts\",\n//   \"b.ts\",\n// ])\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert_eq!(then_targets(directives), vec!["a.ts", "b.ts"]);
    }

    #[test]
    fn thenchange_multiline_array_line_comments_hash() {
        let content =
            "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange([\n#   \"a.py\",\n#   \"b.py\",\n# ])\n";
        let directives = parse_directives_from_content(content, "x.yml").unwrap();
        assert_eq!(then_targets(directives), vec!["a.py", "b.py"]);
    }

    #[test]
    fn thenchange_multiline_single_target_line_comments() {
        let content = "// LINT.IfChange\n// LINT.ThenChange(\n//   \"a.ts\"\n// )\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert_eq!(then_targets(directives), vec!["a.ts"]);
    }

    #[test]
    fn thenchange_multiline_array_line_comments_dash() {
        let content =
            "-- LINT.IfChange\n-- LINT.ThenChange([\n--   \"a.sql\",\n--   \"b.sql\",\n-- ])\n";
        let directives = parse_directives_from_content(content, "x.sql").unwrap();
        assert_eq!(then_targets(directives), vec!["a.sql", "b.sql"]);
    }
}
