#!/bin/sh
# Install script for meshc - the Mesh compiler
# Usage: curl -sSf https://meshlang.dev/install.sh | sh
# Or: sh install.sh [--version VERSION] [--uninstall] [--yes] [--help]
set -eu

REPO="snowdamiz/mesh-lang"
INSTALL_DIR="$HOME/.mesh/bin"
ENV_FILE="$HOME/.mesh/env"
VERSION_FILE="$HOME/.mesh/version"
MARKER="# Mesh compiler"

# --- Color output ---

_use_color() {
    if [ -n "${NO_COLOR:-}" ]; then
        return 1
    fi
    if [ -t 1 ]; then
        return 0
    fi
    return 1
}

say() {
    printf '%s\n' "$1"
}

say_green() {
    if _use_color; then
        printf '\033[32m%s\033[0m\n' "$1"
    else
        printf '%s\n' "$1"
    fi
}

say_red() {
    if _use_color; then
        printf '\033[31m%s\033[0m\n' "$1" >&2
    else
        printf '%s\n' "$1" >&2
    fi
}

say_bold() {
    if _use_color; then
        printf '\033[1m%s\033[0m\n' "$1"
    else
        printf '%s\n' "$1"
    fi
}

# --- Download helper ---

_downloader=""

detect_downloader() {
    if command -v curl > /dev/null 2>&1; then
        _downloader="curl"
    elif command -v wget > /dev/null 2>&1; then
        _downloader="wget"
    else
        say_red "error: Need curl or wget to download meshc."
        return 1
    fi
}

download() {
    local _url="$1"
    local _output="$2"

    if [ "$_downloader" = "curl" ]; then
        curl -sSfL "$_url" -o "$_output"
    elif [ "$_downloader" = "wget" ]; then
        wget -qO "$_output" "$_url"
    fi
}

download_to_stdout() {
    local _url="$1"

    if [ "$_downloader" = "curl" ]; then
        curl -sSfL "$_url"
    elif [ "$_downloader" = "wget" ]; then
        wget -qO- "$_url"
    fi
}

# --- Platform detection ---

detect_platform() {
    local _ostype _cputype

    _ostype="$(uname -s)"
    _cputype="$(uname -m)"

    case "$_ostype" in
        Linux)
            _ostype="unknown-linux-gnu"
            ;;
        Darwin)
            _ostype="apple-darwin"
            # Detect true architecture (handles Rosetta)
            if [ "$_cputype" = "x86_64" ]; then
                if sysctl -n hw.optional.arm64 2>/dev/null | grep -q "1"; then
                    _cputype="aarch64"
                fi
            fi
            ;;
        MINGW* | MSYS* | CYGWIN*)
            say_red "error: Windows detected. Please use install.ps1 instead:"
            say_red "  powershell -ExecutionPolicy ByPass -c \"irm https://meshlang.dev/install.ps1 | iex\""
            return 1
            ;;
        *)
            say_red "error: Unsupported OS: $_ostype"
            return 1
            ;;
    esac

    case "$_cputype" in
        x86_64 | amd64)
            _cputype="x86_64"
            ;;
        aarch64 | arm64)
            _cputype="aarch64"
            ;;
        *)
            say_red "error: Unsupported architecture: $_cputype"
            return 1
            ;;
    esac

    echo "${_cputype}-${_ostype}"
}

# --- Version management ---

