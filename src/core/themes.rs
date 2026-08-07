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
                colors_file: dir.join("current_colors.lua").to_string_lossy().into(),
            },
        );
    }
    for (id, label, _) in THEME_DEFS {
        if !have.contains(*id) {
            catalog.themes.push(Theme {
                id: (*id).into(),
                label: (*label).into(),
                colors_file: dir.join(format!("{id}_colors.lua")).to_string_lossy().into(),
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
    let src = expand_tilde(&theme.colors_file);
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
    if reload {
        plugins::reload_bar(manifest)?;
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
pub fn export_theme_pack(manifest: &Manifest, dest: &PathBuf) -> Result<()> {
    let colors = live_colors(manifest)?;
    ensure_dir(dest)?;
    if colors.exists() {
        copy_recursive(&colors, &dest.join("colors.lua"))?;
    }
    Ok(())
}
