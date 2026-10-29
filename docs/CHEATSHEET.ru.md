# Шпаргалка macmgr

Полная инструкция: [USAGE.ru.md](USAGE.ru.md)

## Модель

```text
LIVE (~/.config)  = правда каждый день
My rice           = закрепить (rice push)
DIFF              = ещё не закрепил — ок
```

## День за днём

| Хочу | Делаю |
|------|--------|
| Открыть UI | `macmgr` |
| Первый Mac | `macmgr setup --yes` · [INSTALL.md](INSTALL.md) |
| Править бар | Cfg/`e` или Plug/`e` → nano |
| Любой brew | Pkgs `a`/`t`/`x` · `pkg add` / `pkg install` |
| Добавить конфиг | Cfg `a` · `config add nvim ~/.config/nvim` |
| Hotkey | More → Hotkeys · `;;` = новая строка |
| Закрепить | `rice push` → `rice commit` → `git-push` |
| Откат | `snapshot undo` / `rice pull --yes` |
| Другой Mac | `export` → `import` → `doctor` |
| JSON | `doctor --json` · `status --json` |

## Клавиши TUI

| | |
|--|--|
| `:` | палитра |
| `1`–`8` | вкладки |
| `?` | help |
| Cfg `s` | rice sync |
| Pkgs `s` | brew search |
| Catalog `f` | pull feeds |

## CLI одной строкой

```bash
macmgr tips | doctor | status
macmgr rice status | push | pull --yes | commit -m '…' | git-push
macmgr catalog pull | list | install ID --reload
macmgr hotkeys list | add --keys '…' --action 'a ;; b'
macmgr snapshot create --name before | undo --yes
macmgr export ~/Desktop/rice.tar.gz
```

## Пути

- Manifest: `~/.config/macmgr/manifest.toml`  
- Rice: `~/.local/share/macmgr/rice`  
- Snaps: `~/.local/share/macmgr/snapshots`  
- Feeds cache: `~/.local/share/macmgr/community-catalog.toml`
