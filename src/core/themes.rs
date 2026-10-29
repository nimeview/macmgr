use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::core::manifest::Manifest;
use crate::core::paths::{ensure_dir, expand_tilde};
use crate::core::plugins;
use crate::util::shell::copy_recursive;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub id: String,
    pub label: String,
    /// Absolute or ~/ path to a colors.lua to copy into LIVE sketchybar
    pub colors_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeCatalog {
    #[serde(default)]
    pub themes: Vec<Theme>,
}

fn themes_dir() -> PathBuf {
    crate::core::paths::config_dir().join("themes")
}

fn catalog_path() -> PathBuf {
    themes_dir().join("catalog.toml")
}

/// Resolve a theme colors path: absolute, `~/…`, or filename relative to themes_dir().
fn resolve_colors_file(colors_file: &str) -> PathBuf {
    let p = colors_file.trim();
    if p.starts_with('/') || p.starts_with("~/") {
        return expand_tilde(p);
    }
    let as_path = PathBuf::from(p);
    if as_path.is_absolute() {
        return as_path;
    }
    themes_dir().join(p)
}

const THEME_DEFS: &[(&str, &str, &str)] = &[
    ("dim", "Catppuccin Macchiato (dim)", DIM),
    ("bright", "Catppuccin Mocha (bright)", BRIGHT),
    ("latte", "Catppuccin Latte (light)", LATTE),
    ("tokyo", "Tokyo Night", TOKYO),
    ("gruvbox", "Gruvbox Dark", GRUVBOX),
    ("nord", "Nord", NORD),
    ("rosepine", "Rosé Pine", ROSEPINE),
    ("dracula", "Dracula", DRACULA),
    ("solarized", "Solarized Dark", SOLARIZED),
    ("monokai", "Monokai", MONOKAI),
];

pub fn ensure_defaults() -> Result<()> {
    let dir = themes_dir();
    ensure_dir(&dir)?;
    write_all_theme_files(&dir)?;

    let cat = catalog_path();
    let mut catalog = if cat.exists() {
        toml::from_str(&fs::read_to_string(&cat)?)?
    } else {
        ThemeCatalog::default()
    };

    // Always ensure current backup entry exists
    let mut have = std::collections::HashSet::new();
    for t in &catalog.themes {
        have.insert(t.id.clone());
    }
    if !have.contains("current") {
        catalog.themes.insert(
            0,
            Theme {
                id: "current".into(),
                label: "Current (backup snapshot)".into(),
                colors_file: "current_colors.lua".into(),
            },
        );
    }
    for (id, label, _) in THEME_DEFS {
        if !have.contains(*id) {
            catalog.themes.push(Theme {
                id: (*id).into(),
                label: (*label).into(),
                colors_file: format!("{id}_colors.lua"),
            });
        }
    }
    fs::write(&cat, toml::to_string_pretty(&catalog)?)?;
    Ok(())
}

fn write_all_theme_files(dir: &Path) -> Result<()> {
    for (id, _, body) in THEME_DEFS {
        fs::write(dir.join(format!("{id}_colors.lua")), body)?;
    }
    Ok(())
}

const DIM: &str = r#"return {
  black = 0xff181926,
  white = 0xffcdd6f4,
  red = 0xffed8796,
  green = 0xffa6da95,
  blue = 0xff8aadf4,
  yellow = 0xffeed49f,
  orange = 0xfff5a97f,
  magenta = 0xffc6a0f6,
  grey = 0xff939ab7,
  transparent = 0x00000000,
  bar = { bg = 0xf0161826, border = 0xff494d64 },
  popup = { bg = 0xc0161826, border = 0xff7f849c },
  bg1 = 0xff1e2030,
  bg2 = 0xff363a4f,
  with_alpha = function(color, alpha)
    if alpha > 1.0 or alpha < 0.0 then return color end
    return (color & 0x00ffffff) | (math.floor(alpha * 255.0) << 24)
  end,
}
"#;

