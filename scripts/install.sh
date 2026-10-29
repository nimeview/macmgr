#!/usr/bin/env bash
# Thin install helper: clone → cargo install → setup.
# Prefer GitHub Release binaries once published (see Formula/macmgr.rb).
set -euo pipefail

REPO="${MACMGR_REPO:-https://github.com/arturpanan/MacOS_Manager.git}"
DIR="${MACMGR_DIR:-$HOME/src/MacOS_Manager}"

echo "macmgr install script"
echo "  repo: $REPO"
echo "  dir:  $DIR"

if ! command -v brew >/dev/null 2>&1; then
  echo "Homebrew required: https://brew.sh"
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  brew install rust
fi

if [[ ! -d "$DIR/.git" ]]; then
  git clone "$REPO" "$DIR"
fi

cd "$DIR"
cargo install --path . --force
mkdir -p "$HOME/.config/macmgr"
cp -f "$DIR/manifest.toml" "$HOME/.config/macmgr/manifest.toml"

echo "Run: macmgr setup --yes"
