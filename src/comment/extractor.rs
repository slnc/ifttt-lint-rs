/// A comment extracted from source code.
pub struct Comment {
    /// 1-based line number where the comment starts.
    pub start_line: usize,
    /// The comment text with delimiters removed.
    pub text: String,
}

/// Extensions that support C-style comments (`//` and `/* */`).
const C_STYLE_EXTS: &[&str] = &[
    "ts", "js", "java", "c", "cpp", "go", "rs", "php", "swift", "kt", "scala", "tsx", "mts", "cjs",
    "jsx", "mjs", "cs", "cc", "cxx", "c++", "h", "hpp", "hh", "hxx", "dart", "groovy", "gradle",
    "kts", "fs", "fsx", "fsi", "zig", "gleam",
];

/// Extensions that support hash-style comments (`#`).
const HASH_STYLE_EXTS: &[&str] = &[
    "py", "bzl", "rb", "sh", "toml", "yml", "yaml", "ps1", "psm1", "psd1", "bash", "zsh", "ksh",
    "r", "pl", "pm", "ex", "exs", "gd", "mojo",
];

/// Extensions that support `--` line comments (and optionally C-style `/* */` blocks).
const DASH_STYLE_EXTS: &[&str] = &["sql", "lua"];

/// Extensions that support `%` line comments.
const PERCENT_STYLE_EXTS: &[&str] = &["m", "erl", "hrl", "pro", "prolog"];

/// Extensions that support `;` line comments.
const SEMICOLON_STYLE_EXTS: &[&str] = &["asm", "s", "lisp", "lsp", "cl", "scm"];

/// Extensions that support apostrophe (`'`) comments.
const APOSTROPHE_STYLE_EXTS: &[&str] = &["vb", "vba", "bas", "cls"];

/// Extensions that support `!` comments.
const BANG_STYLE_EXTS: &[&str] = &["f", "for", "f90", "f95", "f03", "f08"];

/// Extract all comments from `content`, using the comment style implied by `file_ext`
/// (without the leading dot).
///
/// Returns a `Vec<Comment>` with 1-based line numbers.
pub fn extract_comments(content: &str, file_ext: &str) -> Vec<Comment> {
    let ext = file_ext
        .strip_prefix('.')
        .unwrap_or(file_ext)
        .to_ascii_lowercase();
    let ext = ext.as_str();

    if C_STYLE_EXTS.contains(&ext) {
        extract_c_style(content)
    } else if HASH_STYLE_EXTS.contains(&ext) {
        extract_hash_style(content)
    } else if DASH_STYLE_EXTS.contains(&ext) {
        extract_dash_style(content)
    } else if PERCENT_STYLE_EXTS.contains(&ext) {
        extract_percent_style(content)
    } else if SEMICOLON_STYLE_EXTS.contains(&ext) {
        extract_semicolon_style(content)
    } else if APOSTROPHE_STYLE_EXTS.contains(&ext) {
        extract_apostrophe_style(content)
    } else if BANG_STYLE_EXTS.contains(&ext) {
        extract_bang_style(content)
    } else {
        // Unknown extension: try C-style first, fall back to hash if nothing found.
        let comments = extract_c_style(content);
        if comments.is_empty() {
            extract_hash_style(content)
        } else {
            comments
        }
    }
}

/// Extract `//` line comments and `/* … */` block comments.
fn extract_c_style(content: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    let mut chars = content.char_indices().peekable();
    let mut line: usize = 1;
    let mut in_string: Option<char> = None;

    while let Some(&(_i, ch)) = chars.peek() {
        // Track strings so we don't pick up comment-like sequences inside them.
        if let Some(quote) = in_string {
            chars.next();
            if ch == '\\' {
                // Skip escaped character.
                if chars.peek().is_some() {
                    let next = chars.next().unwrap().1;
                    if next == '\n' {
                        line += 1;
                    }
                }
            } else if ch == quote {
                in_string = None;
            } else if ch == '\n' {
                line += 1;
            }
            continue;
        }

        match ch {
            '"' | '\'' | '`' => {
                in_string = Some(ch);
                chars.next();
            }
            '/' => {
                chars.next();
                match chars.peek().map(|&(_, c)| c) {
                    Some('/') => {
                        // Line comment.
                        chars.next(); // consume second '/'
                        let start = match chars.peek() {
                            Some(&(idx, _)) => idx,
                            None => content.len(),
                        };
                        let comment_line = line;
                        // Advance to end of line.
                        while let Some(&(_, c)) = chars.peek() {
                            if c == '\n' {
                                break;
                            }
                            chars.next();
                        }
                        let end = match chars.peek() {
                            Some(&(idx, _)) => idx,
                            None => content.len(),
                        };
                        comments.push(Comment {
                            start_line: comment_line,
                            text: content[start..end].to_string(),
                        });
                    }
                    Some('*') => {
                        // Block comment.
                        chars.next(); // consume '*'
                        let start = match chars.peek() {
                            Some(&(idx, _)) => idx,
                            None => content.len(),
                        };
                        let comment_line = line;
                        let mut end = content.len();
                        let mut found_end = false;
                        while let Some(&(_, c)) = chars.peek() {
                            if c == '\n' {
                                line += 1;
                                chars.next();
                            } else if c == '*' {
                                let star_pos = chars.peek().map(|&(idx, _)| idx).unwrap();
                                chars.next();
                                if let Some(&(_, '/')) = chars.peek() {
                                    end = star_pos;
                                    chars.next(); // consume '/'
                                    found_end = true;
                                    break;
                                }
                            } else {
                                chars.next();
                            }
                        }
                        if !found_end {
                            end = content.len();
                        }
                        let raw = &content[start..end];
                        let text = clean_block_comment(raw);
                        comments.push(Comment {
                            start_line: comment_line,
                            text,
                        });
                    }
                    _ => {}
                }
            }
            '\n' => {
                line += 1;
                chars.next();
            }
            _ => {
                chars.next();
            }
        }
    }

    comments
}

