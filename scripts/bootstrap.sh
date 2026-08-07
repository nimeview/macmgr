#!/usr/bin/env bash
# Bootstrap macmgr on a fresh Mac, then install the rice.
set -euo pipefail

REPO_URL="${MACMGR_REPO:-https://github.com/YOUR_USER/MacOS_Manager.git}"
DIR="${MACMGR_DIR:-$HOME/src/MacOS_Manager}"

if ! command -v brew >/dev/null 2>&1; then
  echo "Installing Homebrew..."
  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "Installing Rust..."
  brew install rust
fi

if [[ ! -d "$DIR/.git" ]]; then
  mkdir -p "$(dirname "$DIR")"
  git clone "$REPO_URL" "$DIR"
fi

cd "$DIR"
cargo install --path . --force
mkdir -p "$HOME/.config/macmgr"
cp -f "$DIR/manifest.toml" "$HOME/.config/macmgr/manifest.toml"

echo "macmgr installed. Next:"
echo "  macmgr doctor"
echo "  macmgr install --scope core"
echo "  macmgr          # TUI"
