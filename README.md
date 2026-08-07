# macmgr

TUI/CLI‑менеджер macOS‑сборки: **yabai** + **skhd** + **SketchyBar** + **borders**.

Полная инструкция (RU): **[docs/USAGE.ru.md](docs/USAGE.ru.md)** · шпаргалка: **[docs/CHEATSHEET.ru.md](docs/CHEATSHEET.ru.md)**

## Зачем

Управлять **живой** оболочкой (`~/.config/...`), а не чужим upstream‑форком:

| Слой | Путь | Роль |
|------|------|------|
| **LIVE** | `~/.config/…` | Источник правды каждый день |
| **My rice** | `~/.local/share/macmgr/rice` | Зеркало для selective sync и GitHub |
| **Pack** | `.tar.gz` | Перенос сборки на другой Mac |

Felix/`~/.dotfiles` **не** используется для DIFF/sync — только историческая основа.

## Установка

```bash
cd ~/Documents/MacOS_Manager   # или клон репо
cargo install --path .
mkdir -p ~/.config/macmgr
cp manifest.toml ~/.config/macmgr/
macmgr doctor
macmgr rice init --git          # опционально: зеркало из LIVE
```

Бинарник: `macmgr`. Без аргументов открывается **TUI**.

## Возможности

- **Сервисы** — start/stop/restart yabai, skhd, sketchybar, borders  
- **Packages** — список из manifest + **любой** brew formula/cask (`a` в TUI / `macmgr pkg install`)  
- **Configs** — правка LIVE в nano, diff vs rice, Finder  
- **Plugins** — enable/disable SketchyBar items/widgets, auto‑reload, Lua validate  
- **Catalog** — builtin + URL + git + **community feeds** (`catalog pull`)  
- **Hotkeys** — CRUD по LIVE `skhdrc`, в т.ч. **multi-line** (`\` / `;;`)  
- **Rules** — yabai rules toggle/add  
- **Themes / Profiles** — цвета бара, work/game профили  
- **Rice sync** — selective push/pull/diff LIVE ↔ My rice  
- **Snapshots + undo** — именованные снимки, откат после pull/import/restore  
- **Export / Import** — один `.tar.gz` на другую машину  
- **Watch** — авто‑reload бара при правках `.lua`  
- **Doctor** — SIP, Accessibility, bins, rice path  

## TUI

```text
1 Dash · 2 Svc · 3 Pkgs · 4 Cfg · 5 Plug · 6 Logs · 7 Doc · 8 More
```

| Где | Клавиши |
|-----|---------|
| Везде | `?` help · **`:` palette** · `/` filter · `j/k` · `q` · `r` refresh |
| **Pkgs** | Enter install/uninstall · **`a` любой brew** · **`s` brew search** · `u` upgrade |
| **Cfg** | Enter/`e` nano · `s` rice sync · `d` diff · `o` Finder |
| **Plug** | Enter toggle · `e` nano · `i` catalog · `a`/`A` stub |
| **More** | секции Daily / presets / safety / install · Enter |
| **Catalog** | Enter install · `u` from git/URL · **`f` pull feeds** |
| **Snapshots** | Enter restore · `n` name · `u` undo · `d` delete |

Снапшот оболочки (v0.1): каталог **[shell/](shell/)** + My rice.

## CLI (шпаргалка)

```bash
macmgr                         # TUI
macmgr doctor | status | tips

# brew — любой пакет, не только из manifest
macmgr pkg list
macmgr pkg search neovim
macmgr pkg install ripgrep
macmgr pkg install cask:firefox
macmgr pkg upgrade
macmgr pkg uninstall foo --yes

# LIVE configs
macmgr config list | edit sketchybar | validate | diff

# SketchyBar plugins + catalog
macmgr plugin list | add mystats --kind widget --reload
macmgr catalog list
macmgr catalog pull
macmgr catalog install clock --reload
macmgr catalog from-url https://github.com/user/repo.git --id x --path path/x.lua --reload
macmgr hotkeys add --keys 'alt - r' --action 'cmd1 ;; cmd2'

# skhd
macmgr hotkeys list | toggle 3 | add --keys 'alt - x' --action '…'

# My rice
macmgr rice init --git
macmgr rice status | push | pull --yes | diff

# safety net
macmgr snapshot create --name before-edit
macmgr snapshot undo --yes
macmgr export ~/Desktop/my-rice.tar.gz
macmgr import ~/Desktop/my-rice.tar.gz --yes

macmgr watch
macmgr update-core --yes
```

## Архитектура (кратко)

```
src/
  main.rs + cli.rs          CLI entry
  tui/                      ratatui app (state / keys / actions / ui)
  core/                     brew, config, rice, plugins, catalog, hotkeys,
                            snapshot, pack, themes, profiles, rules, …
  util/                     editor (nano для TUI), terminal guard, shell
manifest.toml               packages, services, configs → rice paths
```

Опасные операции (pull, import, uninstall, restore) спрашивают confirm; `--yes` для скриптов. Перед pull/import/restore кладётся **undo‑snapshot**.

## Важно

- SIP и Accessibility macmgr **не меняет** — только подсказывает в `doctor`.  
- Источник правды бара — **LIVE** `~/.config/sketchybar`.  
- После правок Lua: `luac -p` (валидация) → только потом reload.  

## Лицензия

MIT