/// Clean a block comment body: strip leading `*` on each line (JSDoc-style).
fn clean_block_comment(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= 1 {
        return raw.to_string();
    }
    let cleaned: Vec<&str> = lines
        .iter()
        .map(|line| {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("* ") {
                rest
            } else if trimmed.strip_prefix('*').is_some_and(|s| s.is_empty()) {
                ""
            } else if let Some(rest) = trimmed.strip_prefix('*') {
                rest
            } else {
                line
            }
        })
        .collect();
    cleaned.join("\n")
}

/// Extract `#` line comments.
fn extract_hash_style(content: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix('#') {
            comments.push(Comment {
                start_line: idx + 1,
                text: rest.to_string(),
            });
        }
    }
    comments
}

/// Extract SQL/Lua-style comments:
/// - line comments with `--`
/// - block comments with `/* ... */` (for SQL compatibility)
fn extract_dash_style(content: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    comments.extend(extract_prefixed_line_comments(content, "--"));
    comments.extend(extract_c_block_only(content));
    comments.sort_by_key(|c| c.start_line);
    comments
}

/// Extract `%` line comments.
fn extract_percent_style(content: &str) -> Vec<Comment> {
    extract_prefixed_line_comments(content, "%")
}

/// Extract `;` line comments.
fn extract_semicolon_style(content: &str) -> Vec<Comment> {
    extract_prefixed_line_comments(content, ";")
}

/// Extract apostrophe (`'`) line comments and `REM` comments (VB/VBA).
fn extract_apostrophe_style(content: &str) -> Vec<Comment> {
    let mut comments = extract_prefixed_line_comments(content, "'");
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.len() >= 3 && trimmed[..3].eq_ignore_ascii_case("rem") {
            let after = trimmed[3..].chars().next();
            if after.is_none() || after.is_some_and(|c| c.is_whitespace()) {
                comments.push(Comment {
                    start_line: idx + 1,
                    text: trimmed[3..].to_string(),
                });
            }
        }
    }
    comments.sort_by_key(|c| c.start_line);
    comments
}

/// Extract `!` line comments (Fortran-style).
fn extract_bang_style(content: &str) -> Vec<Comment> {
    extract_prefixed_line_comments(content, "!")
}

fn extract_prefixed_line_comments(content: &str, prefix: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            comments.push(Comment {
                start_line: idx + 1,
                text: rest.to_string(),
            });
        }
    }
    comments
}

