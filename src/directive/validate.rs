use crate::model::Directive;

/// Check that IfChange/ThenChange directives are properly paired within a file.
///
/// Detects:
/// - Orphan ThenChange (no preceding IfChange)
/// - Orphan IfChange (no following ThenChange)
/// - Consecutive IfChange without intervening ThenChange
///
/// Note: multi-target ThenChange directives (e.g. `ThenChange("a.py", "b.py")`)
/// are represented as multiple consecutive `Directive::ThenChange` entries sharing
/// the same line number. All of them count as a single "closing" ThenChange.
pub fn validate_directive_pairing(directives: &[Directive], file_path: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let mut pending_if: Option<(usize, Option<&str>)> = None; // (line, label)
    let mut last_then_line: Option<usize> = None;

    for directive in directives {
        match directive {
            Directive::IfChange { line, label } => {
                if let Some((prev_line, prev_label)) = pending_if {
                    let label_ctx = match prev_label {
                        Some(l) => format!(" after IfChange('{}')", l),
                        None => " after IfChange".to_string(),
                    };
                    errors.push(format!(
                        "error: {}:{}: missing ThenChange{}",
                        file_path, prev_line, label_ctx
                    ));
                }
                pending_if = Some((*line, label.as_deref()));
                last_then_line = None;
            }
            Directive::ThenChange { line, .. } => {
                // Multiple ThenChange on the same line = multi-target, part of same pair
                if pending_if.is_none() && last_then_line != Some(*line) {
                    errors.push(format!(
                        "error: {}:{}: unexpected ThenChange without preceding IfChange",
                        file_path, line
                    ));
                }
                pending_if = None;
                last_then_line = Some(*line);
            }
            _ => {}
        }
    }

    if let Some((line, label)) = pending_if {
        let label_ctx = match label {
            Some(l) => format!(" after IfChange('{}')", l),
            None => " after IfChange".to_string(),
        };
        errors.push(format!(
            "error: {}:{}: missing ThenChange{}",
            file_path, line, label_ctx
        ));
    }

    errors
}

/// Check for duplicate labels across IfChange labels and Label names within a file.
pub fn validate_directive_uniqueness(directives: &[Directive], file_path: &str) -> Vec<String> {
    use std::collections::HashSet;

    let mut seen: HashSet<&str> = HashSet::new();
    let mut errors = Vec::new();

    for directive in directives {
        let (name, line) = match directive {
            Directive::IfChange {
                label: Some(label),
                line,
            } => (label.as_str(), *line),
            Directive::Label { name, line } => (name.as_str(), *line),
            _ => continue,
        };

        if !seen.insert(name) {
            errors.push(format!(
                "error: {}:{}: duplicate directive label '{}'",
                file_path, line, name
            ));
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn if_change(line: usize, label: Option<&str>) -> Directive {
        Directive::IfChange {
            line,
            label: label.map(str::to_owned),
        }
    }

    fn label(line: usize, name: &str) -> Directive {
        Directive::Label {
            line,
            name: name.to_owned(),
        }
    }

    #[test]
    fn no_duplicates() {
        let errors =
            validate_directive_uniqueness(&[if_change(1, Some("a")), label(5, "b")], "test.ts");
        assert!(errors.is_empty());
    }

    #[test]
    fn duplicate_label() {
        let errors =
            validate_directive_uniqueness(&[if_change(1, Some("a")), label(5, "a")], "test.ts");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("duplicate directive label 'a'"));
    }

    #[test]
    fn skips_bare_ifchange() {
        let errors =
            validate_directive_uniqueness(&[if_change(1, None), if_change(10, None)], "test.ts");
        assert!(errors.is_empty());
    }
}
