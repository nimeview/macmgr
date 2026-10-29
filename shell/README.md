# Shell starter snapshot (v0.2)

Portable LIVE rice configs for first-time install via `macmgr setup` /
`macmgr install --scope core` (seeded into `~/.local/share/macmgr/rice` then
planted to `~/.config`).

Included:

- SketchyBar (spaces, media, wifi, widgets)
- yabai / skhd / borders
- kitty (`kitty.conf` + theme include)
- `bin/ensure_spaces.sh` → up to 9 yabai spaces

No absolute user paths or private remotes. Rebuild native helpers:

```bash
cd shell/.config/sketchybar/helpers && make
```

Daily source of truth after install remains `~/.config/…`.
