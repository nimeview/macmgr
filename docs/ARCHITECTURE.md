# Architecture

## Layers

```text
┌─────────────────────────────────────────────────────────┐
│  macmgr (CLI / TUI)                                     │
│  manifest.toml + user.toml overlay                      │
└───────────────┬───────────────────────────┬─────────────┘
                │                           │
                ▼                           ▼
┌───────────────────────────┐   ┌───────────────────────────┐
│ LIVE  ~/.config/…         │   │ My rice                   │
│ daily source of truth     │◄─►│ ~/.local/share/macmgr/rice│
│ sketchybar, skhd, yabai…  │   │ selective push/pull/diff  │
└───────────────────────────┘   └───────────────────────────┘
                ▲
                │ setup / install plant
                │
┌───────────────────────────┐   ┌───────────────────────────┐
│ Starter  shell/ (repo)    │   │ Pack  .tar.gz             │
│ first-boot snapshot       │   │ export / import machine   │
└───────────────────────────┘   └───────────────────────────┘
```

## Mental model

| Name | Path | Role |
|------|------|------|
| LIVE | `~/.config/…` | What runs every day |
| My rice | `~/.local/share/macmgr/rice` | Pinned mirror for git / sync |
| Starter | `shell/` in this repo | Seeded into rice on `setup` / first `install` |
| user.toml | Application Support `…/macmgr/user.toml` | Packages/configs added from TUI/CLI |
| Themes | Application Support `…/macmgr/themes/` | Generated `*_colors.lua` + kitty theme |

Felix/`~/.dotfiles` is **not** part of the sync model.

## Manifest

- Shipped: [`manifest.toml`](../manifest.toml) (`manifest_version = 1`)
- User overlay: `user.toml` (merge on load; add/remove via `pkg`/`config` commands)
- Newer `manifest_version` than the binary supports → hard error (upgrade macmgr)

## Theme apply

`macmgr theme apply <id>`:

1. Copies `colors.lua` into LIVE SketchyBar
2. Writes `~/.config/kitty/current-theme.conf` (opaque selection)
3. Optionally reloads the bar (`--reload`)

## Services

Controlled via LaunchAgents / native yabai·skhd service helpers. See `macmgr service list`.

## Safety

Destructive ops (pull, import, restore, uninstall) confirm unless `--yes`.  
Pull/import/restore push an **undo snapshot** first when possible.  
`macmgr setup` snapshots existing LIVE configs before seeding.
