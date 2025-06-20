#!/bin/sh
set -e

# gdenv installer script
# Inspired by pkgx's installation approach

GDENV_VERSION="${GDENV_VERSION:-latest}"
GDENV_BASE_URL="https://github.com/bytemeadow/gdenv/releases"

# Colors for output
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    RESET='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    RESET=''
fi

info() {
    printf "${BLUE}▶${RESET} %s\n" "$1"
}

success() {
    printf "${GREEN}✅${RESET} %s\n" "$1"
}

error() {
    printf "${RED}❌${RESET} %s\n" "$1" >&2
}

warning() {
    printf "${YELLOW}⚠️${RESET}  %s\n" "$1"
}

# Detect OS and architecture
detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)

    # Normalize OS names
    case "$OS" in
        linux*) OS="linux" ;;
        darwin*) OS="macos" ;;
        msys*|mingw*|cygwin*) OS="windows" ;;
        *) error "Unsupported operating system: $OS"; exit 1 ;;
    esac

    # Normalize architecture names
    case "$ARCH" in
        x86_64|amd64) ARCH="x86_64" ;;
        arm64|aarch64) ARCH="aarch64" ;;
        *) error "Unsupported architecture: $ARCH"; exit 1 ;;
    esac

    info "Detected platform: $OS/$ARCH"
}

# Check if gdenv is already installed
check_existing() {
    if command -v gdenv >/dev/null 2>&1; then
        CURRENT_VERSION=$(gdenv --version 2>/dev/null | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+' || echo "unknown")
        warning "gdenv $CURRENT_VERSION is already installed at $(command -v gdenv)"
        printf "Do you want to reinstall? [y/N] "
        read -r response
        case "$response" in
            [yY][eE][sS]|[yY]) ;;
            *) info "Installation cancelled"; exit 0 ;;
        esac
    fi
}

# Determine installation directory
get_install_dir() {
    # Check common directories in order of preference
    if [ -w "/usr/local/bin" ] && [ -z "$CI" ]; then
        INSTALL_DIR="/usr/local/bin"
    elif [ -d "$HOME/.local/bin" ]; then
        INSTALL_DIR="$HOME/.local/bin"
    else
        INSTALL_DIR="$HOME/.local/bin"
        mkdir -p "$INSTALL_DIR"
    fi

    info "Installing to: $INSTALL_DIR"
}

# Download gdenv binary
download_gdenv() {
    if [ "$GDENV_VERSION" = "latest" ]; then
        # Get latest release URL
        DOWNLOAD_URL="$GDENV_BASE_URL/latest/download/gdenv-$OS-$ARCH"
    else
        DOWNLOAD_URL="$GDENV_BASE_URL/download/v$GDENV_VERSION/gdenv-$OS-$ARCH"
    fi

    # Add .exe extension for Windows
    if [ "$OS" = "windows" ]; then
        DOWNLOAD_URL="${DOWNLOAD_URL}.exe"
        BINARY_NAME="gdenv.exe"
    else
        BINARY_NAME="gdenv"
    fi

    info "Downloading gdenv from $DOWNLOAD_URL"

    # Create temporary directory
    TEMP_DIR=$(mktemp -d)
    trap 'rm -rf "$TEMP_DIR"' EXIT

    # Download with curl or wget
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL --progress-bar "$DOWNLOAD_URL" -o "$TEMP_DIR/$BINARY_NAME"
    elif command -v wget >/dev/null 2>&1; then
        wget -q --show-progress "$DOWNLOAD_URL" -O "$TEMP_DIR/$BINARY_NAME"
    else
        error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi

    # Make executable
    chmod +x "$TEMP_DIR/$BINARY_NAME"

    # Verify download
    if [ ! -f "$TEMP_DIR/$BINARY_NAME" ]; then
        error "Download failed"
        exit 1
    fi

    success "Downloaded successfully"
}

# Install the binary
install_binary() {
    info "Installing gdenv..."

    # Move to installation directory
    if [ -w "$INSTALL_DIR" ]; then
        mv "$TEMP_DIR/$BINARY_NAME" "$INSTALL_DIR/"
    else
        # Need sudo for system directories
        warning "Installation to $INSTALL_DIR requires sudo privileges"
        sudo mv "$TEMP_DIR/$BINARY_NAME" "$INSTALL_DIR/"
    fi

    success "gdenv installed successfully!"
}

# Setup PATH if needed
setup_path() {
    # Check if install directory is in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            info "✓ $INSTALL_DIR is already in your PATH"
            return
            ;;
    esac

    warning "$INSTALL_DIR is not in your PATH"

    # Detect shell and provide instructions
    SHELL_NAME=$(basename "$SHELL")
    case "$SHELL_NAME" in
        bash)
            PROFILE="$HOME/.bashrc"
            ;;
        zsh)
            PROFILE="$HOME/.zshrc"
            ;;
        fish)
            info "For fish shell, run:"
            printf "  ${GREEN}fish_add_path %s${RESET}\n" "$INSTALL_DIR"
            return
            ;;
        *)
            PROFILE="$HOME/.profile"
            ;;
    esac

    info "Add this line to your $PROFILE:"
    printf "  ${GREEN}export PATH=\"%s:\$PATH\"${RESET}\n" "$INSTALL_DIR"
    info "Then restart your shell or run:"
    printf "  ${GREEN}source %s${RESET}\n" "$PROFILE"
}

# Main installation flow
main() {
    printf "${BLUE}%s${RESET}\n" "
    ┌──────────────────────────────┐
    │  gdenv - Godot Environment   │
    │        Manager               │
    │  https://gdenv.bytemeadow.com │
    └──────────────────────────────┘
    "

    detect_platform
    check_existing
    get_install_dir
    download_gdenv
    install_binary
    setup_path

    echo
    success "Installation complete! 🎉"
    info "Run 'gdenv --help' to get started"
    info "Install a Godot version with: gdenv install 4.2.1"
}

# Run main function
main "$@"
