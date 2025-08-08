#!/bin/bash

# next-dev-utils installer script
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${BOLD}${BLUE}$1${NC}"
}

print_info() {
    echo -e "${BLUE}→${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}!${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Header
print_header "vercel-scripts installer"
echo "================================"
echo

# Change to the home directory
cd $HOME

################################################################################
# Install all the prerequisites.
################################################################################

print_info "Installing prerequisites..."
echo

# Check if `brew` is installed.
if ! command -v brew &> /dev/null; then
    print_error "Homebrew is not installed or not in PATH"
    print_info "Please install Homebrew first: https://brew.sh"
    exit 1
fi

# Check if Deno is installed
if ! command -v deno &> /dev/null; then
    print_info "Installing Deno..."
    brew install deno --quiet
fi

print_success "Found Deno ${BLUE}$(deno --version | head -n1 | awk '{print $2}')${NC}"

# Check if `nu` is installed.
if ! command -v nu &> /dev/null; then
    print_info "Installing nu..."
    curl -fsSL https://raw.githubusercontent.com/wyattjoh/next-dev-utils/refs/heads/main/scripts/install.sh | bash
fi

# Verify that `nu` is installed.
if ! command -v nu &> /dev/null; then
    print_error "nu is not installed or not in PATH"
    print_info "Please install nu first: https://github.com/wyattjoh/next-dev-utils"
    exit 1
fi

print_success "Found nu ${BLUE}$(nu --version | head -n1 | awk '{print $2}')${NC}"

# Check if `jq` is installed.
if ! command -v jq &> /dev/null; then
    print_info "Installing jq..."
    brew install jq --quiet
fi

print_success "Found jq ${BLUE}$(jq --version | awk -F'-' '{print $2}')${NC}"

echo

################################################################################
# Install the package.
################################################################################

# Clear cache to ensure we get the latest version
print_info "🧹 Clearing package cache to ensure latest version..."
deno cache --reload jsr:@wyattjoh/vercel-scripts

# Install with --force to overwrite existing installations
print_info "📦 Installing @wyattjoh/vercel-scripts as 'vss'..."
deno install --quiet --no-config --global --force --reload --allow-read --allow-write --allow-net --allow-run --allow-env --allow-sys -n vss jsr:@wyattjoh/vercel-scripts

################################################################################
# Verify the installation.
################################################################################

# Check if Deno bin is in PATH
DENO_INSTALL_ROOT="${DENO_INSTALL_ROOT:-$HOME/.deno}"
if [[ ":$PATH:" != *":$DENO_INSTALL_ROOT/bin:"* ]]; then
    print_warning "Deno bin directory is not in your PATH"
    print_info "Add this to your shell profile (.bashrc, .zshrc, etc.):"
    echo "  export PATH=\"\$PATH:$DENO_INSTALL_ROOT/bin\""
    echo

    VSS_VERSION="unknown"
else
    # Test installations
    print_info "🔍 Verifying installations..."
    if command -v vss &> /dev/null; then
        VSS_VERSION=$(vss --version 2>/dev/null || echo "unknown")
    else
        print_warning "vss not found in PATH"
        VSS_VERSION="unknown"
    fi
fi

echo
print_success "Installation complete!"
echo
print_info "Available commands:"
echo "  vss (version: $VSS_VERSION)"
echo