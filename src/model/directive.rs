/// The kinds of lint directives supported in source files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Directive {
    IfChange { line: usize, label: Option<String> },
    ThenChange { line: usize, target: String },
    Label { line: usize, name: String },
    EndLabel { line: usize },
}
