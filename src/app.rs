use clap::Parser;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::io::Read;
use std::process;
use std::sync::Mutex;

use crate::diff::parse_changed_lines;
use crate::directive::{parse_directives_from_content, validate_directive_uniqueness};
use crate::engine::lint_diff;

#[derive(Parser)]
#[command(
    name = "ifttt-lint",
    about = "IFTTT lint checker for enforcing conditional change directives"
)]
pub struct Cli {
    /// Diff file path, or '-' / omit to read from stdin
    pub diff_file: Option<String>,

    /// Warn on lint errors but exit with code 0
    #[arg(short = 'w', long = "warn")]
    pub warn: bool,

    /// Show verbose logging (files being processed)
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Number of parallel tasks (0 for auto-detect based on CPU cores)
    #[arg(short = 'p', long = "parallelism", default_value = "0")]
    pub parallelism: usize,

    /// Ignore specified file or file#label during linting (repeatable)
    #[arg(short = 'i', long = "ignore")]
    pub ignore: Vec<String>,

    /// Check directory for LINT directive errors (default: current directory)
    #[arg(short = 'c', long = "check")]
    pub check: Option<String>,

    /// Skip directive syntax check
    #[arg(long = "no-check")]
    pub no_check: bool,

    /// Skip diff-based lint
    #[arg(long = "no-lint")]
    pub no_lint: bool,
}

pub fn run(cli: Cli) -> i32 {
    if cli.no_check && cli.no_lint {
        eprintln!("Error: --no-check and --no-lint cannot both be set");
        return 2;
    }

    if cli.parallelism > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(cli.parallelism)
            .build_global()
            .ok();
    }

    let mut exit_code = 0;

    // Check phase: validate directive syntax across a directory.
    if !cli.no_check {
        let check_dir = cli.check.as_deref().unwrap_or(".");
        let check_result = run_check(check_dir, cli.verbose);
        exit_code = exit_code.max(check_result);
    }

    // Lint phase: validate cross-file dependencies from a diff.
    if !cli.no_lint {
        let diff_text = match cli.diff_file.as_deref() {
            Some(path) if path != "-" => match std::fs::read_to_string(path) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return 2;
                }
            },
            _ => {
                let mut buf = String::new();
                if let Err(e) = std::io::stdin().read_to_string(&mut buf) {
                    eprintln!("Error reading stdin: {}", e);
                    return 2;
                }
                buf
            }
        };

        // Validate diff input
        let trimmed = diff_text.trim();
        if !trimmed.is_empty() {
            let changes = parse_changed_lines(&diff_text);
            if changes.is_empty() {
                let has_valid = trimmed.contains("---")
                    || trimmed.contains("diff --git")
                    || trimmed.contains("index ");
                if !has_valid {
                    let snippet: String = trimmed.chars().take(100).collect();
                    let snippet = snippet.replace('\n', "\\n");
                    eprintln!(
                        "Error: Invalid diff input: no file changes detected (snippet: \"{}...\")",
                        snippet
                    );
                    return 2;
                }
            }
        }

        if cli.verbose {
            let n = if cli.parallelism > 0 {
                cli.parallelism
            } else {
                rayon::current_num_threads()
            };
            eprintln!("Parallelism: {}", n);
        }

        let result = lint_diff(&diff_text, cli.verbose, &cli.ignore);

        for msg in &result.messages {
            println!("{}", msg);
        }

        let lint_exit = if cli.warn && result.exit_code == 1 {
            0
        } else {
            result.exit_code
        };
        exit_code = exit_code.max(lint_exit);
    }

    exit_code
}

fn run_check(dir: &str, verbose: bool) -> i32 {
    let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());

    let entries: Vec<_> = WalkBuilder::new(dir)
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_some_and(|ft| ft.is_file()))
        .collect();

    entries.par_iter().for_each(|entry| {
        let path = entry.path();
        let file_path = path.to_string_lossy().to_string();

        if verbose {
            eprintln!("Validating file: {}", file_path);
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return,
        };

        if !content.contains("LINT.") {
            return;
        }

        match parse_directives_from_content(&content, &file_path) {
            Ok(directives) => {
                let dup_errors = validate_directive_uniqueness(&directives, &file_path);
                if !dup_errors.is_empty() {
                    let mut errs = errors.lock().unwrap();
                    for err in dup_errors {
                        errs.push(err);
                    }
                }
            }
            Err(e) => {
                errors.lock().unwrap().push(e.to_string());
            }
        }
    });

    let errors = errors.into_inner().unwrap();
    for err in &errors {
        eprintln!("{}", err);
    }

    if errors.is_empty() {
        0
    } else {
        1
    }
}

pub fn run_from_env() -> ! {
    let cli = Cli::parse();
    process::exit(run(cli));
}
