"""Entry point for lint-ifchange."""

import subprocess
import sys

from .install import download_binary


def main():
    bin_path = download_binary()
    result = subprocess.run([str(bin_path)] + sys.argv[1:])
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
