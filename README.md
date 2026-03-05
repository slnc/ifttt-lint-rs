# lint-ifchange

**Keep related files in sync — automatically catch forgotten co-changes in pull requests.**

Ever renamed a field in `schema.sql` but forgot to update the ORM model? Changed a constant in one file while its copy in another went stale? These cross-file dependencies are invisible to compilers and easy to miss in code review. `lint-ifchange` is a file dependency linter that enforces co-changes across files so that related code never drifts out of sync.

Add lightweight comment directives to mark related sections. When a guarded block changes in a PR, the linter verifies that all referenced files were also modified — catching config drift, forgotten updates, and out-of-sync files before they reach production.

Based on Google's internal LINT.IfChange/ThenChange system. Language-agnostic, works with any file type that supports comments. Inspired by [ebrevdo/ifttt-lint](https://github.com/ebrevdo/ifttt-lint).

## Install

```bash
cargo install --path .
```

## Usage

Pipe a diff from your version control system or pass a diff file directly.

```bash
# Pipe a diff
git diff HEAD~1 | lint-ifchange

# Or pass a file
lint-ifchange changes.diff

# Check mode: validate directive syntax across a directory
lint-ifchange -c ./src

# Ignore files or labels
lint-ifchange -i '*.json' -i 'config.toml#db' changes.diff
```

| Flag | Description |
|------|-------------|
| `-w, --warn` | Warn instead of failing (exit 0) |
| `-v, --verbose` | Verbose logging to stderr |
| `-p, --parallelism <N>` | Thread count (0 = auto) |
| `-i, --ignore <pattern>` | Ignore file or file#label (repeatable, globs) |
| `-c, --check <dir>` | Check directory for directive errors |

Exit codes: **0** ok, **1** lint errors, **2** fatal error.

## Directive Syntax

Directives live in comments. Supported in 50+ file extensions (C-style `//`, `#`, `--`, `%`, `;`, `'`, `!`) — polyglot by design.

### Basic

```python
# LINT.IfChange
VALUE = 42
# LINT.ThenChange("constants.py")
```

### Labeled regions

```python
# LINT.IfChange("feature")
FEATURE_FLAG = True
# LINT.ThenChange("config.py#feature")
```

```python
# config.py
# LINT.Label("feature")
feature_enabled = true
# LINT.EndLabel
```

### Multiple targets

```text
// LINT.ThenChange([
//   "constants.ts",
//   "config.py#db",
//   "schema.sql",
// ])
```

### Self-references

```python
# LINT.ThenChange("#label1")  # target in same file
```

### Cross-references

When two files reference each other, only changes *within* an `IfChange` block trigger validation, not changes elsewhere in the file.

## CI / Automation

Use as a pre-commit hook, CI lint step, or GitHub Actions check to enforce cross-file consistency in every pull request. Ready-to-copy templates are in [examples/](examples/README.md).

Quick pre-commit setup:

```bash
cp examples/hooks/pre-commit.ifttt-lint.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

## Performance

5000 files, 21 language types, 12-core x86_64:

| Mode | Rust | TypeScript | Speedup |
|------|-----:|----------:|---------:|
| **Lint** | **28 ms** | 714 ms | **~25x** |
| **Check** | **15 ms** | 387 ms | **~26x** |

## Testing

```bash
cargo test          # 28 unit + 19 integration
cargo bench         # criterion benchmarks
```

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## License

MIT
