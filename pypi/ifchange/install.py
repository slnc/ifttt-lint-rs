"""Download and cache the ifchange binary from GitHub releases."""

import hashlib
import io
import os
import platform
import stat
import sys
import tarfile
import tempfile
import urllib.request
import zipfile
from pathlib import Path

from . import __version__

REPO = "slnc/ifchange"
BINARY = "ifchange"

PLATFORM_MAP = {
    ("Linux", "x86_64"): "x86_64-unknown-linux-gnu",
    ("Linux", "aarch64"): "aarch64-unknown-linux-gnu",
    ("Darwin", "x86_64"): "x86_64-apple-darwin",
    ("Darwin", "arm64"): "aarch64-apple-darwin",
    ("Windows", "AMD64"): "x86_64-pc-windows-msvc",
}


def get_target():
    system = platform.system()
    machine = platform.machine()
    target = PLATFORM_MAP.get((system, machine))
    if not target:
        raise RuntimeError(f"Unsupported platform: {system} {machine}")
    return target


def get_bin_dir():
    return Path(__file__).parent / "bin"


def get_bin_path():
    name = f"{BINARY}.exe" if platform.system() == "Windows" else BINARY
    return get_bin_dir() / name


def _fetch(url):
    req = urllib.request.Request(url, headers={"User-Agent": "ifchange-pypi"})
    with urllib.request.urlopen(req) as resp:
        return resp.read()


def _verify_checksum(data, archive_name, checksums_data):
    for line in checksums_data.decode().splitlines():
        if archive_name in line:
            expected = line.strip().split()[0]
            actual = hashlib.sha256(data).hexdigest()
            if expected != actual:
                raise RuntimeError(
                    f"Checksum mismatch: expected {expected}, got {actual}"
                )
            return
    raise RuntimeError(f"Checksum not found for {archive_name}")


def download_binary():
    bin_path = get_bin_path()
    if bin_path.exists():
        return bin_path

    target = get_target()
    version = f"v{__version__}"
    is_windows = platform.system() == "Windows"
    ext = "zip" if is_windows else "tar.gz"
    archive_name = f"{BINARY}-{version}-{target}.{ext}"

    base_url = f"https://github.com/{REPO}/releases/download/{version}"
    archive_url = f"{base_url}/{archive_name}"
    checksums_url = f"{base_url}/SHA256SUMS"

    print(f"Downloading {archive_name}...", file=sys.stderr)
    archive_data = _fetch(archive_url)
    checksums_data = _fetch(checksums_url)

    _verify_checksum(archive_data, archive_name, checksums_data)
    print("Checksum verified.", file=sys.stderr)

    # Extract binary
    bin_name = f"{BINARY}.exe" if is_windows else BINARY
    extracted = None

    if ext == "zip":
        with zipfile.ZipFile(io.BytesIO(archive_data)) as zf:
            for name in zf.namelist():
                if name.endswith(bin_name):
                    extracted = zf.read(name)
                    break
    else:
        with tarfile.open(fileobj=io.BytesIO(archive_data), mode="r:gz") as tf:
            for member in tf.getmembers():
                # Guard against path traversal (e.g. ../../etc/passwd)
                if member.name.startswith("/") or ".." in member.name.split("/"):
                    raise RuntimeError(f"Unsafe path in archive: {member.name}")
                if member.name.endswith(bin_name) and member.isfile():
                    f = tf.extractfile(member)
                    if f:
                        extracted = f.read()
                    break

    if not extracted:
        raise RuntimeError(f"Could not find {bin_name} in archive")

    bin_dir = get_bin_dir()
    bin_dir.mkdir(parents=True, exist_ok=True)
    bin_path.write_bytes(extracted)
    bin_path.chmod(bin_path.stat().st_mode | stat.S_IEXEC | stat.S_IXGRP | stat.S_IXOTH)

    print(f"Installed {BINARY} to {bin_path}", file=sys.stderr)
    return bin_path
