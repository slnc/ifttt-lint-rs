"""Entry point for ifchange."""

import os
import sys

from .install import download_binary


def main():
    bin_path = str(download_binary())
    if sys.platform == "win32":
        import subprocess
        result = subprocess.run([bin_path] + sys.argv[1:])
        sys.exit(result.returncode)
    else:
        os.execvp(bin_path, [bin_path] + sys.argv[1:])


if __name__ == "__main__":
    main()
