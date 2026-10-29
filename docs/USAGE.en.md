# macmgr — English usage guide

Russian: [USAGE.ru.md](USAGE.ru.md) · Install: [INSTALL.md](INSTALL.md) · Architecture: [ARCHITECTURE.md](ARCHITECTURE.md)

## Mental model

| Layer | Path | Role |
|-------|------|------|
| LIVE | `~/.config/…` | What runs daily |
| My rice | `~/.local/share/macmgr/rice` | Pinned mirror |
| Starter | `shell/` | Seeded by `macmgr setup` |
| user.toml | App Support `…/macmgr/user.toml` | Your tracked pkgs/configs |

## Everyday

```bash
macmgr                 # TUI
macmgr doctor
macmgr status --json
macmgr tips
```

### Packages

```bash
macmgr pkg list
macmgr pkg search neovim
macmgr pkg install ripgrep
macmgr pkg add jq                      # track in user.toml
macmgr pkg add cask:firefox --install
macmgr pkg remove jq
```

TUI **Pkgs**: `a` install+track · `t` track · `x` untrack · `s` search.

### Configs

```bash
macmgr config list
macmgr config edit sketchybar
macmgr config add nvim ~/.config/nvim
macmgr config remove nvim
```

TUI **Cfg**: `a` add · `x` remove · `e` edit · `s` rice sync · `d` diff.

### Themes

```bash
macmgr theme list
macmgr theme apply tokyo --reload   # SketchyBar + kitty
```

### Rice

```bash
macmgr rice init --git
macmgr rice status
macmgr rice push sketchybar
macmgr rice pull --yes
macmgr rice commit -m "pin bar"
```

### Safety

```bash
macmgr snapshot create --name before
macmgr snapshot undo --yes
macmgr export ~/Desktop/rice.tar.gz
macmgr import ~/Desktop/rice.tar.gz --yes
```

## First-time machine

```bash
macmgr setup --yes
# then Accessibility for yabai & skhd — see INSTALL.md
```

## RU keyboard in TUI

Physical keys are mapped ЙЦУКЕН → QWERTY so `q`/`j`/`k` hotkeys work on a Russian layout.
