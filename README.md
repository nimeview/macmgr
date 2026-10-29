# macmgr

TUI/CLI manager for a macOS tiling rice: **yabai** + **skhd** + **SketchyBar** + **borders** (+ kitty themes).

Russian docs: **[docs/USAGE.ru.md](docs/USAGE.ru.md)** · **[docs/CHEATSHEET.ru.md](docs/CHEATSHEET.ru.md)**

English: **[docs/USAGE.en.md](docs/USAGE.en.md)** · **[docs/INSTALL.md](docs/INSTALL.md)** · **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)**

> **Requires macOS.** yabai’s scripting addition may need partially disabling SIP — see [docs/INSTALL.md](docs/INSTALL.md).

## Why

Manage your **live** desktop (`~/.config/…`), not someone else’s upstream fork:

| Layer | Path | Role |
|-------|------|------|
| **LIVE** | `~/.config/…` | Daily source of truth |
| **My rice** | `~/.local/share/macmgr/rice` | Selective sync / git mirror |
| **Starter** | [`shell/`](shell/) | First-boot snapshot |
| **Pack** | `.tar.gz` | Move the rice to another Mac |

## Project tour

macmgr turns a collection of macOS tiling-window-manager tools into one
manageable environment. Instead of remembering separate commands and config
locations for yabai, skhd, SketchyBar, borders and kitty, you can inspect,
change, sync and recover the setup from one TUI or CLI.

### TUI dashboard

```text
┌ macmgr · Arthur Rice ──────────────────────────────────────────────────────┐
│  1/Dash  │  2/Svc  │  3/Pkgs  │  4/Cfg  │  5/Plug  │  6/Logs  │  7/Doc  │
├ Dashboard ──────────────────────────────────────────────────────────────────┤
│  Rice overview (LIVE ~/.config)                                             │
│                                                                             │
│  Services: 0/4 core running     Packages: 0/0     Configs: 1 present       │
│  Plugins: 0                     Doctor: 0 warnings  Watch: off             │
│                                                                             │
│  ? help · : palette · 1-8 tabs · r refresh · q quit                        │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Start with a real desktop, not a template

`macmgr setup` checks the machine, installs the required pieces, and seeds the
starter configuration. From that point on, the running configuration stays the
source of truth.

```text
starter config ── setup ──► LIVE ~/.config/… ◄──► My rice / git mirror
                                  │
                                  ├── services, hotkeys and plugins
                                  ├── packages and tracked user config
                                  └── SketchyBar + kitty themes
```

### Change, compare, and recover

Use the dashboard to control services, edit tracked config, manage packages,
toggle plugins, and switch themes. Before risky operations such as import,
pull or restore, macmgr creates an undo snapshot when possible.

The same setup can be exported as one archive and moved to another Mac.

### Built for day-to-day maintenance

The project combines a keyboard-first TUI with a scriptable CLI. It covers
service control, package tracking, configuration diffs, plugin catalogs,
hotkeys, themes, rice sync, snapshots, diagnostics and import/export.

Detailed commands, installation notes and the architecture are kept in
[`docs/`](docs/) so this page can stay focused on the project itself.

## Run locally

```bash
cargo install --path .
cp manifest.toml ~/.config/macmgr/
macmgr setup --yes
macmgr
```

Or use [`scripts/bootstrap.sh`](scripts/bootstrap.sh) on a fresh Mac.

## License

MIT — see [LICENSE](LICENSE) and [NOTICE](NOTICE).
