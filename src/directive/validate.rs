use crate::model::Directive;

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
