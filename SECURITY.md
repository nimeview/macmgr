# Security Policy

## Supported versions

Public preview builds (`0.2.x`) receive fixes on a best-effort basis.

## What macmgr does on your Mac

- Reads/writes configs under `~/.config`, rice under `~/.local/share/macmgr`
- Installs Homebrew packages and taps declared in `manifest.toml`
- Starts LaunchAgents for yabai / skhd / sketchybar / borders
- Does **not** disable SIP or grant Accessibility for you — it only prints hints

## Reporting a vulnerability

Please open a **private** GitHub security advisory on this repository, or email
the maintainer listed in `Cargo.toml` / GitHub profile.

Do not file public issues for undisclosed security bugs.

Include: macOS version, macmgr version (`macmgr --version`), and steps to reproduce.
