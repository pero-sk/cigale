#!/bin/bash

REPO_URL="https://github.com/pero-sk/cigale"
INSTALL_DIR="$HOME/.cigale"
BIN_DIR="$INSTALL_DIR/bin"
SRC_DIR="$INSTALL_DIR/src"

command_exists() {
    command -v "$1" &> /dev/null
}

check_deps() {
    echo "Checking dependencies..."
    if ! command_exists git; then
        echo "Error: git is required but not installed"
        echo "Install from https://git-scm.com"
        exit 1
    fi
    if ! command_exists cargo; then
        echo "Error: cargo is required but not installed"
        echo "Install Rust from https://rustup.rs"
        exit 1
    fi
    echo "✓ git found"
    echo "✓ cargo found"
}

add_to_path() {
    SHELL_RC=""
    if [[ "$SHELL" == *"zsh"* ]]; then
        SHELL_RC="$HOME/.zshrc"
    elif [[ "$SHELL" == *"fish"* ]]; then
        SHELL_RC="$HOME/.config/fish/config.fish"
    else
        SHELL_RC="$HOME/.bashrc"
    fi

    EXPORT_LINE="export PATH=\"\$PATH:$BIN_DIR\""

    if grep -q "$BIN_DIR" "$SHELL_RC" 2>/dev/null; then
        echo "PATH already contains $BIN_DIR"
    else
        echo "" >> "$SHELL_RC"
        echo "# Cigale" >> "$SHELL_RC"
        echo "$EXPORT_LINE" >> "$SHELL_RC"
        echo "✓ Added $BIN_DIR to PATH in $SHELL_RC"
    fi
}

install() {
    local VERSION="$1"

    echo "Installing Cigale..."
    check_deps

    mkdir -p "$BIN_DIR"
    mkdir -p "$SRC_DIR"

    if [ -d "$SRC_DIR/.git" ]; then
        echo "Source already exists, pulling latest..."
        cd "$SRC_DIR" && git pull
    else
        echo "Cloning repository..."
        git clone "$REPO_URL" "$SRC_DIR"
    fi

    # checkout specific version if provided
    if [ -n "$VERSION" ]; then
        echo "Checking out version $VERSION..."
        cd "$SRC_DIR" && git checkout "$VERSION"
    fi

    echo "Building Cigale..."
    cd "$SRC_DIR" && cargo build --release \
        --features="stdl" \
        --bin cigale_stdl
    
    cargo build --release \
        --bin cigale_nostdl \
        --bin cigale_cli

    if [ $? -ne 0 ]; then
        echo "Build failed!"
        exit 1
    fi

    echo "Installing binaries..."
    cp "$SRC_DIR/target/release/cigale_stdl"   "$BIN_DIR/cigale_stdl"
    cp "$SRC_DIR/target/release/cigale_nostdl" "$BIN_DIR/cigale_nostdl"
    cp "$SRC_DIR/cigale.sh"                    "$BIN_DIR/cigale.sh"
    cp "$SRC_DIR/cigale.bat"                   "$BIN_DIR/cigale.bat"
    chmod +x "$BIN_DIR/cigale_stdl"
    chmod +x "$BIN_DIR/cigale_nostdl"
    chmod +x "$BIN_DIR/cigale.sh"

    # cigale itself -- copy to temp then rename (avoids file-in-use issues)
    cp "$SRC_DIR/target/release/cigale_cli" "$BIN_DIR/cigale_new"
    chmod +x "$BIN_DIR/cigale_new"
    mv "$BIN_DIR/cigale_new" "$BIN_DIR/cigale"
    echo "✓ Binaries installed"

    add_to_path

    echo ""
    echo "✓ Cigale installed to $BIN_DIR"
    echo "  Restart your terminal or run: source $SHELL_RC"
    echo "  Then use: cigale run <file.cig>"
    exec $SHELL
}

update() {
    echo "Updating Cigale..."
    check_deps

    if [ ! -d "$SRC_DIR/.git" ]; then
        echo "Cigale is not installed. Run: ./cigale.sh install"
        exit 1
    fi

    echo "Pulling latest changes..."
    cd "$SRC_DIR" && git pull

    if [ $? -ne 0 ]; then
        echo "Failed to pull latest changes!"
        exit 1
    fi

    echo "Rebuilding..."
    cd "$SRC_DIR" && cargo build --release \
        --bin cigale_stdl \
        --bin cigale_nostdl \
        --bin cigale_cli

    if [ $? -ne 0 ]; then
        echo "Build failed!"
        exit 1
    fi

    echo "Installing binaries..."
    cp "$SRC_DIR/target/release/cigale_cli"    "$BIN_DIR/cigale"
    cp "$SRC_DIR/target/release/cigale_stdl"   "$BIN_DIR/cigale_stdl"
    cp "$SRC_DIR/target/release/cigale_nostdl" "$BIN_DIR/cigale_nostdl"
    cp "$SRC_DIR/cigale.sh"                    "$BIN_DIR/cigale.sh"
    cp "$SRC_DIR/cigale.bat"                   "$BIN_DIR/cigale.bat"
    chmod +x "$BIN_DIR/cigale"
    chmod +x "$BIN_DIR/cigale_stdl"
    chmod +x "$BIN_DIR/cigale_nostdl"
    chmod +x "$BIN_DIR/cigale.sh"

    echo ""
    echo "✓ Cigale updated successfully"
    echo "  Version: $(cd $SRC_DIR && git describe --tags 2>/dev/null || git rev-parse --short HEAD)"
}

uninstall() {
    echo "Uninstalling Cigale..."
    if [ -d "$INSTALL_DIR" ]; then
        rm -rf "$INSTALL_DIR"
        echo "✓ Removed $INSTALL_DIR"
    fi

    # remove from PATH
    SHELL_RC=""
    if [[ "$SHELL" == *"zsh"* ]]; then
        SHELL_RC="$HOME/.zshrc"
    else
        SHELL_RC="$HOME/.bashrc"
    fi

    if [ -f "$SHELL_RC" ]; then
        sed -i '/# Cigale/d' "$SHELL_RC"
        sed -i "/export PATH.*$BIN_DIR/d" "$SHELL_RC"
        echo "✓ Removed from PATH in $SHELL_RC"
    fi

    echo "✓ Cigale uninstalled"
}

case "$1" in
    install)   install "$2" ;;
    update)    update ;;
    uninstall) uninstall ;;
    *)
        echo "Cigale Bootstrap Script"
        echo "usage: ./cigale.sh <install|update|uninstall> [version]"
        echo ""
        echo "  install [version]  -- install cigale (optionally at a specific version)"
        echo "  update             -- update cigale to latest"
        echo "  uninstall          -- remove cigale"
        exit 1
        ;;
esac