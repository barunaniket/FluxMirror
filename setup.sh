#!/bin/bash
set -e

echo "🚀 Starting FluxMirror Environment Setup..."

# 1. Detect OS and install system dependencies + scrcpy/adb
OS="$(uname -s)"
if [ "$OS" = "Darwin" ]; then
    echo "🍎 macOS detected. Installing dependencies via Homebrew..."
    if ! command -v brew &> /dev/null; then
        echo "Installing Homebrew..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    fi
    brew install node rust scrcpy android-platform-tools
    echo "Ensuring Xcode Command Line Tools are installed..."
    xcode-select --install || true

elif [ "$OS" = "Linux" ]; then
    echo "🐧 Linux detected. Determining distribution..."
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        case "$ID" in
            arch|manjaro|endeavouros)
                echo "Arch Linux detected. Installing via pacman..."
                sudo pacman -Syu --needed webkit2gtk-4.1 base-devel curl wget openssl appmenu-gtk-module gtk3 libappindicator-gtk3 librsvg nodejs npm scrcpy android-tools
                ;;
            fedora)
                echo "Fedora detected. Installing via dnf..."
                sudo dnf install webkit2gtk4.1-devel curl wget file openssl-devel gcc-c++ gtk3-devel libappindicator-gtk3-devel librsvg2-devel nodejs npm scrcpy android-tools
                ;;
            ubuntu|debian|pop)
                echo "Debian/Ubuntu detected. Installing via apt..."
                sudo apt-get update
                sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev nodejs npm scrcpy adb
                ;;
            *)
                echo "⚠️ Unsupported Linux distribution. Please install Tauri dependencies, scrcpy, and adb manually."
                ;;
        esac
    fi
fi

# 2. Install Rust (if not installed by package manager)
if ! command -v rustc &> /dev/null; then
    echo "🦀 Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# 3. Install Node modules and build
echo "📦 Installing Node.js dependencies..."
npm install

echo "🔨 Building FluxMirror for $OS..."
npm run tauri build

echo "✅ Setup and Build Complete! Check the 'src-tauri/target/release/bundle' folder for your executable."