/// Extract only C-style `/* ... */` block comments (no `//` handling).
fn extract_c_block_only(content: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    let mut chars = content.char_indices().peekable();
    let mut line: usize = 1;
    let mut in_string: Option<char> = None;

    while let Some(&(_i, ch)) = chars.peek() {
        if let Some(quote) = in_string {
            chars.next();
            if ch == '\\' {
                if chars.peek().is_some() {
                    let next = chars.next().unwrap().1;
                    if next == '\n' {
                        line += 1;
                    }
                }
            } else if ch == quote {
                in_string = None;
            } else if ch == '\n' {
                line += 1;
            }
            continue;
        }

        match ch {
            '"' | '\'' | '`' => {
                in_string = Some(ch);
                chars.next();
            }
            '/' => {
                chars.next();
                if let Some('*') = chars.peek().map(|&(_, c)| c) {
                    chars.next();
                    let start = match chars.peek() {
                        Some(&(idx, _)) => idx,
                        None => content.len(),
                    };
                    let comment_line = line;
                    let mut end = content.len();
                    let mut found_end = false;
                    while let Some(&(_, c)) = chars.peek() {
                        if c == '\n' {
                            line += 1;
                            chars.next();
                        } else if c == '*' {
                            let star_pos = chars.peek().map(|&(idx, _)| idx).unwrap();
                            chars.next();
                            if let Some(&(_, '/')) = chars.peek() {
                                end = star_pos;
                                chars.next();
                                found_end = true;
                                break;
                            }
                        } else {
                            chars.next();
                        }
                    }
                    if !found_end {
                        end = content.len();
                    }
                    comments.push(Comment {
                        start_line: comment_line,
                        text: clean_block_comment(&content[start..end]),
                    });
                }
            }
            '\n' => {
                line += 1;
                chars.next();
            }
            _ => {
                chars.next();
            }
        }
    }

    comments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_line_comments() {
        let content = "let x = 1; // first\nlet y = 2; // second\n";
        let comments = extract_comments(content, "ts");
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].start_line, 1);
        assert_eq!(comments[0].text, " first");
        assert_eq!(comments[1].start_line, 2);
        assert_eq!(comments[1].text, " second");
    }

    #[test]
    fn test_c_block_comment() {
        let content = "/* hello */\ncode\n";
        let comments = extract_comments(content, "js");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].start_line, 1);
        assert_eq!(comments[0].text, " hello ");
    }

    #[test]
    fn test_c_block_multiline() {
        let content = "/*\n * line1\n * line2\n */\n";
        let comments = extract_comments(content, "js");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].start_line, 1);
        assert!(comments[0].text.contains("line1"));
        assert!(comments[0].text.contains("line2"));
    }

    #[test]
    fn test_hash_comments() {
        let content = "# comment\ncode\n# another\n";
        let comments = extract_comments(content, "py");
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].start_line, 1);
        assert_eq!(comments[0].text, " comment");
        assert_eq!(comments[1].start_line, 3);
        assert_eq!(comments[1].text, " another");
    }

    #[test]
    fn test_unknown_ext_c_style() {
        let content = "// c-style\n";
        let comments = extract_comments(content, "xyz");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, " c-style");
    }

    #[test]
    fn test_unknown_ext_hash_fallback() {
        let content = "# hash\n";
        let comments = extract_comments(content, "xyz");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, " hash");
    }

    #[test]
    fn test_string_not_treated_as_comment() {
        let content = "let s = \"// not a comment\";\n// real comment\n";
        let comments = extract_comments(content, "ts");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].start_line, 2);
        assert_eq!(comments[0].text, " real comment");
    }

    #[test]
    fn test_sql_comments() {
        let content = "-- line\nSELECT 1;\n/* block */\n";
        let comments = extract_comments(content, "sql");
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].text, " line");
        assert_eq!(comments[1].text, " block ");
    }

    #[test]
    fn test_matlab_percent_comments() {
        let content = "% first\nx = 1;\n";
        let comments = extract_comments(content, "m");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, " first");
    }

    #[test]
    fn test_assembly_semicolon_comments() {
        let content = "; note\nMOV AX, BX\n";
        let comments = extract_comments(content, "asm");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, " note");
    }

    #[test]
    fn test_vb_comments() {
        let content = "' a\nREM b\n";
        let comments = extract_comments(content, "vb");
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].text, " a");
        assert_eq!(comments[1].text, " b");
    }

    #[test]
    fn test_fortran_bang_comments() {
        let content = "! hello\nx = 1\n";
        let comments = extract_comments(content, "f90");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, " hello");
    }

    #[test]
    fn test_c_style_escaped_quote_in_string() {
        let content = "let s = \"hello \\\" // nope\";\n// yes\n";
        let comments = extract_comments(content, "js");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].start_line, 2);
    }

    #[test]
    fn test_c_style_string_with_newline_tracking() {
        let content = "\"line1\\\nline2\"\n// comment\n";
        let comments = extract_comments(content, "js");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].start_line, 3);
    }

    #[test]
    fn test_c_style_unclosed_block_comment() {
        let content = "code\n/* unterminated\nstill comment";
        let comments = extract_comments(content, "js");
        assert_eq!(comments.len(), 1);
        assert!(comments[0].text.contains("unterminated"));
    }

    #[test]
    fn test_c_style_slash_non_comment() {
        let content = "a / b\n";
        let comments = extract_comments(content, "js");
        assert!(comments.is_empty());
    }

    #[test]
    fn test_clean_block_comment_star_variants() {
        let content = "/*\n*\n*abc\n*/\n";
        let comments = extract_comments(content, "js");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, "\n\nabc");
    }

    #[test]
    fn test_sql_block_comment_unclosed() {
        let content = "/* sql\nunterminated";
        let comments = extract_comments(content, "sql");
        assert_eq!(comments.len(), 1);
        assert!(comments[0].text.contains("unterminated"));
    }
}
