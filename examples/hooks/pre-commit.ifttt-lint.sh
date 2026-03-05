#!/usr/bin/env sh
set -eu

# Fail fast if the CLI is not installed.
if ! command -v lint-ifchange >/dev/null 2>&1; then
  echo "lint-ifchange not found in PATH. Install it first." >&2
  echo "Example: cargo install lint-ifchange" >&2
  exit 1
fi

# Skip when there are no staged changes.
if git diff --cached --quiet --exit-code; then
  exit 0
fi

echo "Running lint-ifchange on staged diff..."
git diff --cached --no-ext-diff --relative | lint-ifchange
