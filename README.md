# lint-ifchange

**Keep related files in sync. Automatically catch forgotten correlated changes in pull requests.**

Ever renamed a field in `schema.sql` but forgot to update the ORM model? Changed a constant in one file while its copy in another went stale? These cross-file dependencies are invisible to compilers and easy to miss in code review. `lint-ifchange` is a file dependency linter that enforces co-changes across files so that related code never drifts out of sync.

Add lightweight comment directives to mark related sections. When a guarded block changes in a PR, the linter verifies that all referenced files were also modified, catching config drift, forgotten updates, and out-of-sync files before they reach production.

Based on Google's internal LINT.IfChange/ThenChange system. Supports **128 file extensions** across 50+ languages — works with any file type that has comments. Inspired by [ebrevdo/ifttt-lint](https://github.com/ebrevdo/ifttt-lint).

## Install

```bash
cargo install --path .
```

## Usage

Pipe a diff from your version control system or pass a diff file directly. By default, both directive syntax checking and diff-based linting run in a single invocation.

```bash
# Pipe a diff (checks directive syntax + lints the diff)
git diff HEAD~1 | lint-ifchange

# Or pass a file
lint-ifchange changes.diff

# Check only: validate directive syntax, skip diff lint
lint-ifchange --no-lint

# Check a specific directory
lint-ifchange --no-lint -c ./src

# Lint only: skip directive syntax check
git diff HEAD~1 | lint-ifchange --no-check

# Ignore files or labels
lint-ifchange -i '*.json' -i 'config.toml#db' changes.diff
```

| Flag | Description |
|------|-------------|
| `-w, --warn` | Warn instead of failing (exit 0) |
| `-v, --verbose` | Verbose logging to stderr |
| `-p, --parallelism <N>` | Thread count (0 = auto) |
| `-i, --ignore <pattern>` | Ignore file or file#label (repeatable, globs) |
| `-c, --check <dir>` | Check directory for directive errors (default: `.`) |
| `--no-check` | Skip directive syntax check |
| `--no-lint` | Skip diff-based lint |

Exit codes: **0** ok, **1** lint errors, **2** fatal error.

## Directive Syntax

