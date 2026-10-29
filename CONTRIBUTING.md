# Contributing

Thanks for helping with macmgr.

## Dev setup

```bash
git clone https://github.com/arturpanan/MacOS_Manager.git
cd MacOS_Manager
cargo test --bins
cargo clippy --bins -- -D warnings
cargo fmt --all
```

## Pull requests

- Keep changes focused; match existing module style under `src/core/` and `src/tui/`
- Add unit tests for parsers / merge logic when practical
- Do not commit absolute home paths, tokens, or private remotes
- Update `CHANGELOG.md` under `[Unreleased]` for user-visible changes

## Catalog / community feeds

- Public HTTPS or `builtin:` sources only
- Document license of any copied Lua
- Prefer small demos; large themes belong in `themes/` generators or separate repos
- Add entries to [`feeds/community.toml`](../feeds/community.toml) or publish your own feed URL (`macmgr catalog feed-add`)

## Starter shell (`shell/`)

- Must stay portable (tilde paths only)
- No personal wallpaper paths or private git remotes
- Rebuild helpers with `make` under `shell/.config/sketchybar/helpers` when C sources change; do not commit huge unrelated binaries if avoidable

## Code of conduct

See [CODE_OF_CONDUCT.md](../CODE_OF_CONDUCT.md).
