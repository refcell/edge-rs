#!/usr/bin/env bash
set -euo pipefail

EDGEUP_VERSION="${EDGEUP_VERSION:-latest}"
INSTALL_DIR="${EDGEUP_INSTALL_DIR:-$HOME/.edgeup}"
BIN_DIR="$INSTALL_DIR/bin"
REPO="refcell/edge-rs"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Detect platform
detect_platform() {
  local OS ARCH
  OS=$(uname -s)
  ARCH=$(uname -m)

  case "$OS" in
    Darwin)
      case "$ARCH" in
        x86_64)
          echo "x86_64-apple-darwin"
          ;;
        arm64)
          echo "aarch64-apple-darwin"
          ;;
        *)
          echo "Unsupported architecture: $ARCH" >&2
          exit 1
          ;;
      esac
      ;;
    Linux)
      case "$ARCH" in
        x86_64)
          echo "x86_64-unknown-linux-gnu"
          ;;
        *)
          echo "Unsupported architecture: $ARCH" >&2
          exit 1
          ;;
      esac
      ;;
    *)
      echo "Unsupported OS: $OS" >&2
      exit 1
      ;;
  esac
}

# Get the latest release version
get_latest_version() {
  local version
  version=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
  if [ -z "$version" ]; then
    echo "Failed to determine latest version" >&2
    exit 1
  fi
  echo "$version"
}

# Download and install
install_edgeup() {
  local platform version url binary_dir

  platform=$(detect_platform)
  
  if [ "$EDGEUP_VERSION" = "latest" ]; then
    version=$(get_latest_version)
  else
    version="$EDGEUP_VERSION"
  fi

  url="https://github.com/$REPO/releases/download/$version/edge-rs-$platform"

  echo -e "${YELLOW}Installing edgeup $version for $platform...${NC}"

  # Create directory
  mkdir -p "$BIN_DIR"

  # Download binary
  if ! curl -L -f --progress-bar "$url" -o "$BIN_DIR/edge-rs"; then
    echo -e "${RED}Failed to download binary from $url${NC}" >&2
    exit 1
  fi

  chmod +x "$BIN_DIR/edge-rs"

  # Update shell profiles
  update_shell_profiles "$INSTALL_DIR"

  echo -e "${GREEN}Successfully installed edgeup!${NC}"
  echo ""
  echo "To get started, run:"
  echo "  edge-rs --help"
  echo ""
  echo "Or source your shell profile to update your PATH:"
  echo "  source ~/.bashrc   # for bash"
  echo "  source ~/.zshrc    # for zsh"
}

# Update shell profiles to include ~/.edgeup/bin in PATH
update_shell_profiles() {
  local install_dir="$1"
  local bin_dir="$install_dir/bin"
  local path_export="export PATH=\"$bin_dir:\$PATH\""

  # List of shell rc files to update
  local rc_files=("$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.zshrc" "$HOME/.profile")

  for rc_file in "${rc_files[@]}"; do
    if [ -f "$rc_file" ]; then
      # Check if PATH entry already exists
      if ! grep -q "$bin_dir" "$rc_file" 2>/dev/null; then
        echo "" >> "$rc_file"
        echo "# edgeup" >> "$rc_file"
        echo "$path_export" >> "$rc_file"
      fi
    fi
  done
}

# Main
install_edgeup
