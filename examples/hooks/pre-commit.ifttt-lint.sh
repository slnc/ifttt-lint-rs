#!/usr/bin/env sh
set -eu

if ! command -v lint-ifchange >/dev/null 2>&1; then
  echo "lint-ifchange not found in PATH. Install it first." >&2
  echo "Example: cargo install lint-ifchange" >&2
  exit 1
fi

# Runs both directive syntax check and diff-based lint.
# Add --no-check to skip syntax validation.
git diff --cached --no-ext-diff --relative | lint-ifchange
