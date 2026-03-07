/// Result of linting: collected errors and output messages.
pub struct LintResult {
    pub exit_code: i32,
    pub messages: Vec<String>,
    pub verbose_messages: Vec<String>,
}
