use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::core::manifest::Manifest;
use crate::core::paths::{config_dir, ensure_dir};
use crate::core::plugins;
use crate::core::presets;
use crate::core::wallpaper;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub plugins_on: Vec<String>,
    #[serde(default)]
    pub plugins_off: Vec<String>,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub wallpaper: Option<String>,
    #[serde(default)]
    pub theme: Option<String>,
}

fn profiles_dir() -> PathBuf {
    config_dir().join("profiles")
}

pub fn ensure_defaults() -> Result<()> {
    let dir = profiles_dir();
    ensure_dir(&dir)?;
    let work = dir.join("work.toml");
    let game = dir.join("game.toml");
    if !work.exists() {
        let p = Profile {
            id: "work".into(),
            label: "Work".into(),
            plugins_on: vec![
                "spaces".into(),
                "front_app".into(),
                "calendar".into(),
                "widgets.battery".into(),
                "widgets.cpu".into(),
            ],
            plugins_off: vec!["media".into()],
            preset: Some("compact".into()),
            wallpaper: None,
            theme: Some("dim".into()),
        };
        fs::write(&work, toml::to_string_pretty(&p)?)?;
    }
    if !game.exists() {
        let p = Profile {
            id: "game".into(),
            label: "Game".into(),
            plugins_on: vec!["media".into(), "widgets.volume".into()],
            plugins_off: vec!["widgets.cpu".into(), "menus".into()],
            preset: Some("spacious".into()),
            wallpaper: None,
            theme: Some("bright".into()),
        };
        fs::write(&game, toml::to_string_pretty(&p)?)?;
    }
    Ok(())
}

pub fn list() -> Result<Vec<Profile>> {
    ensure_defaults()?;
    let mut out = Vec::new();
    for entry in fs::read_dir(profiles_dir())? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let raw = fs::read_to_string(&path)?;
        let mut p: Profile = toml::from_str(&raw)
            .with_context(|| format!("parse {}", path.display()))?;
        if p.id.is_empty() {
            p.id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("profile")
                .to_string();
        }
        out.push(p);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

pub fn apply(manifest: &Manifest, id: &str) -> Result<()> {
    let profile = list()?
        .into_iter()
        .find(|p| p.id == id || p.label.eq_ignore_ascii_case(id))
        .ok_or_else(|| anyhow!("unknown profile `{id}`"))?;

    for pid in &profile.plugins_off {
        let _ = plugins::disable(manifest, pid);
    }
    for pid in &profile.plugins_on {
        let _ = plugins::enable(manifest, pid);
    }
    if let Some(preset) = &profile.preset {
        let _ = presets::apply(manifest, preset);
    }
    if let Some(wp) = &profile.wallpaper {
        let _ = wallpaper::set_by_name(manifest, wp);
    }
    if let Some(theme) = &profile.theme {
        let _ = crate::core::themes::apply(manifest, theme, false);
    }
    plugins::reload_bar(manifest)?;
    Ok(())
}

pub fn save_from_current(manifest: &Manifest, id: &str, label: &str) -> Result<PathBuf> {
    ensure_defaults()?;
    let plugs = plugins::list(manifest)?;
    let mut on = Vec::new();
    let mut off = Vec::new();
    for p in plugs {
        if p.enabled {
            on.push(p.id);
        } else {
            off.push(p.id);
        }
    }
    let profile = Profile {
        id: id.to_string(),
        label: label.to_string(),
        plugins_on: on,
        plugins_off: off,
        preset: None,
        wallpaper: None,
        theme: None,
    };
    let path = profiles_dir().join(format!("{id}.toml"));
    fs::write(&path, toml::to_string_pretty(&profile)?)?;
    Ok(path)
}
