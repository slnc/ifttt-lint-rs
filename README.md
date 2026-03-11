# ifchange

[![CI](https://github.com/slnc/ifchange/actions/workflows/ci.yml/badge.svg)](https://github.com/slnc/ifchange/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/slnc/ifchange/branch/main/graph/badge.svg)](https://codecov.io/gh/slnc/ifchange)
[![Vulnerabilities](https://sonarcloud.io/api/project_badges/measure?project=slnc_ifchange&metric=vulnerabilities)](https://sonarcloud.io/summary/new_code?id=slnc_ifchange)
<br />
[![Sigstore](https://img.shields.io/badge/sigstore-signed-blue?logo=sigstore)](https://www.sigstore.dev/)
[![SLSA 3](https://slsa.dev/images/gh-badge-level3.svg)](https://slsa.dev)
[![crates.io](https://img.shields.io/crates/v/ifchange)](https://crates.io/crates/ifchange)
[![npm](https://img.shields.io/npm/v/@slnc/ifchange)](https://www.npmjs.com/package/@slnc/ifchange)
[![PyPI](https://img.shields.io/pypi/v/ifchange)](https://pypi.org/project/ifchange/)

**Lint for cross-file dependencies.** Rename an env var in your deploy config, forget the code that reads it? `ifchange` catches it in the diff. 128 file extensions, 50+ languages. Robust and fast.

How it works:
* Mark related code sections with `LINT.IfChange` / `LINT.ThenChange` comments.
* When a guarded section changes in a PR or commit, every referenced file must change too, or the build fails.

Rust implementation of Google's IfThisThenThat (IFTTT) linting pattern. TypeScript implementation: [ebrevdo/ifttt-lint](https://github.com/ebrevdo/ifttt-lint).

**[Install](#install) · [Usage](#usage) · [Directive Syntax](#directive-syntax) · [CI / Automation](#ci--automation) · [Performance](#performance) · [Supported Languages](#supported-languages)**

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/slnc/ifchange/main/install.sh | sh
cargo install ifchange        # Rust / crates.io
npm install -g @slnc/ifchange # Node.js / npm
pip install ifchange          # Python / PyPI
```

Pre-built binaries for Linux, macOS, and Windows available on [GitHub Releases](https://github.com/slnc/ifchange/releases).

Build from source:

```bash
cargo install --path .
```

## Usage

**1. Fence related sections with directives:**

```yaml
# deploy/app.yml
# LINT.IfChange
env:
  DATABASE_URL: postgres://prod:5432/myapp
  REDIS_URL: redis://prod:6379
# LINT.ThenChange(src/config.py#env)
```

```python
# src/config.py
# LINT.Label(env)
DATABASE_URL = os.environ["DATABASE_URL"]
REDIS_URL = os.environ["REDIS_URL"]
# LINT.EndLabel
```

**2. Rename an env var in the YAML, forget to update `config.py`, run ifchange:**

```bash
git diff HEAD~1 | ifchange
```

```
error: deploy/app.yml:2 -> src/config.py#env: target section has no matching changes in diff

found 1 error (1 lint)
```

You can wire this into a [pre-commit hook or CI action](#ci--automation) to run automatically.

**3. More options:**

```bash
ifchange changes.diff                              # pass a file
ifchange --no-lint                                 # scan only: validate directive syntax
ifchange --no-lint -s ./src                        # scan a specific directory
git diff HEAD~1 | ifchange --no-scan               # lint only: skip syntax scan
ifchange -i '**/*.sql' -i 'config.toml#db' f.diff  # ignore files or labeled sections
```

`--ignore` uses glob patterns (`*`, `?`, `**`) and matches both full relative paths and basenames.

| Flag | Description |
|------|-------------|
| `-w, --warn` | Warn instead of failing (exit 0) |
| `-v, --verbose` | Show processing details and validation summary |
| `-j, --jobs <N>` | Thread count (0 = auto) |
| `-i, --ignore <pattern>` | Ignore path glob or `path-glob#label` (repeatable) |
| `-s, --scan <dir>` | Scan directory for directive errors (default: `.`) |
| `--no-scan` | Skip directive syntax scan |
| `--no-lint` | Skip diff-based lint |

Exit codes: **0** ok, **1** lint errors, **2** fatal error.

## Directive Syntax

Directives go at the start of a comment line. Full syntax reference: [docs/DIRECTIVES.md](docs/DIRECTIVES.md).

### LINT.IfChange / LINT.ThenChange

`IfChange` opens a guarded section. `ThenChange` closes it and lists the files that must co-change.

Simplest case, whole-file target:

```yaml
# deploy/app.yml
# LINT.IfChange
env:
  DATABASE_URL: postgres://prod:5432/myapp
  REDIS_URL: redis://prod:6379
# LINT.ThenChange(src/config.py)
```

If `env` changes, `src/config.py` must also be modified somewhere in the diff.

With labels, narrow the requirement to a specific section:

```yaml
# deploy/app.yml                             |  # src/config.py
# LINT.IfChange("env")                       |  # LINT.Label("env")
env:                                         |  DATABASE_URL = os.environ["DATABASE_URL"]
  DATABASE_URL: postgres://prod:5432/myapp   |  REDIS_URL = os.environ["REDIS_URL"]
  REDIS_URL: redis://prod:6379               |  # LINT.EndLabel
# LINT.ThenChange(src/config.py#env)         |
```

Multiple targets:

```yaml
# deploy/app.yml
# LINT.IfChange("env")
env:
  DATABASE_URL: postgres://prod:5432/myapp
# LINT.ThenChange([
#   "src/config.py#env",
#   "docs/env-reference.md",
# ])
```

### Absolute paths (repo-root-relative)

A leading `/` resolves from the repo root, not the filesystem root. This works regardless of where you run `ifchange` within the repo. The repo root is detected by walking up from CWD looking for `.git`, `.hg`, `.jj`, `.svn`, `.pijul`, `.fslckout`, or `_FOSSIL_`:

```yaml
# deploy/app.yml (anywhere in the repo)
# LINT.IfChange
env:
  DATABASE_URL: postgres://prod:5432/myapp
# LINT.ThenChange(/src/config.py#env)
```

`/src/config.py` resolves to `<repo-root>/src/config.py`. Without the leading `/`, paths are relative to the source file's directory.

### Self-references

Point to a label in the same file with `#label` (no filename):

```yaml
# deploy/app.yml
env:
  DATABASE_URL: postgres://prod:5432/myapp
  # LINT.IfChange
  REDIS_URL: redis://prod:6379
  # LINT.ThenChange(#redis)

# ...

# LINT.Label("redis")
redis:
  host: prod
  port: 6379
# LINT.EndLabel
```

### Cross-references

When two or more files reference each other, only changes *within* an `IfChange` section trigger validation, not changes elsewhere in the file.

### Best practice

Source of truth points at derived files. Bidirectional fencing only when both sides are live code.

## CI / Automation

Run it as a pre-commit hook, or as a GitHub Action. See [examples/](examples/README.md).

<!-- LINT.IfChange("action") -->
### GitHub Action

```yaml
- uses: slnc/ifchange@v1
```

| Input | Description | Default |
|-------|-------------|---------|
| `version` | Release tag to install (e.g. `v1.0.0`). Empty means latest. | latest |
| `args` | Extra arguments passed to ifchange | |
| `diff` | Path to a pre-built diff file. If empty, the action generates one. | |
| `token` | GitHub token for downloading release assets | `github.token` |
<!-- LINT.ThenChange("action.yml#inputs") -->

### Pre-commit hook

```yaml
repos:
  - repo: https://github.com/slnc/ifchange
    rev: v0.1.0
    hooks:
      - id: ifchange        # requires ifchange in PATH
      - id: ifchange-pypi   # OR: auto-downloads binary via PyPI
```

## Performance

Wall-clock time to lint a 30k-line diff or scan all directives in a synthetic 5000-file repo (21 language types, 12-core x86_64) vs the original TypeScript implementation.

| Mode | Rust | TypeScript | Speedup |
|------|-----:|----------:|---------:|
| **Lint** | **~17 ms** | 714 ms | **~42x** |
| **Scan** | **~34 ms** | 387 ms | **~12x** |

## Versioning

- We follow [semver](https://semver.org/).
- [Stability guarantees](docs/VERSIONING.md).
- During `0.x`, minor versions may include breaking changes.

## Supported Languages

<summary>128 file extensions across 50+ languages</summary>

<!-- LINT.IfChange("supported-languages") -->
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

**Special files**: `Dockerfile{,.*}`, `.gitignore`
<!-- LINT.ThenChange("src/comment/extract.rs") -->

## Recommended AGENTS.md / CLAUDE.md

<details>
<summary>Copy this snippet into your repository's AGENTS.md so coding agents use ifchange directives correctly.</summary>

```markdown
  - When two code sections need to change in sync, use ifchange comment directives to enforce it.

  ### Example

  ```yaml
  # deploy/app.yml
  # LINT.IfChange
  env:
    DATABASE_URL: postgres://prod:5432/myapp
    REDIS_URL: redis://prod:6379
  # LINT.ThenChange(src/config.py#env)
  ```

  ```python
  # src/config.py
  # LINT.Label(env)
  DATABASE_URL = os.environ["DATABASE_URL"]
  REDIS_URL = os.environ["REDIS_URL"]
  # LINT.EndLabel
  ```

  Run `ifchange --help` for full syntax and options.
```
</details>

## [Architecture](docs/ARCHITECTURE.md) · [Contributing](docs/CONTRIBUTING.md) · [Versioning](docs/VERSIONING.md) · [License (MIT)](LICENSE)