const BRIGHT: &str = r#"return {
  black = 0xff1e1e2e,
  white = 0xfff5e0dc,
  red = 0xfff38ba8,
  green = 0xffa6e3a1,
  blue = 0xff89b4fa,
  yellow = 0xfff9e2af,
  orange = 0xfffab387,
  magenta = 0xffcba6f7,
  grey = 0xffbac2de,
  transparent = 0x00000000,
  bar = { bg = 0xe01e1e2e, border = 0xffcba6f7 },
  popup = { bg = 0xe01e1e2e, border = 0xff89b4fa },
  bg1 = 0xff313244,
  bg2 = 0xff45475a,
  with_alpha = function(color, alpha)
    if alpha > 1.0 or alpha < 0.0 then return color end
    return (color & 0x00ffffff) | (math.floor(alpha * 255.0) << 24)
  end,
}
"#;

const LATTE: &str = r#"return {
  black = 0xffeff1f5,
  white = 0xff4c4f69,
  red = 0xffd20f39,
  green = 0xff40a02b,
  blue = 0xff1e66f5,
  yellow = 0xffdf8e1d,
  orange = 0xfffe640b,
  magenta = 0xff8839ef,
  grey = 0xff6c6f85,
  transparent = 0x00000000,
  bar = { bg = 0xe0eff1f5, border = 0xff8839ef },
  popup = { bg = 0xe0eff1f5, border = 0xff1e66f5 },
  bg1 = 0xffe6e9ef,
  bg2 = 0xffccd0da,
  with_alpha = function(color, alpha)
    if alpha > 1.0 or alpha < 0.0 then return color end
    return (color & 0x00ffffff) | (math.floor(alpha * 255.0) << 24)
  end,
}
"#;

const TOKYO: &str = r#"return {
  black = 0xff1a1b26,
  white = 0xffc0caf5,
  red = 0xfff7768e,
  green = 0xff9ece6a,
  blue = 0xff7aa2f7,
  yellow = 0xffe0af68,
  orange = 0xffff9e64,
  magenta = 0xffbb9af7,
  grey = 0xff565f89,
  transparent = 0x00000000,
  bar = { bg = 0xe01a1b26, border = 0xff7aa2f7 },
  popup = { bg = 0xe01a1b26, border = 0xffbb9af7 },
  bg1 = 0xff24283b,
  bg2 = 0xff414868,
  with_alpha = function(color, alpha)
    if alpha > 1.0 or alpha < 0.0 then return color end
    return (color & 0x00ffffff) | (math.floor(alpha * 255.0) << 24)
  end,
}
"#;

const GRUVBOX: &str = r#"return {
  black = 0xff282828,
  white = 0xffebdbb2,
  red = 0xfffb4934,
  green = 0xffb8bb26,
  blue = 0xff83a598,
  yellow = 0xfffabd2f,
  orange = 0xfffe8019,
  magenta = 0xffd3869b,
  grey = 0xff928374,
  transparent = 0x00000000,
  bar = { bg = 0xe0282828, border = 0xfffabd2f },
  popup = { bg = 0xe0282828, border = 0xff83a598 },
  bg1 = 0xff3c3836,
  bg2 = 0xff504945,
  with_alpha = function(color, alpha)
    if alpha > 1.0 or alpha < 0.0 then return color end
    return (color & 0x00ffffff) | (math.floor(alpha * 255.0) << 24)
  end,
}
"#;

const NORD: &str = r#"return {
  black = 0xff2e3440,
  white = 0xffeceff4,
  red = 0xffbf616a,
  green = 0xffa3be8c,
  blue = 0xff81a1c1,
  yellow = 0xffebcb8b,
  orange = 0xffd08770,
  magenta = 0xffb48ead,
  grey = 0xff4c566a,
  transparent = 0x00000000,
  bar = { bg = 0xe02e3440, border = 0xff88c0d0 },
  popup = { bg = 0xe02e3440, border = 0xff81a1c1 },
  bg1 = 0xff3b4252,
  bg2 = 0xff434c5e,
  with_alpha = function(color, alpha)
    if alpha > 1.0 or alpha < 0.0 then return color end
    return (color & 0x00ffffff) | (math.floor(alpha * 255.0) << 24)
  end,
}
"#;

