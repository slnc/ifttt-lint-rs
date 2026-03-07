# Examples

Ready-to-copy setup snippets for projects that use `lint-ifchange`.

<!-- LINT.IfChange("action") -->
## GitHub Action (recommended)

The simplest way to add lint-ifchange to your CI:

```yaml
- uses: slnc/lint-ifchange@v1
```

See [`workflows/lint-ifchange.yml`](workflows/lint-ifchange.yml) for a complete workflow file.

| Input | Description | Default |
|-------|-------------|---------|
| `version` | Release tag to install (e.g. `v1.0.0`). Empty means latest. | latest |
| `args` | Extra arguments passed to lint-ifchange | |
| `diff` | Path to a pre-built diff file. If empty, the action generates one. | |
| `token` | GitHub token for downloading release assets | `github.token` |
<!-- LINT.ThenChange("../action.yml#inputs") -->

## Other options

- `hooks/pre-commit.ifttt-lint.sh`: local Git pre-commit hook that lints staged changes.
- `workflows/ifttt-lint-use-reusable.yml`: caller workflow that imports the reusable workflow (prebuilt + verified binary install).
- `workflows/ifttt-lint.yml`: standalone GitHub Actions workflow using prebuilt + verified binary install.

These files are templates. Copy them into your repository and adjust details as needed.

## What runs by default

Every invocation runs both **directive syntax checking** (validates structure across the repo) and **diff-based linting** (validates cross-file dependencies). Use `--no-scan` or `--no-lint` to skip either phase.
