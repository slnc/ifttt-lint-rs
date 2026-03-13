use std::collections::{HashMap, HashSet};

use crate::model::FileChanges;

/// Decode C-style octal escape sequences (e.g., `\360\237\224\216`) into UTF-8
/// bytes and then into a String. Non-escape characters pass through unchanged.
fn decode_octal_escapes(s: &str) -> String {
    let src = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(src.len());
    let mut i = 0;

    while i < src.len() {
        if src[i] == b'\\' && i + 3 < src.len() {
            let d1 = src[i + 1];
            let d2 = src[i + 2];
            let d3 = src[i + 3];
            if d1.is_ascii_digit() && d2.is_ascii_digit() && d3.is_ascii_digit() {
                if let Ok(val) =
                    u8::from_str_radix(std::str::from_utf8(&src[i + 1..i + 4]).unwrap(), 8)
                {
                    out.push(val);
                    i += 4;
                    continue;
                }
            }
        }
        out.push(src[i]);
        i += 1;
    }

    String::from_utf8_lossy(&out).into_owned()
}

/// Strip surrounding double quotes from a path if present.
fn strip_quotes(s: &str) -> &str {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Strip a single-character prefix directory (e.g., `a/path` -> `path`,
/// `b/path` -> `path`). Only strips if the second character is '/'.
fn strip_prefix_dir(s: &str) -> &str {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b'/' {
        &s[2..]
    } else {
        s
    }
}

/// Determine if a `---` or `+++` line contains a valid file path with a
/// recognized single-char prefix directory.
fn has_valid_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[1] == b'/'
}

/// Parse a unified diff text and return a map from file path to the set of
/// added and removed line numbers.
///
/// Parsing strategy:
/// 1. Split diff into lines.
/// 2. Filter out `diff ` header lines.
/// 3. Filter out spurious `--- ` / `+++ ` lines that lack a valid prefix path.
/// 4. Pair remaining `---` / `+++` lines to identify file entries.
/// 5. Skip deleted files (target is `/dev/null`).
/// 6. Decode octal escapes and strip prefix dirs from paths.
/// 7. Parse `@@ ... @@` hunk headers and accumulate added/removed lines.
pub fn parse_changed_lines(diff_text: &str) -> HashMap<String, FileChanges> {
    let mut result: HashMap<String, FileChanges> = HashMap::new();

    // Filter out main diff header lines and spurious ---/+++ lines.
    let lines: Vec<&str> = diff_text
        .lines()
        .filter(|line| {
            if line.starts_with("diff ") {
                return false;
            }
            if let Some(rest) = line.strip_prefix("--- ") {
                let path = strip_quotes(rest);
                // Allow /dev/null
                if path == "/dev/null" {
                    return true;
                }
                return has_valid_prefix(path);
            }
            if let Some(rest) = line.strip_prefix("+++ ") {
                let path = strip_quotes(rest);
                if path == "/dev/null" {
                    return true;
                }
                return has_valid_prefix(path);
            }
            true
        })
        .collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        // Look for a --- line to start a file entry.
        if line.starts_with("--- ") {
            // Expect the next line to be +++.
            if i + 1 < lines.len() && lines[i + 1].starts_with("+++ ") {
                let plus_line = lines[i + 1];
                let plus_path_raw = if let Some(rest) = plus_line.strip_prefix("+++ ") {
                    strip_quotes(rest)
                } else {
                    i += 1;
                    continue;
                };

                // Skip deleted files.
                if plus_path_raw == "/dev/null" {
                    i += 2;
                    continue;
                }

                let decoded = decode_octal_escapes(plus_path_raw);
                let file_path = strip_prefix_dir(&decoded).trim_end().to_string();

                // Move past the --- and +++ lines.
                i += 2;

                // Parse hunks for this file.
                let mut added_lines: HashSet<usize> = HashSet::new();
                let mut removed_lines: HashSet<usize> = HashSet::new();
                let mut addition_new_lines: HashSet<usize> = HashSet::new();
                let mut removal_new_lines: Vec<(usize, String)> = Vec::new();

                while i < lines.len() {
                    let hunk_line = lines[i];

                    // Stop if we hit the next file entry.
                    if hunk_line.starts_with("--- ") {
                        break;
                    }

                    // Parse hunk header.
                    if hunk_line.starts_with("@@ ") {
                        let (old_start, new_start) = parse_hunk_header(hunk_line);
                        let mut old_line = old_start;
                        let mut new_line = new_start;

                        i += 1;

                        while i < lines.len() {
                            let content_line = lines[i];

                            // Stop at next hunk, next file, or other control lines.
                            if content_line.starts_with("@@ ") || content_line.starts_with("--- ") {
                                break;
                            }

                            // Skip "\ No newline at end of file" and similar
                            if content_line.starts_with('\\') {
                                i += 1;
                                continue;
                            }

                            if content_line.starts_with('+') {
                                added_lines.insert(new_line);
                                addition_new_lines.insert(new_line);
                                new_line += 1;
                            } else if let Some(stripped) = content_line.strip_prefix('-') {
                                removed_lines.insert(old_line);
                                removal_new_lines.push((new_line, stripped.to_string()));
                                old_line += 1;
                            } else {
                                // Context line (starts with ' ' or is empty for
                                // blank context lines).
                                old_line += 1;
                                new_line += 1;
                            }

                            i += 1;
                        }

                        continue;
                    }

                    // Skip non-hunk lines (e.g., "Binary files differ", index
                    // lines, etc.)
                    i += 1;
                }

                // Merge into result (a file may appear in multiple diff sections).
                let entry = result
                    .entry(file_path.clone())
                    .or_insert_with(|| FileChanges {
                        added_lines: HashSet::new(),
                        removed_lines: HashSet::new(),
                        addition_new_lines: HashSet::new(),
                        removal_new_lines: Vec::new(),
                    });
                entry.added_lines.extend(added_lines);
                entry.removed_lines.extend(removed_lines);
                entry.addition_new_lines.extend(addition_new_lines);
                entry.removal_new_lines.extend(removal_new_lines);

                continue;
            }
        }

        i += 1;
    }

    result
}