const ROSEPINE: &str = r#"return {
  black = 0xff191724,
  white = 0xffe0def4,
  red = 0xffeb6f92,
  green = 0xff9ccfd8,
  blue = 0xff31748f,
  yellow = 0xfff6c177,
  orange = 0xffebbcba,
  magenta = 0xffc4a7e7,
  grey = 0xff6e6a86,
  transparent = 0x00000000,
  bar = { bg = 0xe0191724, border = 0xffc4a7e7 },
  popup = { bg = 0xe0191724, border = 0xff9ccfd8 },
  bg1 = 0xff1f1d2e,
  bg2 = 0xff26233a,
  with_alpha = function(color, alpha)
    if alpha > 1.0 or alpha < 0.0 then return color end
    return (color & 0x00ffffff) | (math.floor(alpha * 255.0) << 24)
  end,
}
"#;

const DRACULA: &str = r#"return {
  black = 0xff282a36,
  white = 0xfff8f8f2,
  red = 0xffff5555,
  green = 0xff50fa7b,
  blue = 0xff8be9fd,
  yellow = 0xfff1fa8c,
  orange = 0xffffb86c,
  magenta = 0xffff79c6,
  grey = 0xff6272a4,
  transparent = 0x00000000,
  bar = { bg = 0xe0282a36, border = 0xffbd93f9 },
  popup = { bg = 0xe0282a36, border = 0xffff79c6 },
  bg1 = 0xff44475a,
  bg2 = 0xff6272a4,
  with_alpha = function(color, alpha)
    if alpha > 1.0 or alpha < 0.0 then return color end
    return (color & 0x00ffffff) | (math.floor(alpha * 255.0) << 24)
  end,
}
"#;

const SOLARIZED: &str = r#"return {
  black = 0xff002b36,
  white = 0xffeee8d5,
  red = 0xffdc322f,
  green = 0xff859900,
  blue = 0xff268bd2,
  yellow = 0xffb58900,
  orange = 0xffcb4b16,
  magenta = 0xffd33682,
  grey = 0xff586e75,
  transparent = 0x00000000,
  bar = { bg = 0xe0002b36, border = 0xff268bd2 },
  popup = { bg = 0xe0002b36, border = 0xff859900 },
  bg1 = 0xff073642,
  bg2 = 0xff586e75,
  with_alpha = function(color, alpha)
    if alpha > 1.0 or alpha < 0.0 then return color end
    return (color & 0x00ffffff) | (math.floor(alpha * 255.0) << 24)
  end,
}
"#;

const MONOKAI: &str = r#"return {
  black = 0xff272822,
  white = 0xfff8f8f2,
  red = 0xfff92672,
  green = 0xffa6e22e,
  blue = 0xff66d9ef,
  yellow = 0xffe6db74,
  orange = 0xfffd971f,
  magenta = 0xffae81ff,
  grey = 0xff75715e,
  transparent = 0x00000000,
  bar = { bg = 0xe0272822, border = 0xfff92672 },
  popup = { bg = 0xe0272822, border = 0xff66d9ef },
  bg1 = 0xff3e3d32,
  bg2 = 0xff49483e,
  with_alpha = function(color, alpha)
    if alpha > 1.0 or alpha < 0.0 then return color end
    return (color & 0x00ffffff) | (math.floor(alpha * 255.0) << 24)
  end,
}
"#;

pub fn list() -> Result<Vec<Theme>> {
    ensure_defaults()?;
    let raw = fs::read_to_string(catalog_path())?;
    let cat: ThemeCatalog = toml::from_str(&raw)?;
    Ok(cat.themes)
}

fn live_colors(manifest: &Manifest) -> Result<PathBuf> {
    let root = expand_tilde(
        &manifest
            .find_config("sketchybar")
            .ok_or_else(|| anyhow!("no sketchybar config"))?
            .path,
    );
    Ok(root.join("colors.lua"))
}

