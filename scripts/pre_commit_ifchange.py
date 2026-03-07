#!/usr/bin/env python3
"""Run ifchange against the staged git diff for pre-commit."""

from __future__ import annotations

import subprocess
import sys


def main() -> int:
    diff_proc = subprocess.run(
        ["git", "diff", "--cached", "--no-ext-diff", "--relative"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if diff_proc.returncode != 0:
        if diff_proc.stderr:
            sys.stderr.buffer.write(diff_proc.stderr)
        return diff_proc.returncode

    lint_proc = subprocess.run(
        ["ifchange"],
        input=diff_proc.stdout,
        stderr=sys.stderr,
        check=False,
    )
    return lint_proc.returncode


if __name__ == "__main__":
    raise SystemExit(main())