get_latest_version() {
    local _response _version

    _response="$(download_to_stdout "https://api.github.com/repos/${REPO}/releases/latest")"

    if command -v jq > /dev/null 2>&1; then
        _version="$(echo "$_response" | jq -r '.tag_name' | sed 's/^v//')"
    else
        _version="$(echo "$_response" | grep '"tag_name"' | sed 's/.*"v\([^"]*\)".*/\1/')"
    fi

    if [ -z "$_version" ]; then
        say_red "error: Failed to determine latest version."
        say_red "  Check https://github.com/${REPO}/releases for available versions."
        return 1
    fi

    echo "$_version"
}

check_update_needed() {
    local _target_version="$1"

    if [ -f "$VERSION_FILE" ]; then
        local _current
        _current="$(cat "$VERSION_FILE")"
        if [ "$_current" = "$_target_version" ]; then
            say_green "meshc v${_target_version} is already installed and up-to-date."
            return 1
        fi
        say "Updating meshc from v${_current} to v${_target_version}..."
    fi
    return 0
}

# --- Checksum verification ---

verify_checksum() {
    local _file="$1"
    local _expected="$2"
    local _actual

    if command -v sha256sum > /dev/null 2>&1; then
        _actual="$(sha256sum "$_file" | cut -d' ' -f1)"
    elif command -v shasum > /dev/null 2>&1; then
        _actual="$(shasum -a 256 "$_file" | cut -d' ' -f1)"
    else
        say "warning: No SHA-256 tool found, skipping checksum verification."
        return 0
    fi

    if [ "$_actual" != "$_expected" ]; then
        say_red "error: Checksum verification failed."
        say_red "  expected: $_expected"
        say_red "  actual:   $_actual"
        return 1
    fi
}

# --- PATH configuration ---

configure_path() {
    local _mesh_env="$HOME/.mesh/env"

    # Create env file
    cat > "$_mesh_env" << 'ENVEOF'
# Mesh compiler
# This file is sourced by shell profiles to add meshc to PATH
export PATH="$HOME/.mesh/bin:$PATH"
ENVEOF

    # Bash
    local _bash_done=0
    for _profile in "$HOME/.bashrc" "$HOME/.bash_profile"; do
        if [ -f "$_profile" ]; then
            if ! grep -q "$MARKER" "$_profile" 2>/dev/null; then
                printf '\n%s\n. "%s"\n' "$MARKER" "$_mesh_env" >> "$_profile"
            fi
            _bash_done=1
            break
        fi
    done

    # Zsh
    local _zshrc="$HOME/.zshrc"
    if [ -f "$_zshrc" ] || [ "$(basename "${SHELL:-}")" = "zsh" ]; then
        if ! grep -q "$MARKER" "$_zshrc" 2>/dev/null; then
            printf '\n%s\n. "%s"\n' "$MARKER" "$_mesh_env" >> "$_zshrc"
        fi
    fi

    # Fish
    local _fish_config="$HOME/.config/fish/config.fish"
    if [ -f "$_fish_config" ] || command -v fish > /dev/null 2>&1; then
        mkdir -p "$(dirname "$_fish_config")"
        if ! grep -q "$MARKER" "$_fish_config" 2>/dev/null; then
            printf '\n%s\nfish_add_path %s/.mesh/bin\n' "$MARKER" "$HOME" >> "$_fish_config"
        fi
    fi
}

# --- Uninstall ---

uninstall() {
    say "Uninstalling meshc..."

    # Remove installation directory
    if [ -d "$HOME/.mesh" ]; then
        rm -rf "$HOME/.mesh"
    fi

    # Remove from bash profiles
    for _profile in "$HOME/.bashrc" "$HOME/.bash_profile"; do
        if [ -f "$_profile" ]; then
            # Remove marker line and the source line after it
            local _tmp
            _tmp="$(mktemp)"
            sed "/$MARKER/,+1d" "$_profile" > "$_tmp"
            mv "$_tmp" "$_profile"
        fi
    done

    # Remove from zsh profile
    if [ -f "$HOME/.zshrc" ]; then
        local _tmp
        _tmp="$(mktemp)"
        sed "/$MARKER/,+1d" "$HOME/.zshrc" > "$_tmp"
        mv "$_tmp" "$HOME/.zshrc"
    fi

    # Remove from fish config
    local _fish_config="$HOME/.config/fish/config.fish"
    if [ -f "$_fish_config" ]; then
        local _tmp
        _tmp="$(mktemp)"
        sed "/$MARKER/,+1d" "$_fish_config" > "$_tmp"
        mv "$_tmp" "$_fish_config"
    fi

    say_green "meshc has been uninstalled."
}

# --- Install binary helper ---

install_binary() {
    local _bin_name="$1"
    local _bin_version="$2"
    local _platform _archive _url _tmpdir _checksum_file _expected_hash

    # Detect platform
    _platform="$(detect_platform)"

    say "Installing ${_bin_name} v${_bin_version} (${_platform})..."

    # Construct download URL
    _archive="${_bin_name}-v${_bin_version}-${_platform}.tar.gz"
    _url="https://github.com/${REPO}/releases/download/v${_bin_version}/${_archive}"

    # Create temp directory
    _tmpdir="$(mktemp -d)"

    # Download tarball
    if ! download "$_url" "$_tmpdir/$_archive"; then
        say_red "error: Failed to download ${_bin_name} v${_bin_version} (HTTP error)."
        say_red "  URL: $_url"
        say_red "  Check https://github.com/${REPO}/releases for available versions."
        rm -rf "$_tmpdir"
        return 1
    fi

    # Download and verify checksum
    _checksum_file="$_tmpdir/SHA256SUMS"
    if download "https://github.com/${REPO}/releases/download/v${_bin_version}/SHA256SUMS" "$_checksum_file" 2>/dev/null; then
        _expected_hash="$(grep "$_archive" "$_checksum_file" | cut -d' ' -f1)"
        if [ -n "$_expected_hash" ]; then
            verify_checksum "$_tmpdir/$_archive" "$_expected_hash"
        else
            say "warning: Archive not found in SHA256SUMS, skipping verification."
        fi
    else
        say "warning: Could not download SHA256SUMS, skipping checksum verification."
    fi

    # Extract
    tar xzf "$_tmpdir/$_archive" -C "$_tmpdir"

    # Install binary
    mkdir -p "$INSTALL_DIR"
    mv "$_tmpdir/${_bin_name}" "$INSTALL_DIR/${_bin_name}"
    chmod +x "$INSTALL_DIR/${_bin_name}"

    # Remove macOS quarantine attribute
    case "$(uname -s)" in
        Darwin)
            xattr -d com.apple.quarantine "$INSTALL_DIR/${_bin_name}" 2>/dev/null || true
            ;;
    esac

    rm -rf "$_tmpdir"
}

# --- Install ---

install() {
    local _version="$1"

    detect_downloader

    # Determine version
    if [ -z "$_version" ]; then
        say "Fetching latest version..."
        _version="$(get_latest_version)"
    fi

    # Check if update needed
    if ! check_update_needed "$_version"; then
        return 0
    fi

    # Install meshc
    install_binary "meshc" "$_version"

    # Install meshpkg
    install_binary "meshpkg" "$_version"

    # Write version file
    echo "$_version" > "$VERSION_FILE"

    # Configure PATH
    configure_path

    say_green "Installed meshc and meshpkg v${_version} to ~/.mesh/bin/"
    say "Run 'meshc --version' and 'meshpkg --version' to verify, or restart your shell."
}

# --- Usage ---

usage() {
    say "meshc installer"
    say ""
    say "Usage: install.sh [OPTIONS]"
    say ""
    say "Options:"
    say "  --version VERSION  Install a specific version (default: latest)"
    say "  --uninstall        Remove meshc and clean up PATH changes"
    say "  --yes              Accept defaults (for CI, already non-interactive)"
    say "  --help             Show this help message"
}

# --- Main ---

main() {
    local _version=""
    local _do_uninstall=0

    while [ $# -gt 0 ]; do
        case "$1" in
            --version)
                if [ $# -lt 2 ]; then
                    say_red "error: --version requires a value"
                    return 1
                fi
                _version="$2"
                shift 2
                ;;
            --uninstall)
                _do_uninstall=1
                shift
                ;;
            --yes | -y)
                # Accept and ignore -- script is already non-interactive
                shift
                ;;
            --help | -h)
                usage
                return 0
                ;;
            *)
                say_red "error: Unknown option: $1"
                usage
                return 1
                ;;
        esac
    done

    if [ "$_do_uninstall" = "1" ]; then
        uninstall
    else
        install "$_version"
    fi
}

main "$@"