pub fn apply(manifest: &Manifest, id: &str, reload: bool) -> Result<()> {
    let themes = list()?;
    let theme = themes
        .iter()
        .find(|t| t.id == id)
        .ok_or_else(|| anyhow!("unknown theme `{id}`"))?;
    let src = resolve_colors_file(&theme.colors_file);
    if !src.exists() {
        return Err(anyhow!("theme file missing: {}", src.display()));
    }
    let dst = live_colors(manifest)?;
    if dst.exists() {
        let bak = themes_dir().join("current_colors.lua");
        ensure_dir(&themes_dir())?;
        fs::copy(&dst, &bak)?;
    }
    fs::copy(&src, &dst).with_context(|| format!("copy theme to {}", dst.display()))?;

    // Keep kitty in sync (opaque colored selection, not glass/default)
    let lua = fs::read_to_string(&src)?;
    if let Some(palette) = parse_palette(&lua) {
        let _ = apply_kitty_theme(manifest, &palette);
        let _ = apply_borders_theme(manifest, &palette);
    }

    if reload {
        plugins::reload_bar(manifest)?;
        let _ = std::process::Command::new("borders").arg("reload").status();
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct Palette {
    black: u32,
    white: u32,
    red: u32,
    green: u32,
    blue: u32,
    yellow: u32,
    orange: u32,
    magenta: u32,
    grey: u32,
    bg1: u32,
    bg2: u32,
}

fn parse_palette(lua: &str) -> Option<Palette> {
    let get = |key: &str| -> Option<u32> {
        for line in lua.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix(key) {
                let rest = rest.trim_start();
                if let Some(rest) = rest.strip_prefix('=') {
                    let rest = rest.trim_start().trim_end_matches(',');
                    if let Some(hex) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
                        return u32::from_str_radix(hex, 16).ok();
                    }
                }
            }
        }
        None
    };
    Some(Palette {
        black: get("black")?,
        white: get("white")?,
        red: get("red")?,
        green: get("green")?,
        blue: get("blue")?,
        yellow: get("yellow")?,
        orange: get("orange")?,
        magenta: get("magenta")?,
        grey: get("grey")?,
        bg1: get("bg1")?,
        bg2: get("bg2")?,
    })
}

fn rgb_hex(argb: u32) -> String {
    format!("#{:06x}", argb & 0x00ff_ffff)
}

fn luminance(argb: u32) -> f32 {
    let r = ((argb >> 16) & 0xff) as f32;
    let g = ((argb >> 8) & 0xff) as f32;
    let b = (argb & 0xff) as f32;
    0.299 * r + 0.587 * g + 0.114 * b
}

fn kitty_theme_conf(p: &Palette) -> String {
    let bg = rgb_hex(p.black);
    let fg = rgb_hex(p.white);
    let cursor = rgb_hex(p.white);
    // Opaque macOS-like selection: solid accent fill + contrasting text
    let sel_bg = rgb_hex(p.blue);
    let sel_fg = if luminance(p.blue) > 160.0 {
        rgb_hex(p.black)
    } else {
        rgb_hex(p.white)
    };
    let tab_bg = rgb_hex(p.bg1);
    let inactive = rgb_hex(p.grey);
    let color0 = rgb_hex(p.black);
    let color8 = rgb_hex(p.bg2);
    let color1 = rgb_hex(p.red);
    let color2 = rgb_hex(p.green);
    let color3 = rgb_hex(p.yellow);
    let color4 = rgb_hex(p.blue);
    let color5 = rgb_hex(p.magenta);
    let color6 = rgb_hex(p.orange);
    let color7 = rgb_hex(p.white);

    format!(
        r#"# Generated by macmgr theme apply — do not edit by hand
# Opaque selection (no translucent/glass look)

background {bg}
foreground {fg}

selection_background {sel_bg}
selection_foreground {sel_fg}

cursor {cursor}
cursor_text_color background

url_color {color4}

active_tab_background {tab_bg}
active_tab_foreground {fg}
active_tab_font_style bold
inactive_tab_background {tab_bg}
inactive_tab_foreground {inactive}
inactive_tab_font_style normal
tab_bar_background {bg}

# Black
color0 {color0}
color8 {color8}

# Red
color1 {color1}
color9 {color1}

# Green
color2 {color2}
color10 {color2}

# Yellow
color3 {color3}
color11 {color3}

# Blue
color4 {color4}
color12 {color4}

# Magenta
color5 {color5}
color13 {color5}

# Cyan / orange accent
color6 {color6}
color14 {color6}

# White
color7 {color7}
color15 {color7}
"#
    )
}

