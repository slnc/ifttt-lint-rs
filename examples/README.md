# Examples

Ready-to-copy setup snippets for projects that use `lint-ifchange`.

- `hooks/pre-commit.ifttt-lint.sh`: local Git pre-commit hook that lints staged changes.
- `workflows/ifttt-lint-use-reusable.yml`: caller workflow that imports the reusable workflow from this repo (prebuilt + verified binary install).
- `workflows/ifttt-lint.yml`: standalone GitHub Actions workflow using prebuilt + verified binary install.

These files are templates. Copy them into your repository and adjust install/source details as needed.
