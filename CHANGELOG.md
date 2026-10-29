# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] — 2026-08-11

### Added

- `macmgr setup` — first-run wizard (seed `shell/` → rice → install core → doctor)
- Install plants starter `shell/` when rice mirror is empty
- Doctor: kitty.conf, skhdrc, spaces count, shell pack, actionable `fix:` hints
- `macmgr doctor --json` / `macmgr status --json`
- User overlay `user.toml`: `pkg add|remove`, `config add|remove` (CLI + TUI)
- Theme apply also writes kitty `current-theme.conf` with opaque selection
- TUI RU layout key mapping (ЙЦУКЕН → QWERTY)
- Docs: INSTALL, ARCHITECTURE, USAGE.en, CONTRIBUTING, SECURITY, CODE_OF_CONDUCT
- CI: fmt, clippy, intel + arm; Release workflow for `v*` tags
- Homebrew formula stub `Formula/macmgr.rb`
- `manifest_version = 1` schema gate
- LICENSE + NOTICE

### Changed

- Version bump to **0.2.0** (public preview)
- Theme catalog paths are relative (no absolute user home paths in repo)
- README rewritten for English-first public entry

### Added (post-0.2 polish)

- `macmgr purge` — full wipe of rice environment, brew packages, LIVE/rice/macmgr data

## [0.1.0] — 2026-08-07

### Added

- Initial macmgr TUI/CLI: services, packages, configs, plugins, catalog, hotkeys,
  rice sync, snapshots, themes, profiles, export/import
- Starter `shell/` snapshot (SketchyBar / yabai / skhd / borders)

[Unreleased]: https://github.com/arturpanan/MacOS_Manager/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/arturpanan/MacOS_Manager/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/arturpanan/MacOS_Manager/releases/tag/v0.1.0