fn live_kitty_dir(manifest: &Manifest) -> PathBuf {
    manifest
        .find_config("kitty")
        .map(|c| expand_tilde(&c.path))
        .unwrap_or_else(|| expand_tilde("~/.config/kitty"))
}

fn apply_kitty_theme(manifest: &Manifest, palette: &Palette) -> Result<()> {
    let dir = live_kitty_dir(manifest);
    ensure_dir(&dir)?;
    let theme_path = dir.join("current-theme.conf");
    let body = kitty_theme_conf(palette);
    fs::write(&theme_path, body)
        .with_context(|| format!("write kitty theme {}", theme_path.display()))?;

    // Ensure main conf includes the theme file (and opacity settings)
    let conf = dir.join("kitty.conf");
    if conf.exists() {
        let mut text = fs::read_to_string(&conf)?;
        let mut changed = false;
        if !text.contains("include current-theme.conf") {
            text.push_str("\ninclude current-theme.conf\n");
            changed = true;
        }
        if !text.contains("background_opacity") {
            text.push_str("background_opacity 1.0\n");
            changed = true;
        }
        if changed {
            fs::write(&conf, text)?;
        }
    } else {
        fs::write(
            &conf,
            "background_opacity 1.0\ninclude current-theme.conf\n",
        )?;
    }

    // Ask running kitty to reload (best-effort)
    let _ = std::process::Command::new("kitty")
        .args(["@", "load-config"])
        .status();
    Ok(())
}

fn apply_borders_theme(manifest: &Manifest, palette: &Palette) -> Result<()> {
    let path = manifest
        .find_config("borders")
        .map(|c| expand_tilde(&c.path))
        .unwrap_or_else(|| expand_tilde("~/.config/borders/bordersrc"));
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    // SketchyBar ARGB → borders 0xAARRGGBB with soft alpha on inactive
    let active = palette.white | 0xc000_0000;
    let inactive = (palette.bg1 & 0x00ff_ffff) | 0xc000_0000;
    let background = (palette.black & 0x00ff_ffff) | 0x3000_0000;
    let body = format!(
        r#"#!/bin/bash
# Generated by macmgr theme apply
options=(
	style=round
	width=6.0
	hidpi=off
	active_color=0x{active:08x}
	inactive_color=0x{inactive:08x}
	background_color=0x{background:08x}
)
borders "${{options[@]}}"
"#
    );
    fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms)?;
    }
    Ok(())
}

pub fn backup_current(manifest: &Manifest) -> Result<PathBuf> {
    ensure_defaults()?;
    let src = live_colors(manifest)?;
    let dst = themes_dir().join("current_colors.lua");
    if src.exists() {
        fs::copy(&src, &dst)?;
    }
    Ok(dst)
}

#[allow(dead_code)]
pub fn export_theme_pack(manifest: &Manifest, dest: &Path) -> Result<()> {
    let colors = live_colors(manifest)?;
    ensure_dir(dest)?;
    if colors.exists() {
        copy_recursive(&colors, &dest.join("colors.lua"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dim_palette() {
        let p = parse_palette(DIM).expect("palette");
        assert_eq!(p.black & 0xffffff, 0x181926);
        assert_eq!(p.blue & 0xffffff, 0x8aadf4);
    }

    #[test]
    fn kitty_conf_has_opaque_selection() {
        let p = parse_palette(DIM).unwrap();
        let conf = kitty_theme_conf(&p);
        assert!(conf.contains("selection_background #8aadf4"));
        assert!(
            conf.contains("selection_foreground #cdd6f4")
                || conf.contains("selection_foreground #181926")
        );
        assert!(conf.contains("background #181926"));
    }
}
