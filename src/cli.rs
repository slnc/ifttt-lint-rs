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
use crate::engine::{find_repo_root, lint_diff, normalize_path_str, split_target_label};

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
    name = "ifchange",
    version,
    about = "Linter for enforcing conditional change directives",
    after_help = "\
Directives:
  Mark related code with comment directives (use your language's comment style):

  # LINT.IfChange                             Open a guarded section
  env:
    DATABASE_URL: postgres://prod:5432/myapp
  # LINT.ThenChange(src/config.py#env)        Close it; list files that must co-change

  # LINT.Label(env)                           Named target section
  DATABASE_URL = os.environ[\"DATABASE_URL\"]
  # LINT.EndLabel

  Multiple targets: LINT.ThenChange(a.py#foo, b.py)
  Self-reference:   LINT.ThenChange(#other-label)

Examples:
  git diff HEAD~1 | ifchange                  Lint a diff from stdin
  ifchange changes.diff                       Lint a diff file
  ifchange --no-lint                          Validate directive syntax only
  ifchange --no-lint -s ./src                 Scan a specific directory
  git diff HEAD~1 | ifchange --no-scan        Lint only, skip syntax scan
  ifchange -i '**/*.sql' f.diff               Ignore files matching a glob
  ifchange -i 'config.toml#db' f.diff         Ignore a labeled section

Exit codes: 0 = ok, 1 = lint errors, 2 = fatal error"
)]
pub struct Cli {
    /// Diff file path, or '-' / omit to read from stdin
    pub diff_file: Option<String>,

    /// Warn on lint errors but exit with code 0
    #[arg(short = 'w', long = "warn")]
    pub warn: bool,

    /// Show processing details and validation summary
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Show per-file processing details (implies --verbose)
    #[arg(short = 'd', long = "debug")]
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

    let debug = cli.debug;
    let verbose = cli.verbose || debug;

    // Discover repo root for resolving repo-relative paths.
    let cwd = std::env::current_dir().ok();
    let repo_root = cwd.as_ref().and_then(|c| find_repo_root(c));

    if verbose {
        if let Some(ref root) = repo_root {
            if cwd.as_ref() == Some(root) {
                eprintln!("{}", dim("repo root: ."));
            } else {
                eprintln!("{}", dim(&format!("repo root: {}", root.display())));
            }
        }
    }

    let root = repo_root
        .or(cwd)
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    run_inner(cli, verbose, debug, &root)
}

fn run_inner(cli: Cli, verbose: bool, debug: bool, repo_root: &std::path::Path) -> i32 {
    if cli.jobs > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(cli.jobs)
            .build_global()
            .ok();
    }

    if verbose {
        let n = if cli.jobs > 0 {
            cli.jobs
        } else {
            rayon::current_num_threads()
        };
        eprintln!("{}", dim(&format!("jobs: {}", n)));
        if debug {
            eprintln!();
        }
    }

    let mut exit_code = 0;
    let mut scan_errors = 0usize;
    let mut lint_errors = 0usize;
    // Scan phase: validate directive syntax across a directory.
    if !cli.no_scan {
        let scan_dir = cli.scan.as_deref().unwrap_or(".");
        let scan_root = if cli.scan.is_some() {
            let scan_path = std::path::Path::new(scan_dir);
            let scan_abs = if scan_path.is_absolute() {
                scan_path.to_path_buf()
            } else {
                repo_root.join(scan_path)
            };
            find_repo_root(&scan_abs).unwrap_or(scan_abs)
        } else {
            repo_root.to_path_buf()
        };
        let (scan_exit, scan_err_count) = run_scan(scan_dir, verbose, debug, &scan_root);
        scan_errors = scan_err_count;
        exit_code = exit_code.max(scan_exit);
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

        let result = lint_diff(&diff_text, verbose, debug, &cli.ignore, repo_root);

        lint_errors = result.messages.len();

        // Verbose: lint header
        if verbose {
            eprintln!();
            if debug {
                eprintln!();
            }
            let header = if result.files_checked > 0 {
                format!(
                    "Lint summary: {} {} checked, {} {} in diff",
                    result.files_checked,
                    if result.files_checked == 1 {
                        "file"
                    } else {
                        "files"
                    },
                    result.pairs_checked,
                    if result.pairs_checked == 1 {
                        "pair"
                    } else {
                        "pairs"
                    },
                )
            } else {
                format!(
                    "Lint summary: {} {} in diff",
                    result.pairs_checked,
                    if result.pairs_checked == 1 {
                        "pair"
                    } else {
                        "pairs"
                    },
                )
            };
            if lint_errors > 0 {
                let error_part = format!(
                    ", {} {}",
                    lint_errors,
                    if lint_errors == 1 { "error" } else { "errors" },
                );
                eprintln!("{}{}", dim(&header), red(&error_part));
            } else {
                eprintln!("{}", dim(&header));
            }

            for msg in &result.verbose_messages {
                eprintln!("{}", dim(msg));
            }
        }

        // Errors at column 0
        if verbose && !result.messages.is_empty() {
            eprintln!();
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

    // Final error summary line
    let total_errors = scan_errors + lint_errors;
    if total_errors > 0 {
        let mut parts: Vec<String> = Vec::new();
        if scan_errors > 0 {
            parts.push(format!("{} scan", scan_errors));
        }
        if lint_errors > 0 {
            parts.push(format!("{} lint", lint_errors));
        }
        eprintln!(
            "\n{}",
            red(&format!(
                "found {} {} ({})",
                total_errors,
                if total_errors == 1 { "error" } else { "errors" },
                parts.join(", ")
            ))
        );
    }

    exit_code
}

fn validate_thenchange_targets(
    directives: &[crate::model::Directive],
    parent: &std::path::Path,
    repo_root: &std::path::Path,
    file_path: &str,
) -> Vec<String> {
    let mut errors = Vec::new();
    for d in directives {
        if let crate::model::Directive::ThenChange { line, target } = d {
            let (target_name, label) = split_target_label(target);
            if target_name.is_empty() {
                continue; // self-reference
            }
            let resolved = if let Some(stripped) = target_name.strip_prefix('/') {
                let normalized = normalize_path_str(stripped.trim_start_matches('/'));
                repo_root.join(normalized)
            } else {
                parent.join(target_name)
            };
            if target_name.ends_with('/') {
                // Directory target
                if label.is_some() {
                    errors.push(format!(
                        "error: {}:{}: labels are not supported for directory targets ('{}')",
                        file_path, line, target_name
                    ));
                } else if !resolved.is_dir() {
                    errors.push(format!(
                        "error: {}:{}: ThenChange target '{}' does not exist",
                        file_path, line, target_name
                    ));
                }
            } else if resolved.is_dir() {
                errors.push(format!(
                    "error: {}:{}: ThenChange target '{}' is a directory; add trailing '/' if intentional",
                    file_path, line, target_name
                ));
            } else if !resolved.exists() {
                errors.push(format!(
                    "error: {}:{}: ThenChange target '{}' does not exist",
                    file_path, line, target_name
                ));
            }
        }
    }
    errors
}

fn run_scan(dir: &str, verbose: bool, debug: bool, repo_root: &std::path::Path) -> (i32, usize) {
    let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let verbose_lines: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let file_count = AtomicUsize::new(0);
    let directive_pair_count = AtomicUsize::new(0);
    let label_count = AtomicUsize::new(0);

    let entries: Vec<_> = WalkBuilder::new(dir)
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_some_and(|ft| ft.is_file()))
        .collect();

    let walked_count = entries.len();

    entries.par_iter().for_each(|entry| {
        let path = entry.path();
        let file_path = path.to_string_lossy().to_string();

        if debug {
            eprintln!("{}", dim(&format!("Scanning: {}", file_path)));
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return,
        };

        if !content.as_bytes().windows(6).any(|w| {
            w[..5].eq_ignore_ascii_case(b"LINT.")
                && matches!(w[5] | 0x20, b'i' | b't' | b'l' | b'e')
        }) {
            return;
        }

        file_count.fetch_add(1, Ordering::Relaxed);

        match parse_directives_from_content(&content, &file_path) {
            Ok(directives) => {
                let pair_count = directives
                    .iter()
                    .filter(|d| matches!(d, crate::model::Directive::IfChange { .. }))
                    .count();
                let lbl_count = directives
                    .iter()
                    .filter(|d| matches!(d, crate::model::Directive::Label { .. }))
                    .count();
                directive_pair_count.fetch_add(pair_count, Ordering::Relaxed);
                label_count.fetch_add(lbl_count, Ordering::Relaxed);

                if verbose && (pair_count > 0 || lbl_count > 0) {
                    let mut parts = Vec::new();
                    if pair_count > 0 {
                        parts.push(format!(
                            "{} {}",
                            pair_count,
                            if pair_count == 1 { "pair" } else { "pairs" }
                        ));
                    }
                    if lbl_count > 0 {
                        parts.push(format!(
                            "{} {}",
                            lbl_count,
                            if lbl_count == 1 { "label" } else { "labels" }
                        ));
                    }
                    verbose_lines.lock().unwrap().push(format!(
                        "  {}: {}",
                        file_path,
                        parts.join(", ")
                    ));
                }

                let dup_errors = validate_directive_uniqueness(&directives, &file_path);
                if !dup_errors.is_empty() {
                    let mut errs = errors.lock().unwrap();
                    for err in dup_errors {
                        errs.push(err);
                    }
                }

                let parent = path.parent().unwrap_or(std::path::Path::new("."));
                let target_errors =
                    validate_thenchange_targets(&directives, parent, repo_root, &file_path);
                if !target_errors.is_empty() {
                    let mut errs = errors.lock().unwrap();
                    errs.extend(target_errors);
                }
            }
            Err(e) => {
                errors.lock().unwrap().push(e.to_string());
            }
        }
    });

    let errors = errors.into_inner().unwrap();
    let verbose_lines = verbose_lines.into_inner().unwrap();
    let err_count = errors.len();

    if verbose {
        eprintln!();
        if debug {
            eprintln!();
        }
        let files = file_count.load(Ordering::Relaxed);
        let pairs = directive_pair_count.load(Ordering::Relaxed);
        let labels = label_count.load(Ordering::Relaxed);
        let mut parts = vec![format!(
            "Scan summary: {} files walked ({} with directives), {} directive {}",
            walked_count,
            files,
            pairs,
            if pairs == 1 { "pair" } else { "pairs" },
        )];
        parts.push(format!(
            "{} {}",
            labels,
            if labels == 1 { "label" } else { "labels" }
        ));
        let header = parts.join(", ");
        for line in &verbose_lines {
            eprintln!("{}", dim(line));
        }

        if err_count > 0 {
            let error_part = format!(
                ", {} {}",
                err_count,
                if err_count == 1 { "error" } else { "errors" },
            );
            eprintln!("{}{}", dim(&header), red(&error_part));
        } else {
            eprintln!("{}", dim(&header));
        }
    }

    // Errors at column 0
    if verbose && !errors.is_empty() {
        eprintln!();
    }
    for err in &errors {
        eprintln!("{}", red(err));
    }

    let exit_code = if errors.is_empty() { 0 } else { 1 };
    (exit_code, err_count)
}

pub fn run_from_env() -> ! {
    let cli = Cli::parse();
    process::exit(run(cli));
}
