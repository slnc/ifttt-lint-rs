use clap::Parser;
use ignore::WalkBuilder;
use is_terminal::IsTerminal;
use rayon::prelude::*;
use std::io::Read;
use std::process;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::diff::parse_changed_lines;
use crate::directive::{parse_directives_from_content, validate_directive_uniqueness};
use crate::engine::lint_diff;

static COLOR_ENABLED: AtomicBool = AtomicBool::new(false);

fn setup_color() {
    let enabled = std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    COLOR_ENABLED.store(enabled, Ordering::Relaxed);
}

fn red(s: &str) -> String {
    if COLOR_ENABLED.load(Ordering::Relaxed) {
        format!("\x1b[31m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

fn dim(s: &str) -> String {
    if COLOR_ENABLED.load(Ordering::Relaxed) {
        format!("\x1b[2m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

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

    /// Show directive pairs and validation summary
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Show internal processing details (implies --verbose)
    #[arg(long = "debug")]
    pub debug: bool,

    /// Number of parallel tasks (0 for auto-detect based on CPU cores)
    #[arg(short = 'j', long = "jobs", default_value = "0")]
    pub jobs: usize,

    /// Ignore specified file or file#label during linting (repeatable)
    #[arg(short = 'i', long = "ignore")]
    pub ignore: Vec<String>,

    /// Scan directory for LINT directive errors (default: current directory)
    #[arg(short = 's', long = "scan")]
    pub scan: Option<String>,

    /// Skip directive syntax scan
    #[arg(long = "no-scan")]
    pub no_scan: bool,

    /// Skip diff-based lint
    #[arg(long = "no-lint")]
    pub no_lint: bool,
}

pub fn run(cli: Cli) -> i32 {
    setup_color();

    if cli.no_scan && cli.no_lint {
        eprintln!(
            "{} --no-scan and --no-lint cannot both be set",
            red("Error:")
        );
        return 2;
    }

    let verbose = cli.verbose || cli.debug;
    let debug = cli.debug;

    if cli.jobs > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(cli.jobs)
            .build_global()
            .ok();
    }

    let mut exit_code = 0;

    // Scan phase: validate directive syntax across a directory.
    if !cli.no_scan {
        let scan_dir = cli.scan.as_deref().unwrap_or(".");
        let scan_result = run_scan(scan_dir, verbose, debug);
        exit_code = exit_code.max(scan_result);
    }

    // Lint phase: validate cross-file dependencies from a diff.
    if !cli.no_lint {
        let diff_text = match cli.diff_file.as_deref() {
            Some(path) if path != "-" => match std::fs::read_to_string(path) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("{} {}", red("Error:"), e);
                    return 2;
                }
            },
            _ => {
                let mut buf = String::new();
                if let Err(e) = std::io::stdin().read_to_string(&mut buf) {
                    eprintln!("{} reading stdin: {}", red("Error:"), e);
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
                        "{} Invalid diff input: no file changes detected (snippet: \"{}...\")",
                        red("Error:"),
                        snippet
                    );
                    return 2;
                }
            }
        }

        if debug {
            let n = if cli.jobs > 0 {
                cli.jobs
            } else {
                rayon::current_num_threads()
            };
            eprintln!("{}", dim(&format!("Parallelism: {}", n)));
        }

        let result = lint_diff(&diff_text, verbose, debug, &cli.ignore);

        for msg in &result.verbose_messages {
            eprintln!("{}", dim(msg));
        }

        for msg in &result.messages {
            eprintln!("{}", red(msg));
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

fn run_scan(dir: &str, verbose: bool, debug: bool) -> i32 {
    let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let file_count = AtomicUsize::new(0);
    let directive_pair_count = AtomicUsize::new(0);

    let entries: Vec<_> = WalkBuilder::new(dir)
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_some_and(|ft| ft.is_file()))
        .collect();

    entries.par_iter().for_each(|entry| {
        let path = entry.path();
        let file_path = path.to_string_lossy().to_string();

        if debug {
            eprintln!("{}", dim(&format!("Validating file: {}", file_path)));
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return,
        };

        if !content.contains("LINT.") {
            return;
        }

        file_count.fetch_add(1, Ordering::Relaxed);

        match parse_directives_from_content(&content, &file_path) {
            Ok(directives) => {
                let pair_count = directives
                    .iter()
                    .filter(|d| matches!(d, crate::model::Directive::IfChange { .. }))
                    .count();
                directive_pair_count.fetch_add(pair_count, Ordering::Relaxed);

                if verbose && pair_count > 0 {
                    eprintln!(
                        "{}",
                        dim(&format!(
                            "[ifttt] {}: {} directive {}",
                            file_path,
                            pair_count,
                            if pair_count == 1 { "pair" } else { "pairs" }
                        ))
                    );
                }

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
        eprintln!("{}", red(err));
    }

    if verbose {
        let files = file_count.load(Ordering::Relaxed);
        let pairs = directive_pair_count.load(Ordering::Relaxed);
        let errs = errors.len();
        eprintln!(
            "{}",
            dim(&format!(
                "[ifttt] scanned {} {}, {} directive {}, {} {}",
                files,
                if files == 1 { "file" } else { "files" },
                pairs,
                if pairs == 1 { "pair" } else { "pairs" },
                errs,
                if errs == 1 { "error" } else { "errors" },
            ))
        );
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
