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

/// Extract the effective file extension (or special filename) for comment-style detection.
///
/// Handles `Dockerfile.variant` by returning `"dockerfile"` so that hash-style comments
/// are used regardless of the suffix.
fn effective_extension(file_path: &str) -> &str {
    let filename = std::path::Path::new(file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(file_path);
    let filename_lower = filename.as_bytes();
    // "Dockerfile" or "Dockerfile.something"
    if filename_lower.len() >= 10
        && filename_lower[..10].eq_ignore_ascii_case(b"dockerfile")
        && (filename_lower.len() == 10 || filename_lower[10] == b'.')
    {
        return "dockerfile";
    }
    // "go.mod" uses // line comments
    if filename.eq_ignore_ascii_case("go.mod") {
        return "go.mod";
    }
    filename.rsplit('.').next().unwrap_or("")
}

/// Parse LINT directives from content string (used by both file parsing and check mode).
pub fn parse_directives_from_content(
    content: &str,
    file_path: &str,
) -> Result<Vec<Directive>, DirectiveParseError> {
    let ext = effective_extension(file_path);
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
            if let Some(caps) = pats.if_change_labeled_unquoted.captures(line_text) {
                let label = caps.get(1).unwrap().as_str().trim().to_string();
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
            if starts_with_ci(line_text.trim(), "LINT.ThenChange") {
                let trimmed = line_text.trim();
                if trimmed.contains('(') && !trimmed.contains(')') {
                    // Multi-line: accumulate until ')'
                    let mut accumulated = line_text.to_string();
                    let directive_line = current_line;

                    // First try within this comment's remaining lines
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
                        let mut expected_line = comment.start_line + 1;
                        let mut next_ci = comment_idx + 1;
                        while next_ci < comments.len() {
                            let next_comment = &comments[next_ci];
                            // Only consume truly adjacent single-line comments
                            if next_comment.text.lines().count() != 1
                                || next_comment.start_line != expected_line
                            {
                                break;
                            }
                            accumulated.push(' ');
                            accumulated.push_str(&next_comment.text);
                            expected_line += 1;
                            if next_comment.text.contains(')') {
                                next_ci += 1;
                                found_close = true;
                                break;
                            }
                            next_ci += 1;
                        }
                        if found_close {
                            // next_ci points past the last consumed comment.
                            // Subtract 1 because the outer loop does comment_idx += 1.
                            comment_idx = next_ci - 1;
                        }
                    }

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
                    // Fallback: multi-line without brackets (e.g., quoted targets separated by commas)
                    if let Some(caps) = pats.then_change_fallback.captures(&accumulated) {
                        let raw = caps.get(1).unwrap().as_str().trim();
                        let targets = parse_array_targets(raw);
                        if !targets.is_empty() {
                            for target in targets {
                                directives.push(Directive::ThenChange {
                                    line: directive_line,
                                    target,
                                });
                            }
                            if found_close && comment_lines.len() == 1 {
                                break;
                            }
                            continue;
                        }
                        // Single unquoted target fallback (mirrors single-line behavior)
                        let target = raw
                            .trim_matches(|c| c == '\'' || c == '"')
                            .trim()
                            .to_string();
                        if !target.is_empty() {
                            directives.push(Directive::ThenChange {
                                line: directive_line,
                                target,
                            });
                            if found_close && comment_lines.len() == 1 {
                                break;
                            }
                            continue;
                        }
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
                // Fallback: try to extract anything from the ThenChange directive
                if let Some(caps) = pats.then_change_fallback.captures(line_text) {
                    let raw = caps.get(1).unwrap().as_str().trim();
                    // Multiple quoted strings without brackets: treat as implicit array
                    let targets = parse_array_targets(raw);
                    if targets.len() > 1 {
                        for target in targets {
                            directives.push(Directive::ThenChange {
                                line: current_line,
                                target,
                            });
                        }
                        line_idx += 1;
                        continue;
                    }
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
            if starts_with_ci(line_text.trim(), "LINT.Label") {
                if let Some(caps) = pats.label.captures(line_text) {
                    let name = caps.get(1).unwrap().as_str().to_string();
                    directives.push(Directive::Label {
                        line: current_line,
                        name,
                    });
                    line_idx += 1;
                    continue;
                }
                if let Some(caps) = pats.label_unquoted.captures(line_text) {
                    let name = caps.get(1).unwrap().as_str().trim().to_string();
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
    if !inner.contains(',') {
        // Single target: try stripping quotes
        static QUOTED: OnceLock<Regex> = OnceLock::new();
        let re = QUOTED.get_or_init(|| Regex::new(r#"['\"]([^'\"]*)['\"]"#).unwrap());
        return re
            .captures_iter(inner)
            .map(|c| c.get(1).unwrap().as_str().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    // Split on commas, strip quotes from each element.
    // Handles quoted, unquoted, and mixed lists uniformly.
    inner
        .split(',')
        .map(|s| {
            s.trim()
                .trim_matches(|c| c == '\'' || c == '"')
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
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

    #[rstest]
    #[case("// LINT.IfChanges\n", "Malformed LINT.IfChange")]
    #[case("// LINT.ThenChanges\n", "Malformed LINT.ThenChange")]
    #[case("// LINT.Labels\n", "Malformed LINT.Label")]
    #[case("// LINT.Label(\n", "Malformed LINT.Label")]
    fn malformed_directive_errors(#[case] input: &str, #[case] expected: &str) {
        let err = parse_directives_from_content(input, "x.ts").unwrap_err();
        assert!(err.to_string().contains(expected));
    }

    #[test]
    fn unknown_directive_error() {
        let err = parse_directives_from_content("// LINT.Frobulate(\"x\")\n", "x.ts").unwrap_err();
        assert!(err.to_string().contains("Unknown LINT directive"));
    }

    #[rstest]
    fn case_insensitive_ifchange_bare(
        #[values("lint.ifchange", "Lint.Ifchange", "LINT.IFCHANGE", "Lint.IfChange")] variant: &str,
    ) {
        let content = format!("// {variant}\n// LINT.ThenChange(\"b.ts\")\n");
        let directives = parse_directives_from_content(&content, "x.ts").unwrap();
        assert!(directives
            .iter()
            .any(|d| matches!(d, Directive::IfChange { label: None, .. })));
    }

    #[rstest]
    fn case_insensitive_ifchange_labeled(
        #[values(
            r#"lint.ifchange("lbl")"#,
            r#"LINT.IFCHANGE("lbl")"#,
            r#"Lint.Ifchange("lbl")"#
        )]
        variant: &str,
    ) {
        let content = format!("// {variant}\n// LINT.ThenChange(\"b.ts\")\n");
        let directives = parse_directives_from_content(&content, "x.ts").unwrap();
        assert!(directives
            .iter()
            .any(|d| matches!(d, Directive::IfChange { label: Some(l), .. } if l == "lbl")));
    }

    #[rstest]
    fn case_insensitive_thenchange(
        #[values(
            r#"lint.thenchange("b.ts")"#,
            r#"LINT.THENCHANGE("b.ts")"#,
            r#"Lint.ThenChange("b.ts")"#
        )]
        variant: &str,
    ) {
        let content = format!("// LINT.IfChange\n// {variant}\n");
        let directives = parse_directives_from_content(&content, "x.ts").unwrap();
        assert_eq!(then_targets(directives), vec!["b.ts"]);
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

    #[rstest]
    #[case(
        "// LINT.IfChange\nconst x = 1;\n// LINT.ThenChange([\n//   \"a.ts\",\n//   \"b.ts\",\n// ])\n",
        "x.ts",
        vec!["a.ts", "b.ts"],
    )]
    #[case(
        "# LINT.IfChange\nVALUE = 1\n# LINT.ThenChange([\n#   \"a.py\",\n#   \"b.py\",\n# ])\n",
        "x.yml",
        vec!["a.py", "b.py"],
    )]
    #[case(
        "-- LINT.IfChange\n-- LINT.ThenChange([\n--   \"a.sql\",\n--   \"b.sql\",\n-- ])\n",
        "x.sql",
        vec!["a.sql", "b.sql"],
    )]
    fn thenchange_multiline_array_line_comments(
        #[case] content: &str,
        #[case] ext: &str,
        #[case] expected: Vec<&str>,
    ) {
        let directives = parse_directives_from_content(content, ext).unwrap();
        assert_eq!(then_targets(directives), expected);
    }

    #[test]
    fn thenchange_multiline_single_target_line_comments() {
        let content = "// LINT.IfChange\n// LINT.ThenChange(\n//   \"a.ts\"\n// )\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert_eq!(then_targets(directives), vec!["a.ts"]);
    }

    #[rstest]
    #[case("src/foo.rs", "rs")]
    #[case("foo.py", "py")]
    #[case("noext", "noext")]
    fn effective_extension_regular_files(#[case] path: &str, #[case] expected: &str) {
        assert_eq!(effective_extension(path), expected);
    }

    #[rstest]
    #[case("go.mod")]
    #[case("path/to/go.mod")]
    fn effective_extension_go_mod(#[case] path: &str) {
        assert_eq!(effective_extension(path), "go.mod");
    }

    #[test]
    fn go_mod_parses_slash_comments() {
        let content = "// LINT.IfChange\nrequire foo v1.0.0\n// LINT.ThenChange(\"other.go\")\n";
        let directives = parse_directives_from_content(content, "go.mod").unwrap();
        assert_eq!(then_targets(directives), vec!["other.go"]);
    }

    #[rstest]
    #[case("Dockerfile", "dockerfile")]
    #[case("Dockerfile.prod", "dockerfile")]
    #[case("path/to/Dockerfile", "dockerfile")]
    #[case("path/to/Dockerfile.dev", "dockerfile")]
    #[case("DOCKERFILE", "dockerfile")]
    #[case("dockerfile.staging", "dockerfile")]
    fn effective_extension_dockerfile_variants(#[case] path: &str, #[case] expected: &str) {
        assert_eq!(effective_extension(path), expected);
    }

    #[test]
    fn dockerfile_variant_parses_hash_comments() {
        let content = "# LINT.IfChange\nRUN echo hi\n# LINT.ThenChange(\"other.py\")\n";
        let directives = parse_directives_from_content(content, "Dockerfile.prod").unwrap();
        assert_eq!(then_targets(directives), vec!["other.py"]);
    }
}

#[cfg(test)]
mod bug_tests {
    use super::*;
    use rstest::rstest;

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
    fn parse_array_targets_skips_empty_strings() {
        let targets = parse_array_targets(r#""a.py", "", "b.py""#);
        assert_eq!(targets, vec!["a.py", "b.py"]);
    }

    #[test]
    fn thenchange_array_with_empty_element() {
        let content = "// LINT.ThenChange([\"a.ts\", \"\", \"b.ts\"])\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert_eq!(then_targets(directives), vec!["a.ts", "b.ts"]);
    }

    #[test]
    fn thenchange_multiple_quoted_without_brackets() {
        let content = "// LINT.ThenChange(\"a.py\", \"b.py\")\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert_eq!(then_targets(directives), vec!["a.py", "b.py"]);
    }

    #[rstest]
    #[case("// LINT.ThenChange(/foo.txt, /bar.txt)\n", vec!["/foo.txt", "/bar.txt"])]
    #[case("// LINT.ThenChange(foo.txt, bar.txt)\n", vec!["foo.txt", "bar.txt"])]
    #[case("// LINT.ThenChange(/foo.txt#label1, /bar.txt#label2)\n", vec!["/foo.txt#label1", "/bar.txt#label2"])]
    #[case("// LINT.ThenChange(  /foo.txt ,  /bar.txt  )\n", vec!["/foo.txt", "/bar.txt"])]
    #[case("// LINT.ThenChange(foo.txt)\n", vec!["foo.txt"])]
    #[case("// LINT.ThenChange(a.txt, b.txt, c.txt)\n", vec!["a.txt", "b.txt", "c.txt"])]
    fn thenchange_unquoted_variants(#[case] content: &str, #[case] expected: Vec<&str>) {
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert_eq!(then_targets(directives), expected);
    }

    #[rstest]
    #[case("my_section")]
    #[case("my-section-v2")]
    fn label_unquoted(#[case] name: &str) {
        let content = format!("// LINT.Label({name})\n// LINT.EndLabel\n");
        let directives = parse_directives_from_content(&content, "x.ts").unwrap();
        assert!(directives
            .iter()
            .any(|d| matches!(d, Directive::Label { name: n, .. } if n == name)));
    }

    #[rstest]
    fn label_case_insensitive_mixed(
        #[values(
            r#"lint.label("sec")"#,
            r#"LINT.LABEL("sec")"#,
            r#"Lint.Label("sec")"#,
            r#"lint.LaBeL("sec")"#,
            r#"LINT.label("sec")"#
        )]
        variant: &str,
    ) {
        let content = format!("// {variant}\n// LINT.EndLabel\n");
        let directives = parse_directives_from_content(&content, "x.ts").unwrap();
        assert!(directives
            .iter()
            .any(|d| matches!(d, Directive::Label { name, .. } if name == "sec")));
    }

    #[test]
    fn label_unquoted_case_insensitive() {
        let content = "// lint.label(my_section)\n// lint.endlabel\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert!(directives
            .iter()
            .any(|d| matches!(d, Directive::Label { name, .. } if name == "my_section")));
    }

    #[rstest]
    #[case("LINT.IfChange(my-feature)", "my-feature")]
    #[case("lint.ifchange(my_feature)", "my_feature")]
    fn ifchange_unquoted_label(#[case] directive: &str, #[case] expected_label: &str) {
        let content = format!("// {directive}\n// LINT.ThenChange(\"b.ts\")\n");
        let directives = parse_directives_from_content(&content, "x.ts").unwrap();
        assert!(directives.iter().any(
            |d| matches!(d, Directive::IfChange { label: Some(l), .. } if l == expected_label)
        ));
    }

    #[rstest]
    fn directive_mid_comment_ignored(
        #[values(
            "// some text LINT.IfChange\n",
            "// mentioning LINT.ThenChange(\"foo\")\n",
            "// about LINT.Label(\"x\")\n"
        )]
        input: &str,
    ) {
        let directives = parse_directives_from_content(input, "x.ts").unwrap();
        assert!(directives.is_empty());
    }

    #[test]
    fn directive_with_leading_whitespace() {
        let content = "/*\n  LINT.IfChange\n  LINT.ThenChange(\"b.ts\")\n*/\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert_eq!(directives.len(), 2);
        assert!(matches!(
            &directives[0],
            Directive::IfChange { label: None, .. }
        ));
    }

    #[rstest]
    fn endlabel_case_insensitive_mixed(
        #[values(
            "lint.endlabel",
            "LINT.ENDLABEL",
            "Lint.EndLabel",
            "lint.EndLabel",
            "LINT.endlabel"
        )]
        variant: &str,
    ) {
        let content = format!("// LINT.Label(\"sec\")\n// {variant}\n");
        let directives = parse_directives_from_content(&content, "x.ts").unwrap();
        assert!(directives
            .iter()
            .any(|d| matches!(d, Directive::EndLabel { .. })));
    }

    // ── Multi-line ThenChange without brackets ──

    #[rstest]
    #[case("// LINT.ThenChange(\n//   \"a.ts\",\n//   \"b.ts\",\n// )\n", "x.ts", vec!["a.ts", "b.ts"])]
    #[case("# LINT.ThenChange(\n#   'a.py',\n#   'b.py',\n# )\n", "x.yml", vec!["a.py", "b.py"])]
    #[case("-- LINT.ThenChange(\n--   'a.sql',\n--   'b.sql',\n-- )\n", "x.sql", vec!["a.sql", "b.sql"])]
    fn thenchange_multiline_no_brackets_comment_styles(
        #[case] content: &str,
        #[case] ext: &str,
        #[case] expected: Vec<&str>,
    ) {
        let directives = parse_directives_from_content(content, ext).unwrap();
        assert_eq!(then_targets(directives), expected);
    }

    #[test]
    fn thenchange_multiline_no_brackets_close_on_last_target_line() {
        let content = "// LINT.ThenChange(\n//   'a.ts',\n//   'b.ts')\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert_eq!(then_targets(directives), vec!["a.ts", "b.ts"]);
    }

    #[test]
    fn thenchange_multiline_no_brackets_no_trailing_comma() {
        let content = "// LINT.ThenChange(\n//   'a.ts',\n//   'b.ts'\n// )\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert_eq!(then_targets(directives), vec!["a.ts", "b.ts"]);
    }

    #[test]
    fn thenchange_multiline_no_brackets_directive_after_block_not_skipped() {
        let content = "\
// LINT.IfChange
// LINT.ThenChange(
//   'a.ts',
// )
// LINT.IfChange
// LINT.ThenChange(
//   'b.ts',
// )
";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert_eq!(directives.len(), 4);
        assert!(matches!(&directives[0], Directive::IfChange { .. }));
        assert!(matches!(&directives[1], Directive::ThenChange { target, .. } if target == "a.ts"));
        assert!(matches!(&directives[2], Directive::IfChange { .. }));
        assert!(matches!(&directives[3], Directive::ThenChange { target, .. } if target == "b.ts"));
    }

    #[test]
    fn thenchange_multiline_no_brackets_block_comment() {
        let content = "/*\nLINT.ThenChange(\n\"a.ts\",\n\"b.ts\",\n)\n*/\n";
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert_eq!(then_targets(directives), vec!["a.ts", "b.ts"]);
    }

    #[test]
    fn thenchange_multiline_no_brackets_unclosed_errors() {
        let content = "// LINT.ThenChange(\n//   'a.ts',\n//   'b.ts',\n";
        let err = parse_directives_from_content(content, "x.ts").unwrap_err();
        assert!(err.to_string().contains("Malformed LINT.ThenChange"));
    }

    #[rstest]
    #[case("// LINT.ThenChange(\n//   foo.txt\n// )\n", "x.ts")]
    #[case("/*\nLINT.ThenChange(\nfoo.txt\n)\n*/\n", "x.ts")]
    fn thenchange_multiline_no_brackets_unquoted_single_target(
        #[case] content: &str,
        #[case] ext: &str,
    ) {
        let directives = parse_directives_from_content(content, ext).unwrap();
        assert_eq!(then_targets(directives), vec!["foo.txt"]);
    }

    #[test]
    fn thenchange_multiline_non_adjacent_comments_not_consumed() {
        let content = "// LINT.ThenChange(\n\n// )\n";
        let err = parse_directives_from_content(content, "x.ts").unwrap_err();
        assert!(err.to_string().contains("Malformed LINT.ThenChange"));
    }

    #[test]
    fn thenchange_multiline_code_between_comments_not_consumed() {
        let content = "// LINT.ThenChange(\nconst x = 1;\n// \"a.ts\"\n// )\n";
        let err = parse_directives_from_content(content, "x.ts").unwrap_err();
        assert!(err.to_string().contains("Malformed LINT.ThenChange"));
    }

    #[rstest]
    #[case("// LINT.ThenChange(\"a.ts\", b.ts)\n", vec!["a.ts", "b.ts"])]
    #[case("// LINT.ThenChange(\n//   \"a.ts\",\n//   b.ts,\n// )\n", vec!["a.ts", "b.ts"])]
    fn thenchange_mixed_quoted_unquoted(#[case] content: &str, #[case] expected: Vec<&str>) {
        let directives = parse_directives_from_content(content, "x.ts").unwrap();
        assert_eq!(then_targets(directives), expected);
    }
}
