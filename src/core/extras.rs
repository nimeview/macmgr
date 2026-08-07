use anyhow::{anyhow, Result};

use crate::core::manifest::Manifest;
use crate::core::paths::expand_tilde;
use crate::util::shell::run_status;

/// Ensure yabai spaces helper runs if present.
pub fn ensure_spaces(manifest: &Manifest) -> Result<()> {
    if manifest.paths.ensure_spaces.is_empty() {
        return Err(anyhow!("ensure_spaces path not set in manifest"));
    }
    let script = expand_tilde(&manifest.paths.ensure_spaces);
    if !script.exists() {
        return Err(anyhow!("missing {}", script.display()));
    }
    run_status(script.to_str().unwrap_or(""), &[])
}

/// Quick SIP / accessibility reminder + core reload tip.
pub fn tips() -> String {
    r#"Useful next steps
──────────────────
MENTAL MODEL
  LIVE (~/.config) = то, что крутится сейчас (правишь каждый день)
  My rice          = закреплённая копия (macmgr rice push)
  DIFF             = LIVE ≠ rice (незакреплённые правки — это нормально)

EDIT TODAY
  macmgr                  → Configs/Plugins/Hotkeys → e / a
  macmgr config edit sketchybar

PIN / «закрепить в сборку»
  macmgr rice status
  macmgr rice push sketchybar     # или: rice push (всё)
  macmgr rice commit -m '…'       # git commit в My rice
  macmgr rice git-push            # если remote уже есть

TUI SHORTCUTS
  :  command palette
  s  (Pkgs) brew search · a any install
  s  (Cfg)  rice sync

UNDO
  macmgr snapshot create --name before
  macmgr rice pull sketchybar --yes
  macmgr snapshot undo --yes

OTHER
  macmgr pkg install ripgrep | cask:firefox
  macmgr catalog install clock --reload
  macmgr doctor / watch / export ~/Desktop/my-rice.tar.gz
  docs: USAGE.ru.md · CHEATSHEET.ru.md
  System Settings → Privacy → Accessibility → yabai & skhd
"#
    .into()
}
