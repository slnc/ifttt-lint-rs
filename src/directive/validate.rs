use crate::model::Directive;

/// Check for duplicate labels across IfChange labels and Label names within a file.
pub fn validate_directive_uniqueness(directives: &[Directive], file_path: &str) -> Vec<String> {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
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

        if !seen.insert(name.to_string()) {
            errors.push(format!(
                "[ifttt] {}:{} -> duplicate directive label '{}'",
                file_path, line, name
            ));
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_uniqueness_no_duplicates() {
        let directives = vec![
            Directive::IfChange {
                line: 1,
                label: Some("a".to_string()),
            },
            Directive::Label {
                line: 5,
                name: "b".to_string(),
            },
        ];
        let errors = validate_directive_uniqueness(&directives, "test.ts");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_uniqueness_duplicate_label() {
        let directives = vec![
            Directive::IfChange {
                line: 1,
                label: Some("a".to_string()),
            },
            Directive::Label {
                line: 5,
                name: "a".to_string(),
            },
        ];
        let errors = validate_directive_uniqueness(&directives, "test.ts");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("duplicate directive label 'a'"));
    }

    #[test]
    fn test_validate_uniqueness_skips_bare_ifchange() {
        let directives = vec![
            Directive::IfChange {
                line: 1,
                label: None,
            },
            Directive::IfChange {
                line: 10,
                label: None,
            },
        ];
        let errors = validate_directive_uniqueness(&directives, "test.ts");
        assert!(errors.is_empty());
    }
}
