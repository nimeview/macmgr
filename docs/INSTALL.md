# Installing macmgr

## Requirements

- macOS 13+ (Intel or Apple Silicon)
- [Homebrew](https://brew.sh)
- Rust (`brew install rust` or rustup) — only if building from source

## Option A — From source (recommended while preview)

```bash
git clone https://github.com/arturpanan/MacOS_Manager.git
cd MacOS_Manager
cargo install --path .
mkdir -p ~/.config/macmgr
cp manifest.toml ~/.config/macmgr/
macmgr setup --yes
macmgr doctor
```

Or: `./scripts/bootstrap.sh` / `./scripts/install.sh`.

## Option B — GitHub Release binary

After a `v*` tag is published:

1. Download `macmgr-macos-aarch64` or `macmgr-macos-x86_64` from Releases
2. `chmod +x macmgr-macos-* && mv macmgr-macos-* /usr/local/bin/macmgr`
3. Copy `manifest.toml` to `~/.config/macmgr/`
4. `macmgr setup --yes`

## Option C — Homebrew tap (stub)

```bash
# After the tap repo exists and sha256 is filled in Formula/macmgr.rb:
brew tap arturpanan/macmgr
brew install macmgr
```

See [`Formula/macmgr.rb`](../Formula/macmgr.rb).

## Post-install checklist

1. **Accessibility** — System Settings → Privacy & Security → Accessibility → enable **yabai** and **skhd** (and SketchyBar if prompted).
2. **SIP / scripting addition** — basic window management works with SIP on; advanced features (e.g. some window opacity / focus rules) need yabai’s scripting addition, which typically requires partially disabling SIP. Follow the [yabai wiki](https://github.com/koekeishiya/yabai/wiki).
3. **Spaces** — `macmgr spaces` or `~/bin/ensure_spaces.sh` (installed by setup).
4. **Hotkeys** — Left Option + 1–9 for spaces (RU layout: physical keycodes). See skhdrc comments.
5. Run `macmgr doctor` until critical `!!` lines are gone.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `shell pack: not found` | Run setup from a full git clone (needs `shell/`) |
| Spaces hotkeys dead on RU layout | Use Left Option; see starter `skhdrc` keycodes |
| Bar shows app menus instead of spaces | Click front_app / spaces indicator to swap |
| Kitty stays default colors | `macmgr theme apply <id> --reload` writes `~/.config/kitty/current-theme.conf` |
| `yabai` query fails | `macmgr service start yabai` + Accessibility |
| Doctor JSON for scripts | `macmgr doctor --json` |

## Uninstall

```bash
macmgr uninstall --yes --keep-configs   # or omit --keep-configs
brew uninstall yabai skhd sketchybar borders   # optional
```

### Full purge (environment + libraries)

Stops services, `brew uninstall`s every package in the manifest, deletes LIVE
configs, My rice, snapshots/backups, and macmgr Application Support data:

```bash
macmgr purge              # type PURGE to confirm
macmgr purge --yes
macmgr purge --yes --remove-cli          # also cargo uninstall macmgr
macmgr purge --yes --keep-packages       # configs/data only
macmgr purge --yes --keep-live           # keep ~/.config paths
```

Homebrew taps and SIP/Accessibility settings are left untouched.

Rice mirror at `~/.local/share/macmgr/rice` is kept unless you purge (or delete it manually).
