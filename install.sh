#!/bin/sh
set -eu

REPO="slnc/ifchange"
BINARY="ifchange"
DEFAULT_INSTALL_DIR="$HOME/.local/bin"

usage() {
    cat <<EOF
Usage: install.sh [OPTIONS]

Install ifchange from GitHub releases.

Options:
    --version VERSION   Install a specific version (e.g. v0.1.0) or `latest` (default)
    --prefix DIR        Install to DIR/bin (default: ~/.local)
    --help              Show this help message

Examples:
    curl -fsSL https://raw.githubusercontent.com/slnc/ifchange/main/install.sh | sh
    curl -fsSL https://raw.githubusercontent.com/slnc/ifchange/main/install.sh | sh -s -- --version v0.1.0
    curl -fsSL https://raw.githubusercontent.com/slnc/ifchange/main/install.sh | sh -s -- --prefix /usr/local
EOF
    exit 0
}

error() {
    printf "error: %s\n" "$1" >&2
    exit 1
}

info() {
    printf "  %s\n" "$1"
}

# Parse arguments
VERSION="latest"
INSTALL_DIR=""

while [ $# -gt 0 ]; do
    case "$1" in
        --version)
            [ $# -ge 2 ] || error "--version requires a value"
            VERSION="$2"
            shift 2
            ;;
        --prefix)
            [ $# -ge 2 ] || error "--prefix requires a value"
            INSTALL_DIR="$2/bin"
            shift 2
            ;;
        --help)
            usage
            ;;
        *)
            error "unknown option: $1"
            ;;
    esac
done

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        MINGW*|MSYS*|CYGWIN*) error "Windows is not supported by this installer. Download the binary manually from GitHub releases." ;;
        *) error "unsupported operating system: $(uname -s)" ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64)  echo "aarch64" ;;
        *) error "unsupported architecture: $(uname -m)" ;;
    esac
}

# Map OS to target triple component
os_target() {
    case "$1" in
        linux) echo "unknown-linux-gnu" ;;
        macos) echo "apple-darwin" ;;
    esac
}

# Check for required commands
check_command() {
    command -v "$1" >/dev/null 2>&1 || error "required command not found: $1"
}

check_command tar

OS="$(detect_os)"
ARCH="$(detect_arch)"
TARGET="${ARCH}-$(os_target "$OS")"

info "Detected platform: ${OS} ${ARCH} (${TARGET})"

# Determine download tool
if command -v curl >/dev/null 2>&1; then
    download() { curl -fsSL "$1"; }
elif command -v wget >/dev/null 2>&1; then
    download() { wget -qO- "$1"; }
else
    error "either curl or wget is required"
fi

# Resolve latest version when requested
if [ "$VERSION" = "latest" ]; then
    info "Fetching latest release..."
    VERSION=$(download "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')
    [ -n "$VERSION" ] || error "failed to determine latest version"
fi

info "Installing ${BINARY} ${VERSION}"

# Build download URLs
ARCHIVE_NAME="${BINARY}-${VERSION}-${TARGET}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE_NAME}"
CHECKSUMS_URL="https://github.com/${REPO}/releases/download/${VERSION}/SHA256SUMS"

# Create temp directory
TMPDIR_INSTALL="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_INSTALL"' EXIT

# Download archive and checksums
info "Downloading ${ARCHIVE_NAME}..."
download "$DOWNLOAD_URL" > "${TMPDIR_INSTALL}/${ARCHIVE_NAME}" || error "failed to download ${DOWNLOAD_URL}"

info "Downloading checksums..."
download "$CHECKSUMS_URL" > "${TMPDIR_INSTALL}/SHA256SUMS" || error "failed to download checksums"

# Verify checksum
info "Verifying checksum..."
EXPECTED_SUM=$(grep "${ARCHIVE_NAME}" "${TMPDIR_INSTALL}/SHA256SUMS" | awk '{print $1}')
[ -n "$EXPECTED_SUM" ] || error "checksum not found for ${ARCHIVE_NAME} in SHA256SUMS"

if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL_SUM=$(sha256sum "${TMPDIR_INSTALL}/${ARCHIVE_NAME}" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
    ACTUAL_SUM=$(shasum -a 256 "${TMPDIR_INSTALL}/${ARCHIVE_NAME}" | awk '{print $1}')
else
    error "sha256sum or shasum is required for checksum verification"
fi

if [ "$EXPECTED_SUM" != "$ACTUAL_SUM" ]; then
    error "checksum mismatch: expected ${EXPECTED_SUM}, got ${ACTUAL_SUM}"
fi
info "Checksum verified."

# Extract
info "Extracting..."
tar -xzf "${TMPDIR_INSTALL}/${ARCHIVE_NAME}" -C "${TMPDIR_INSTALL}"

# Find binary (may be at top level or in a subdirectory)
EXTRACTED_BIN=""
if [ -f "${TMPDIR_INSTALL}/${BINARY}" ]; then
    EXTRACTED_BIN="${TMPDIR_INSTALL}/${BINARY}"
elif [ -f "${TMPDIR_INSTALL}/${BINARY}-${VERSION}-${TARGET}/${BINARY}" ]; then
    EXTRACTED_BIN="${TMPDIR_INSTALL}/${BINARY}-${VERSION}-${TARGET}/${BINARY}"
else
    # Search for it
    EXTRACTED_BIN="$(find "${TMPDIR_INSTALL}" -name "${BINARY}" -type f | head -1)"
    [ -n "$EXTRACTED_BIN" ] || error "could not find ${BINARY} binary in archive"
fi

chmod +x "$EXTRACTED_BIN"

# Install
if [ -z "$INSTALL_DIR" ]; then
    INSTALL_DIR="$DEFAULT_INSTALL_DIR"
fi

if [ -w "$INSTALL_DIR" ] 2>/dev/null || mkdir -p "$INSTALL_DIR" 2>/dev/null; then
    mv "$EXTRACTED_BIN" "${INSTALL_DIR}/${BINARY}"
else
    info "Cannot write to ${INSTALL_DIR}, trying with sudo..."
    sudo mkdir -p "$INSTALL_DIR"
    sudo mv "$EXTRACTED_BIN" "${INSTALL_DIR}/${BINARY}"
fi

info "Installed ${BINARY} to ${INSTALL_DIR}/${BINARY}"

# Check if install dir is in PATH
case ":$PATH:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
        printf "\n"
        info "WARNING: ${INSTALL_DIR} is not in your PATH."
        info "Add it with: export PATH=\"${INSTALL_DIR}:\$PATH\""
        ;;
esac

info "Done. Run '${BINARY} --help' to get started."