Directives live inside comments. Supported in [128 file extensions](#supported-languages) with comment styles: `//`, `/* */`, `#`, `<!-- -->`, `--`, `%`, `;`, `'`, `!`, and more.

**Case sensitivity:**
- **Directive keywords** — case-insensitive. `LINT.IfChange`, `lint.ifchange`, `Lint.Ifchange`, `LINT.THENCHANGE`, `lint.LaBeL` all work.
- **File extensions** — case-insensitive. `FOO.CSS`, `foo.css`, and `Foo.Css` are all recognized.
- **File paths and label names** — case-sensitive, matching git and Unix filesystem semantics. `ThenChange("Foo.css")` and `ThenChange("foo.css")` are different targets.

### LINT.IfChange

Marks the start of a guarded block. When lines inside this block change, all `ThenChange` targets must also be modified.

```python
# LINT.IfChange
VALUE = 42
# LINT.ThenChange(constants.py)
```

With a label (for targeted cross-references):

```python
# LINT.IfChange(feature)
FEATURE_FLAG = True
# LINT.ThenChange(config.py#feature)
```

All accepted formats:

```text
LINT.IfChange                     # bare (unlabeled)
LINT.IfChange("my-label")        # labeled, double quotes
LINT.IfChange('my-label')        # labeled, single quotes
LINT.IfChange(my-label)          # labeled, unquoted
```

### LINT.ThenChange

Closes an `IfChange` block and declares which files (and optionally labels) must also change.

**Single target:**

```text
LINT.ThenChange("other.py")        # quoted
LINT.ThenChange('other.py')        # single quotes
LINT.ThenChange(other.py)          # unquoted
LINT.ThenChange("other.py#label")  # with label reference
LINT.ThenChange("#label")          # self-reference (same file)
```

**Multiple targets — array syntax:**

```text
LINT.ThenChange(["a.ts", "b.ts"])                   # inline array
LINT.ThenChange(["a.ts", "config.py#db", "c.sql"])  # with labels
```

Multi-line array (each line in its own comment):

```js
// LINT.ThenChange([
//   "constants.ts",
//   "config.py#db",
//   "schema.sql",
// ])
```

**Multiple targets — comma-separated (no brackets):**

```text
LINT.ThenChange("a.py", "b.py")        # quoted, no brackets
LINT.ThenChange(a.py, b.py)            # unquoted, no brackets
LINT.ThenChange(/src/a.py, /src/b.py)  # absolute paths, unquoted
```

### LINT.Label / LINT.EndLabel

Defines a named region in a target file. When a `ThenChange` references `file.py#section`, only the lines between `Label("section")` and `EndLabel` must change — not the entire file.

```python
# LINT.Label("section")
value = 42
# LINT.EndLabel
```

All accepted formats:

```text
LINT.Label("name")     # double quotes
LINT.Label('name')     # single quotes
LINT.Label(name)       # unquoted
LINT.EndLabel          # closes the label region
```

Label names can contain letters, numbers, hyphens, underscores, and dots.

### Self-references

Point to a label in the same file using `#label` without a filename:

```python
# LINT.ThenChange(#other-section)
```

### Cross-references

When two files reference each other, only changes *within* an `IfChange` block trigger validation, not changes elsewhere in the file.

### Best practice

When one side is the **source of truth** (live code) and the other is derived (docs, config), place the fence only on the source-of-truth side pointing at the derived side. Use **bidirectional** fencing only when both sides are live code that must stay in sync.

## CI / Automation

Use as a pre-commit hook, CI lint step, or GitHub Actions check to enforce cross-file consistency in every pull request. Ready-to-copy templates are in [examples/](examples/README.md).

<!-- LINT.IfChange("action") -->
### GitHub Action

```yaml
- uses: slnc/lint-ifchange@v1
```

| Input | Description | Default |
|-------|-------------|---------|
| `version` | Release tag to install (e.g. `v1.0.0`). Empty means latest. | latest |
| `args` | Extra arguments passed to lint-ifchange | |
| `diff` | Path to a pre-built diff file. If empty, the action generates one. | |
| `token` | GitHub token for downloading release assets | `github.token` |
<!-- LINT.ThenChange("action.yml#inputs") -->

### Pre-commit hook

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

<!-- LINT.IfChange("supported-languages") -->
## Supported Languages

| | | | | | |
|---|---|---|---|---|---|
| `.ada` | `.cr` | `.gleam` | `.kt` | `.proto` | `.swift` |
| `.adb` | `.cs` | `.go` | `.kts` | `.ps1` | `.tex` |
| `.ads` | `.css` | `.gradle` | `.latex` | `.psd1` | `.tf` |
| `.asm` | `.cxx` | `.groovy` | `.less` | `.psm1` | `.tfvars` |
| `.bas` | `.dart` | `.h` | `.lisp` | `.py` | `.thrift` |
| `.bash` | `.el` | `.hcl` | `.lsp` | `.r` | `.toml` |
| `.bat` | `.env` | `.hh` | `.lua` | `.rb` | `.ts` |
| `.bzl` | `.erb` | `.hpp` | `.m` | `.rkt` | `.tsx` |
| `.c` | `.erl` | `.hrl` | `.md` | `.rs` | `.v` |
| `.c++` | `.ex` | `.hs` | `.mjs` | `.s` | `.vb` |
| `.cc` | `.exs` | `.htm` | `.mk` | `.sass` | `.vba` |
| `.cjs` | `.f` | `.html` | `.mm` | `.scala` | `.vhd` |
| `.cl` | `.f03` | `.hxx` | `.mojo` | `.scm` | `.vhdl` |
| `.clj` | `.f08` | `.ini` | `.mts` | `.scss` | `.vue` |
| `.cljc` | `.f90` | `.java` | `.nim` | `.sh` | `.xml` |
| `.cljs` | `.f95` | `.jl` | `.nix` | `.sql` | `.xsl` |
| `.cls` | `.for` | `.js` | `.php` | `.sty` | `.xslt` |
| `.cmake` | `.fs` | `.jsonc` | `.pl` | `.styl` | `.yaml` |
| `.cmd` | `.fsi` | `.jsp` | `.pm` | `.sv` | `.yml` |
| `.conf` | `.fsx` | `.jsx` | `.pro` | `.svelte` | `.zig` |
| `.cpp` | `.gd` | `.ksh` | `.prolog` | `.svg` | `.zsh` |

**Special files** — `Dockerfile` (including `Dockerfile.*` variants), `.gitignore`
<!-- LINT.ThenChange("src/comment/extractor.rs") -->

## Recommended AGENTS.md

Add the following to your repository's `AGENTS.md` (or equivalent AI-agent instructions file) so coding agents use `lint-ifchange` directives correctly:

```markdown
## Cross-file dependencies (lint-ifchange)

This repo uses `lint-ifchange` to enforce that related code changes together.
CI runs `git diff ... | lint-ifchange` on every PR.

### Rules for AI agents

- When you change code inside a `LINT.IfChange` / `LINT.ThenChange` block, you MUST also change every file (and label) listed in the `ThenChange`.
- When you add a new cross-file dependency, fence BOTH sides:
  - Source file: wrap the guarded lines with `LINT.IfChange` … `LINT.ThenChange(other-file.ext)`.
  - Target file: if only a specific section matters, mark it with `LINT.Label(name)` … `LINT.EndLabel` and reference it as `ThenChange(other-file.ext#name)`.
- Use the comment style native to the file's language (`//`, `#`, `<!-- -->`, `--`, etc.).
- Directive keywords are case-insensitive (`LINT.IfChange`, `lint.ifchange`, etc. all work). File paths and label names are case-sensitive.
- Prefer labeled blocks (`LINT.IfChange(feature)`) over bare ones when more than one fence exists in a file.
- For multiple targets use comma-separated or array syntax:
  `LINT.ThenChange(a.py, b.py)` or `LINT.ThenChange([a.py, b.py])`.
- Self-references use `#label` with no filename: `LINT.ThenChange(#other-section)`.
- Place fences on the source-of-truth side pointing at derived files. Use bidirectional fences only when both sides are live code.
- Run `lint-ifchange --no-lint` to validate directive syntax before committing.
```

## [Architecture](docs/ARCHITECTURE.md) · [Contributing](docs/CONTRIBUTING.md) · [License (MIT)](LICENSE)

---

[![Test](https://github.com/slnc/lint-ifchange/actions/workflows/test.yml/badge.svg)](https://github.com/slnc/lint-ifchange/actions/workflows/test.yml)
[![codecov](https://codecov.io/gh/slnc/lint-ifchange/branch/main/graph/badge.svg)](https://codecov.io/gh/slnc/lint-ifchange)
[![Sigstore](https://img.shields.io/badge/sigstore-signed-blue?logo=sigstore)](https://www.sigstore.dev/)
[![SLSA 3](https://slsa.dev/images/gh-badge-level3.svg)](https://slsa.dev)