/// Parse a hunk header like `@@ -10,5 +20,8 @@` and return (old_start, new_start).
fn parse_hunk_header(line: &str) -> (usize, usize) {
    // Format: @@ -old_start[,old_count] +new_start[,new_count] @@
    let mut old_start: usize = 1;
    let mut new_start: usize = 1;

    // Find the content between the @@ markers.
    if let Some(rest) = line.strip_prefix("@@ ") {
        // Find the closing @@.
        let header = if let Some(idx) = rest.find(" @@") {
            &rest[..idx]
        } else {
            rest
        };

        for part in header.split_whitespace() {
            if let Some(stripped) = part.strip_prefix('-') {
                // Parse old range: -start[,count]
                if let Some(comma_idx) = stripped.find(',') {
                    old_start = stripped[..comma_idx].parse().unwrap_or(1);
                } else {
                    old_start = stripped.parse().unwrap_or(1);
                }
            } else if let Some(stripped) = part.strip_prefix('+') {
                // Parse new range: +start[,count]
                if let Some(comma_idx) = stripped.find(',') {
                    new_start = stripped[..comma_idx].parse().unwrap_or(1);
                } else {
                    new_start = stripped.parse().unwrap_or(1);
                }
            }
        }
    }

    (old_start, new_start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_octal_emoji() {
        assert_eq!(decode_octal_escapes(r"\360\237\224\216"), "\u{1F50E}");
    }

    #[test]
    fn decode_octal_mixed() {
        assert_eq!(
            decode_octal_escapes(r"path/to/\303\251file.txt"),
            "path/to/\u{00E9}file.txt"
        );
    }

    #[test]
    fn decode_octal_invalid_digits_kept() {
        assert_eq!(decode_octal_escapes(r"\398.txt"), r"\398.txt");
    }

    #[test]
    fn strip_quotes_variants() {
        assert_eq!(strip_quotes("\"hello\""), "hello");
        assert_eq!(strip_quotes("hello"), "hello");
        assert_eq!(strip_quotes("\"\""), "");
    }

    #[test]
    fn strip_prefix_dir_variants() {
        assert_eq!(strip_prefix_dir("a/src/main.rs"), "src/main.rs");
        assert_eq!(strip_prefix_dir("b/src/main.rs"), "src/main.rs");
        assert_eq!(strip_prefix_dir("src/main.rs"), "src/main.rs");
    }

    #[test]
    fn hunk_header_parsing() {
        assert_eq!(parse_hunk_header("@@ -10,5 +20,8 @@"), (10, 20));
        assert_eq!(parse_hunk_header("@@ -1 +1 @@"), (1, 1));
        assert_eq!(
            parse_hunk_header("@@ -100,3 +200,7 @@ fn main()"),
            (100, 200)
        );
        assert_eq!(parse_hunk_header("@@ -7,1 +9,2"), (7, 9));
    }

    #[test]
    fn simple_diff() {
        let diff = "\
diff --git a/foo.txt b/foo.txt
index abc1234..def5678 100644
--- a/foo.txt
+++ b/foo.txt
@@ -1,3 +1,4 @@
 line1
+added
 line2
 line3
";
        let result = parse_changed_lines(diff);
        assert_eq!(result.len(), 1);
        let changes = result.get("foo.txt").unwrap();
        assert!(changes.added_lines.contains(&2));
        assert!(changes.removed_lines.is_empty());
    }

    #[test]
    fn deleted_file_skipped() {
        let diff = "\
diff --git a/removed.txt b/removed.txt
deleted file mode 100644
--- a/removed.txt
+++ /dev/null
@@ -1,2 +0,0 @@
-line1
-line2
";
        assert!(parse_changed_lines(diff).is_empty());
    }

    #[test]
    fn multiple_files() {
        let diff = "\
diff --git a/a.txt b/a.txt
--- a/a.txt
+++ b/a.txt
@@ -1,2 +1,3 @@
 line1
+new
 line2
diff --git a/b.txt b/b.txt
--- a/b.txt
+++ b/b.txt
@@ -1,3 +1,2 @@
 line1
-removed
 line3
";
        let result = parse_changed_lines(diff);
        assert_eq!(result.len(), 2);
        assert!(result.get("a.txt").unwrap().added_lines.contains(&2));
        assert!(result.get("b.txt").unwrap().removed_lines.contains(&2));
    }

    #[test]
    fn binary_file_skipped_gracefully() {
        let diff = "\
diff --git a/image.png b/image.png
--- a/image.png
+++ b/image.png
Binary files a/image.png and b/image.png differ
";
        let result = parse_changed_lines(diff);
        if let Some(changes) = result.get("image.png") {
            assert!(changes.added_lines.is_empty());
            assert!(changes.removed_lines.is_empty());
        }
    }

    #[test]
    fn quoted_path_with_octal() {
        let diff = "\
diff --git a/file b/file
--- \"a/caf\\303\\251.txt\"
+++ \"b/caf\\303\\251.txt\"
@@ -1,2 +1,3 @@
 line1
+added
 line2
";
        assert!(parse_changed_lines(diff).contains_key("caf\u{00E9}.txt"));
    }

    #[test]
    fn hunk_no_newline_marker_ignored() {
        let diff = "\
--- a/foo.txt
+++ b/foo.txt
@@ -1 +1 @@
-line1
+line1
\\ No newline at end of file
";
        let changes = parse_changed_lines(diff).remove("foo.txt").unwrap();
        assert!(changes.added_lines.contains(&1));
        assert!(changes.removed_lines.contains(&1));
    }

    #[test]
    fn unmatched_minus_header_skipped() {
        let diff = "\
--- a/foo.txt
@@ -1 +1 @@
-a
+b
";
        assert!(parse_changed_lines(diff).is_empty());
    }

    #[test]
    fn adversarial_input_does_not_crash() {
        // Null bytes in diff content
        let null_diff = "--- a/x.txt\n+++ b/x.txt\n@@ -1 +1 @@\n-\x00old\n+\x00new\n";
        let _ = parse_changed_lines(null_diff);

        // Completely nonsensical input (valid UTF-8, no structure)
        let garbage = "\x00\x01\x02\x03\x04\x05\x06\x07\n\x0b\x0c\x0e\x0f\n";
        assert!(parse_changed_lines(garbage).is_empty());

        // Truncated hunk header
        let truncated = "--- a/f.txt\n+++ b/f.txt\n@@ -\n";
        let _ = parse_changed_lines(truncated);

        // Huge line numbers in hunk header
        let huge = "--- a/f.txt\n+++ b/f.txt\n@@ -999999999,1 +999999999,1 @@\n-a\n+b\n";
        let _ = parse_changed_lines(huge);

        // Multi-byte UTF-8 everywhere
        let unicode =
            "--- a/\u{1F4A9}.txt\n+++ b/\u{1F4A9}.txt\n@@ -1 +1 @@\n-\u{FFFF}\n+\u{10FFFF}\n";
        let _ = parse_changed_lines(unicode);

        // Empty lines and just whitespace
        let empty = "\n\n\n   \n\t\n";
        assert!(parse_changed_lines(empty).is_empty());
    }

    // BUG 2: git appends trailing tab to ---/+++ lines for paths with spaces.
    #[test]
    fn trailing_tab_stripped_from_filenames() {
        let diff = "--- a/my dir/file.py\t\n+++ b/my dir/file.py\t\n@@ -1 +1 @@\n-old\n+new\n";
        let result = parse_changed_lines(diff);
        assert!(
            result.contains_key("my dir/file.py"),
            "should strip trailing tab; keys: {:?}",
            result.keys().collect::<Vec<_>>()
        );
    }
}
