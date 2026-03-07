#!/usr/bin/env sh
set -eu

# Skip ifchange without disabling other pre-commit hooks (unlike --no-verify).
# Usage: IFCHANGE_DISABLED=1 git commit -m '...'
if [ "${IFCHANGE_DISABLED:-}" = "1" ] || [ "${IFCHANGE_DISABLED:-}" = "true" ]; then
  exit 0
fi

if ! command -v ifchange >/dev/null 2>&1; then
  echo "ifchange not found in PATH. Install it first." >&2
  echo "Examples: cargo install ifchange | npm install -g @slnc/ifchange | pip install ifchange" >&2
  exit 1
fi

# Runs both directive syntax check and diff-based lint.
# Add --no-scan to skip syntax validation.
git diff --cached --no-ext-diff --relative | ifchange
