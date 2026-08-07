# macmgr — подробная инструкция

Менеджер твоей macOS-сборки: **yabai + skhd + SketchyBar + borders**.

- **LIVE** (`~/.config/...`) — источник правды каждый день  
- **My rice** (`~/.local/share/macmgr/rice`) — зеркало для selective sync и публикации  
- Felix / `~/.dotfiles` **не** используется для DIFF/sync (историческая основа сборки)

**Оглавление:** [ментальная модель](#как-этим-пользоваться-самое-важное) · [карта путей](#карта-путей) · [установка](#установка) · [TUI](#tui-интерактивный-режим) · [CLI](#cli--все-команды) · [манифест](#манифест) · [сценарии](#типичные-сценарии) · [snapshots / pack](#snapshots-undo-export--import) · [FAQ](#faq--типичные-вопросы) · [что дальше](#что-дальше)

---

## Как этим пользоваться (самое важное)

Думай так:

```text
  правишь LIVE ──► нравится? ──► rice push (= закрепить в «свою сборку»)
       │                              │
       │                              ▼
       │                         git (GitHub) / export .tar.gz
       ▼
  не понравилось? ──► snapshot undo / rice pull
```

### Три слова

| Слово | Где лежит | Зачем |
|-------|-----------|--------|
| **LIVE** | `~/.config/sketchybar`, `skhd`, `yabai`… | То, что **сейчас** крутится на Mac. Правишь каждый день. |
| **My rice** | `~/.local/share/macmgr/rice` | Копия LIVE, которую ты **осознанно сохранил**. Для GitHub и другой машины. |
| **DIFF** | статус в TUI / `rice status` | LIVE и rice **разъехались**. Не ошибка — значит есть незакреплённые правки. |

«Дефолтное окружение» = то, что у тебя в **LIVE** сейчас.  
«Закрепить в сборку» = скопировать в **My rice** (`rice push`).

### Сценарий A — просто поправить бар / hotkey

```bash
macmgr                          # TUI
# Configs → sketchybar → e (nano) → сохранить → выйти
# или Plugins → e на виджет
# или More → Hotkeys → a / Enter toggle
```

После выхода из nano macmgr сам проверит Lua и сделает reload.  
**На этом можно остановиться** — LIVE уже изменён. Rice трогать не обязательно.

### Сценарий B — «хочу, чтобы это осталось в моей сборке»

Когда правка удалась и ты хочешь её **закрепить** (бэкап / GitHub / другой Mac):

```bash
macmgr rice status              # где DIFF?
macmgr rice push sketchybar     # только бар
# или
macmgr rice push                # всё managed из manifest
```

В TUI: Configs → **`s`** (Rice sync) → Enter/`p` на нужном конфиге (или `a` = push all).

Потом, если хочешь в GitHub:

```bash
macmgr rice commit -m "sketchybar: battery tweak"
# один раз: cd ~/.local/share/macmgr/rice && git remote add origin <url>
macmgr rice git-push
```

### Сценарий C — добавить brew-пакет «в сборку навсегда»

1. Поставить: `macmgr pkg install jq` (или TUI Pkgs → **`a`**)  
2. Чтобы на новом Mac он ставился с rice — добавь в `~/.config/macmgr/manifest.toml` секцию `[[packages]]` и скопируй манифест в репо / rice при желании.

Пока пакет только установлен через brew — он **на этой машине**.  
В «определение сборки» он попадает через **manifest.toml**.

### Сценарий D — откатить

```bash
macmgr snapshot create --name before-experiment
# … сломал …
macmgr snapshot undo --yes      # если был pull/import/restore
# или
macmgr rice pull sketchybar --yes   # вернуть бар из rice (с undo)
```

### Синк / rice ops — шпаргалка

| Команда | Направление | Когда |
|---------|-------------|--------|
| `rice init --git` | LIVE → создать rice | Один раз |
| `rice status` | — | Посмотреть DIFF |
| `rice push [id]` | **LIVE → rice** | Закрепить удачные правки |
| `rice pull [id]` | **rice → LIVE** | Откатить LIVE к закреплённому (осторожно) |
| `rice commit` | git commit в rice | После push — зафиксировать в git |
| `rice git-push` | git push origin | На GitHub (нужен remote) |
| `rice diff [id]` | сравнить | Понять, что разъехалось |
| `rice reload` | сервисы | Перезапуск yabai/skhd/bar |

**Правило:** в повседневке почти всегда **push**.  
**pull** — только когда хочешь вернуть LIVE к уже сохранённой версии в rice.

### Чеклист на день

1. `macmgr` → правь LIVE (Cfg / Plug / Hotkeys)  
2. Если бар ок — **готово** (rice не обязателен)  
3. Если правка «навсегда» → `rice push` → опционально `rice commit` / `git-push`  
4. Перед рискованным экспериментом → `snapshot create --name …`

---

## Карта путей

| Что | Путь | Кто пишет |
|-----|------|-----------|
| Манифест менеджера | `~/.config/macmgr/manifest.toml` | ты / `self-install` |
| Локальный catalog | `~/.config/macmgr/catalog.toml` | catalog add / defaults |
| Feeds (URL’ы) | `~/.config/macmgr/feeds.toml` | `catalog feed-add` (seed auto) |
| Themes | `~/Library/Application Support/dev.macmgr.macmgr/themes/` | `theme apply` |
| Community cache | `~/.local/share/macmgr/community-catalog.toml` | `catalog pull` |
| My rice | `~/.local/share/macmgr/rice/` | `rice init` / `push` |
| Snapshots + undo | `~/.local/share/macmgr/snapshots/` | snapshot / pull / import |
| LIVE bar | `~/.config/sketchybar/` | ты + plugins/catalog |
| LIVE skhd | `~/.config/skhd/skhdrc` (см. manifest) | hotkeys CRUD |
| LIVE yabai | `~/.config/yabai/…` | ты + rules |
| Логи | `/tmp/yabai_*.err.log`, `skhd_*.err.log` | сервисы |
| Pack export | любой `.tar.gz` | `export` / `import` |

Редактор в TUI: **nano** (или `$MACMGR_EDITOR`).  
Редактор в CLI `config edit`: **`$EDITOR` / `$VISUAL`**.

---

## Установка

```bash
cd ~/Documents/MacOS_Manager
cargo install --path .
mkdir -p ~/.config/macmgr
cp manifest.toml ~/.config/macmgr/manifest.toml
```

Проверка:

```bash
macmgr --version
macmgr tips
macmgr doctor
```

На новом Mac (после правки `REPO_URL` в скрипте):

```bash
bash scripts/bootstrap.sh
```

---

## Быстрый старт

| Цель | Команда |
|------|---------|
| Открыть TUI | `macmgr` |
| Статус сервисов | `macmgr status` |
| Диагностика | `macmgr doctor` |
| Список плагинов бара | `macmgr plugin list` |
| Добавить виджет | `macmgr plugin add myclock --kind widget --reload` |
| Править LIVE-конфиг | `macmgr config edit sketchybar` |
| Перезапуск сборки | `macmgr rice reload` |
| Снимок конфигов | `macmgr snapshot` |

---

## TUI (интерактивный режим)

Запуск: `macmgr` или `macmgr tui`.

### Вкладки (клавиши `1`–`8`)

1. **Dash** — обзор  
2. **Svc** — сервисы  
3. **Pkgs** — brew из manifest + **`a` любой** · **`s` search** · `u` upgrade · `/` filter  
4. **Cfg** — LIVE (`e` nano, `s` rice sync, `d` diff) · **`:`** палитра везде  
5. **Plug** — плагины (`i` catalog, auto-reload)  
6. **Logs** — логи (+ toast при новых ошибках stderr)  
7. **Doc** — doctor (Enter → связанная вкладка)  
8. **More** — install, themes, hotkeys, snapshots, rice, export…  

### Общие клавиши

| Клавиша | Действие |
|---------|----------|
| `:` | **Command palette** — поиск действий (вкладки, rice, snaps, brew…) |
| `h` / `l`, Tab | Вкладки |
| `j` / `k` | Выбор (списки скроллятся) |
| Enter | Действие |
| `/` | Фильтр текущего списка |
| `?` | Help |
| `r` | Refresh (по вкладке) |
| `R` | Reload rice (или bar на Plugins) |
| `q` | Выход |

Toast (правый верх): успех/ошибка действий + новые строки в `*.err.log` (~каждые 5 с).

### More (важные пункты)

| Пункт | Зачем |
|-------|--------|
| Themes / Profiles | Цвета бара, work/game |
| Yabai rules / Hotkeys | Toggle и CRUD |
| Plugin catalog | install + feeds |
| Snapshots / Undo | Именованные снимки и откат |
| Export / Import | Pack на другой Mac |
| Rice sync / Init rice | LIVE ↔ My rice |
| Watch-mode | Авто-reload `.lua` |

---

## More menu

Вкладка **More** разбита на секции (заголовки `── … ──` не кликабельны):

1. **Daily** — themes, profiles, hotkeys, rules, catalog, wallpaper, watch, spaces, reload  
2. **Layout presets** — compact / comfortable / spacious  
3. **Safety / sync** — snapshots, undo, rice, backup, export/import  
4. **Install (rare)** — CORE/FULL install, brew upgrade core  

Частые вещи также через **`:`** palette.

---

```bash
macmgr theme list
macmgr theme apply dim --reload
macmgr theme backup

macmgr profile list
macmgr profile apply work
macmgr profile save mysetup --label "My setup"

macmgr catalog list
macmgr catalog install clock --reload
macmgr catalog from-url https://github.com/org/repo.git --id myplug --path path/to.lua --reload
macmgr catalog add myplug --source https://github.com/org/repo.git --path path/to.lua
macmgr hotkeys list
macmgr hotkeys add --keys 'alt - x' --action 'yabai -m window --close'
macmgr hotkeys toggle 3

macmgr rules list
macmgr rules toggle 36
macmgr rules add Discord --manage-off

macmgr watch --interval 800
macmgr update-core --yes
```

В TUI всё это доступно во вкладке **More**.

### Configs

| Клавиша | Действие |
|---------|----------|
| Enter / `e` / `E` | Открыть LIVE в **nano** (или `$MACMGR_EDITOR`) |
| `s` | Rice sync overlay (LIVE ↔ My rice) |
| `o` | Finder |
| `d` | Diff LIVE vs rice |
| `b` | Backup всех LIVE-конфигов |

Для папок (sketchybar): список файлов → Enter/`e` → nano. TUI на паузе, пока редактор открыт.

После правки `.lua`: **валидация синтаксиса** (`luac -p`) → только потом reload. Если ошибка — бар не трогаем.

CLI по-прежнему: `macmgr config edit …` → `$EDITOR` / `$VISUAL` (hx и т.п.).
Проверка без reload: `macmgr config validate`.

`DIFF` = LIVE отличается от **My rice** (`~/.local/share/macmgr/rice`). Это нормально, пока не сделаешь `rice push`.

### Plugins

| Клавиша | Действие |
|---------|----------|
| Enter | Вкл/выкл плагин |
| `e` / `E` | Редактировать `.lua` в nano |
| `a` | Новый **widget**-stub |
| `A` | Новый **item**-stub |
| `d` | Выключить |
| `R` | Перезапустить только sketchybar |

Плагины = файлы в `~/.config/sketchybar/items/` и `items/widgets/`.
Вкл/выкл правит `require(...)` в `items/init.lua` / `widgets/init.lua`
(выключенные строки помечаются `-- macmgr:off`).

### Packages / Install

Список во вкладке **Pkgs** — пакеты из `manifest.toml`.

**Любой** brew‑пакет (не из списка):

| TUI | CLI |
|-----|-----|
| `a` → имя или `cask:firefox` | `macmgr pkg install ripgrep` |
| **`s` → brew search → Enter install** | `macmgr pkg install cask:firefox` |
| | `macmgr pkg search query` |

Enter на строке — install / uninstall. `u` — upgrade выбранного.

Перед brew TUI **снимает** alternate screen, показывает вывод, по Enter возвращается.

---

## CLI — все команды

### `macmgr` / `macmgr tui`

Интерактивный интерфейс.

### `macmgr status`

Краткий статус сервисов, outdated brew, число конфигов.

### `macmgr doctor`

Проверки: brew, бинарники, сервисы, SIP, LaunchAgents, пути конфигов, outdated.

### `macmgr tips`

Шпаргалка полезных следующих шагов.

---

### Установка / удаление сборки

```bash
macmgr install --scope core          # ядро: taps + yabai/skhd/sketchybar/borders + конфиги
macmgr install --scope full --yes    # + пакеты из манифеста (без optional)
macmgr uninstall --keep-configs --yes
macmgr self-install                  # cargo install + копирует manifest в ~/.config/macmgr
```

`--yes` пропускает подтверждение.

---

### Сервисы

```bash
macmgr service list
macmgr service start yabai
macmgr service stop skhd
macmgr service restart sketchybar
macmgr service restart all
```

yabai/skhd управляются через `--start-service` / `--restart-service`,
sketchybar/borders — через `brew services`.

---

### Пакеты (Homebrew)

```bash
macmgr pkg list
macmgr pkg search borders
macmgr pkg install borders
macmgr pkg uninstall borders --yes
macmgr pkg upgrade borders
macmgr pkg upgrade                 # все
macmgr pkg update-all --yes        # brew update && upgrade
```

Список берётся из `manifest.toml` (+ ad-hoc install по имени).

---

### Конфиги (всегда LIVE)

```bash
macmgr config list
macmgr config files sketchybar     # все файлы в LIVE-папке
macmgr config edit yabai           # $EDITOR
macmgr config edit sketchybar
macmgr config open kitty           # Finder
macmgr config backup
macmgr config restore --yes
macmgr config diff                 # LIVE vs My rice
```

ID конфигов: `yabai`, `skhd`, `sketchybar`, `borders`, `kitty`, `helix`, `starship`, `zsh`, `tmux`.

---

### Hotkeys (skhd)

```bash
macmgr hotkeys list
macmgr hotkeys toggle 3
macmgr hotkeys add --keys 'alt - x' --action 'open -a Calculator'
macmgr hotkeys add --keys 'alt - r' --action 'yabai -m space --rotate 90 ;; yabai -m space --balance'
macmgr hotkeys set 3 --keys 'alt - y' --action '…'
macmgr hotkeys delete 3 --yes
```

Выключение — префикс `# macmgr:off ` в LIVE `skhdrc` (как у yabai rules).

TUI → More → **Hotkeys CRUD…**: Enter toggle · `a` add (`;;` = новая строка) · `d` delete · `e` nano.

Парсятся bindings с `\` continuation в skhdrc; toggle/delete работают на весь блок.

### Catalog (builtin / URL / git / feeds)

```bash
macmgr catalog pull                          # community feeds → cache
macmgr catalog feeds
macmgr catalog feed-add https://example.com/plugins.toml --id friends
macmgr catalog list                          # local + feed
macmgr catalog install clock --reload
macmgr catalog from-url https://raw.githubusercontent.com/…/x.lua --id foo --reload
macmgr catalog from-url https://github.com/user/repo.git --id bar --path items/bar.lua --reload
macmgr catalog add bar --source https://github.com/user/repo.git --path items/bar.lua
```

TUI → catalog · Enter install · `u` URL · **`f`/`p` pull feeds**.

Builtin feed: `feeds/community.toml` (вшит как `builtin:community`).

### Плагины SketchyBar

```bash
macmgr plugin list
macmgr plugin enable media --reload
macmgr plugin disable widgets.cpu --reload
macmgr plugin toggle calendar --reload

macmgr plugin add weather --kind widget --reload
macmgr plugin add banner --kind item --reload

macmgr plugin edit widgets.battery
macmgr plugin remove weather --delete --yes --reload
macmgr plugin reload
```

**kind:**

- `item` → `~/.config/sketchybar/items/<name>.lua`
- `widget` → `~/.config/sketchybar/items/widgets/<name>.lua`

После правок Lua почти всегда нужен `--reload` или `macmgr plugin reload`.

Пример цикла «новый виджет»:

```bash
macmgr plugin add mystats --kind widget
macmgr plugin edit mystats          # пишешь логику
macmgr plugin reload
```

---

### Rice / пресеты / обои / hotkeys / логи

```bash
# My rice mirror (не Felix)
macmgr rice init --git                 # создать ~/.local/share/macmgr/rice из LIVE
macmgr rice status
macmgr rice push                       # все managed LIVE → rice
macmgr rice push sketchybar            # один конфиг
macmgr rice push sketchybar --file items/widgets/battery.lua
macmgr rice pull sketchybar --yes      # rice → LIVE (undo first)
macmgr rice diff
macmgr rice diff sketchybar
macmgr rice reload                     # restart core services

# git внутри My rice (после rice push):
macmgr rice git-status
macmgr rice commit -m "my bar tweaks"
macmgr rice git-push                   # нужен remote: git remote add origin …

# legacy aliases:
# macmgr rice sync --direction to-repo|from-repo

macmgr preset list
macmgr preset apply compact               # padding/gap/opacity в yabai (runtime)
macmgr preset apply comfortable
macmgr preset apply spacious

macmgr wallpaper list
macmgr wallpaper set 0
macmgr wallpaper set alps                 # по подстроке имени

macmgr hotkeys list                           # LIVE skhdrc
macmgr hotkeys toggle 12                      # # macmgr:off
macmgr hotkeys add --keys 'alt - x' --action 'open -a Calculator'
macmgr hotkeys add --keys 'alt - r' --action 'cmd1 ;; cmd2'   # multi-line
macmgr hotkeys set 12 --action '…'
macmgr hotkeys delete 12 --yes
macmgr logs list
macmgr logs yabai-err --lines 100
macmgr logs skhd-err
```

После `rice init --git` публикация на GitHub:

```bash
cd ~/.local/share/macmgr/rice
git remote add origin git@github.com:YOU/your-rice.git
git add -A && git commit -m "init"
git push -u origin main
```

---

### Snapshot и spaces

```bash
macmgr snapshot                 # = snapshot create
macmgr snapshot create
macmgr snapshot list            # ~/.local/share/macmgr/snapshots/

macmgr spaces                   # ~/bin/ensure_spaces.sh
```

Snapshot копирует все LIVE-конфиги из манифеста — удобный бэкап перед экспериментами с плагинами.

---

## Манифест

Файл: [`manifest.toml`](../manifest.toml)  
После `self-install` предпочтительно: `~/.config/macmgr/manifest.toml`

Там задаются:

- taps, сервисы, пакеты  
- пути LIVE-конфигов  
- defaults macOS  
- пресеты yabai  
- post_install шаги  

Редактируй манифест, чтобы менеджер знал о новых программах/путях.

---

## Типичные сценарии

### 1. Добавить виджет в бар

```bash
macmgr plugin add ram --kind widget --reload
macmgr plugin edit ram
macmgr plugin reload
```

### 2. Временно убрать CPU с бара

```bash
macmgr plugin disable widgets.cpu --reload
```

### 3. Поправить свой SketchyBar (LIVE)

```bash
macmgr config files sketchybar
macmgr config edit sketchybar   # или TUI → Configs → sketchybar
macmgr plugin reload
```

### 4. Поставить borders

```bash
macmgr pkg install borders
macmgr service start borders
macmgr doctor
```

### 5. Бэкап перед правками

```bash
macmgr snapshot
# ... эксперименты ...
# при проблемах: macmgr config restore --yes  (из config backup)
# или ручной откат из ~/.local/share/macmgr/snapshots/
```

---

## Snapshots, Undo, Export / Import

### Snapshots + undo

```bash
macmgr snapshot create --name before-battery
macmgr snapshot list
macmgr snapshot restore 20260807-031251_before-battery --yes
macmgr snapshot undo --yes          # откат последнего restore/import/sync
macmgr snapshot delete <id> --yes
```

Каждый **restore / import / rice pull** сначала кладёт текущий LIVE в undo-стек  
(`~/.local/share/macmgr/snapshots/` + undo-stack).

В TUI → **More**:
- **Snapshots…** — список · Enter restore · `n` имя · `u` undo · `d` удалить  
- **Undo last restore/import**  
- **Rice sync…** / Configs `s` — selective LIVE ↔ My rice  
- **Export / Import rice pack…**

### Export / Import pack

Архив `.tar.gz`: LIVE-конфиги + `manifest.toml` + список brew + enabled plugins + `PACK.toml`.

```bash
macmgr export ~/Desktop/my-rice.tar.gz
macmgr import ~/Desktop/my-rice.tar.gz --yes
macmgr import ~/Desktop/my-rice.tar.gz --yes --install-packages   # ещё и brew
```

На новой машине: поставить macmgr → `import` → `doctor` → `service start` при необходимости.

---

## FAQ / типичные вопросы

**Чем LIVE отличается от rice?**  
LIVE — то, что запущено сейчас. Rice — закреплённая копия «моей сборки». Правишь LIVE; push — когда хочешь закрепить.

**Почему всегда DIFF?**  
Ты правил LIVE и ещё не делал `rice push`. Это нормально.

**Нужен ли rice каждый день?**  
Нет. Достаточно LIVE. Rice — для бэкапа, git и другой машины.

**Сломал бар после правки Lua**  
Safe reload: validate → undo snapshot → restart; если процесс не поднялся — откат undo.  
Руками: `macmgr snapshot undo --yes` или `rice pull sketchybar --yes`.

**Как добавить пакет «в определение сборки»?**  
`pkg install` ставит на машину. Чтобы ставилось на новом Mac — добавь `[[packages]]` в `manifest.toml` и храни манифест в rice/git/pack.

**Где править hotkeys?**  
LIVE `skhdrc` через More → Hotkeys или `macmgr hotkeys …`. Multi-line: `;;` в action или `\` в файле.

**Как поставить чужой виджет?**  
`catalog from-url …` / TUI Catalog `u`, или `catalog pull` + install из feed.

**Felix / ~/.dotfiles всё ещё нужен?**  
Нет для sync/DIFF. Может лежать как история — macmgr на него не опирается.

**SIP / Accessibility**  
macmgr только подсказывает в `doctor`. Включить Accessibility для yabai/skhd — в System Settings вручную.

**TUI vs CLI редактор**  
TUI → nano (`MACMGR_EDITOR`). CLI `config edit` → `$EDITOR` (например hx).

---

## Глоссарий

| Термин | Смысл |
|--------|--------|
| LIVE | Рабочие конфиги в `~/.config` |
| My rice | Зеркало в `~/.local/share/macmgr/rice` |
| DIFF | LIVE ≠ rice |
| Pack | `.tar.gz` перенос сборки |
| Snapshot | Именованный снимок LIVE |
| Undo | Стек отката после restore/import/pull |
| Manifest | Описание пакетов/сервисов/путей |
| Catalog | Каталог плагинов (local + feeds) |
| Feed | Удалённый/builtin TOML-индекс плагинов |
| Safe reload | Validate + undo + restart бара |

---

## Что дальше

Уже закрыто по roadmap P0–P3: тесты/CI, safe reload, rice git helpers, palette, brew search, toast, catalog feeds, multi-line skhd.

Имеет смысл позже (не срочно):

1. **Паритет CLI↔TUI** — редкие команды только в одном месте  
2. **Режимы skhd** (`:: default`, mode switch) в CRUD  
3. **Публичный remote feed** на GitHub (сейчас builtin + свои URL)  
4. **Меньше пунктов в More** — вынести частые вещи на вкладки/palette  
5. **Больше интеграционных тестов** вокруг rice push/pull и pack  

---

## Важно

- SIP и Accessibility менеджер **не меняет** сам — только подсказывает в `doctor`.  
- Опасные действия (uninstall, restore, sync from-repo) спрашивают подтверждение (`--yes` для скриптов).  
- Источник правды — **LIVE** `~/.config/...`. My rice (`~/.local/share/macmgr/rice`) — зеркало для push/pull и GitHub.
