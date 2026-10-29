//! User overlay on top of the shipped/base `manifest.toml`.
//!
//! Path: `~/.config/macmgr/user.toml`
//!
//! - `[[packages]]` / `[[configs]]` — entries added from TUI/CLI
//! - `excluded_*` — hide shipped base entries without editing the base file

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::core::brew;
use crate::core::manifest::{ConfigDef, Manifest, PackageDef};
use crate::core::paths::{config_dir, ensure_dir, expand_tilde, home_dir};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserOverlay {
    #[serde(default)]
    pub packages: Vec<PackageDef>,
    #[serde(default)]
    pub configs: Vec<ConfigDef>,
    #[serde(default)]
    pub excluded_packages: Vec<String>,
    #[serde(default)]
    pub excluded_configs: Vec<String>,
}

pub fn user_overlay_path() -> PathBuf {
    config_dir().join("user.toml")
}

pub fn load_overlay() -> Result<UserOverlay> {
    let path = user_overlay_path();
    if !path.exists() {
        return Ok(UserOverlay::default());
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let overlay: UserOverlay =
        toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    Ok(overlay)
}

pub fn save_overlay(overlay: &UserOverlay) -> Result<PathBuf> {
    let dir = config_dir();
    ensure_dir(&dir)?;
    let path = user_overlay_path();
    let raw = toml::to_string_pretty(overlay).context("serialize user.toml")?;
    let tmp = path.with_extension("toml.tmp");
    fs::write(&tmp, raw).with_context(|| format!("write {}", tmp.display()))?;
    fs::rename(&tmp, &path).with_context(|| format!("rename → {}", path.display()))?;
    Ok(path)
}

pub fn merge_into(manifest: &mut Manifest, overlay: &UserOverlay) {
    manifest
        .packages
        .retain(|p| !overlay.excluded_packages.iter().any(|e| e == &p.name));
    manifest
        .configs
        .retain(|c| !overlay.excluded_configs.iter().any(|e| e == &c.id));

    for p in &overlay.packages {
        if let Some(existing) = manifest.packages.iter_mut().find(|x| x.name == p.name) {
            *existing = p.clone();
        } else {
            manifest.packages.push(p.clone());
        }
    }
    for c in &overlay.configs {
        if let Some(existing) = manifest.configs.iter_mut().find(|x| x.id == c.id) {
            *existing = c.clone();
        } else {
            manifest.configs.push(c.clone());
        }
    }
}

/// Apply user overlay onto a freshly loaded base manifest.
pub fn apply_overlay(mut manifest: Manifest) -> Result<Manifest> {
    let overlay = load_overlay()?;
    merge_into(&mut manifest, &overlay);
    Ok(manifest)
}

pub fn track_package(spec: &str) -> Result<PackageDef> {
    let (kind, name) = brew::parse_pkg_spec(spec);
    if name.is_empty() {
        return Err(anyhow!("empty package name"));
    }
    let pkg = PackageDef {
        name: name.clone(),
        kind,
        group: "user".into(),
        core: false,
        optional: false,
        head: false,
    };
    let mut overlay = load_overlay()?;
    overlay.excluded_packages.retain(|e| e != &name);
    if let Some(existing) = overlay.packages.iter_mut().find(|p| p.name == name) {
        *existing = pkg.clone();
    } else {
        overlay.packages.push(pkg.clone());
    }
    save_overlay(&overlay)?;
    Ok(pkg)
}

pub fn untrack_package(name: &str, force: bool) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(anyhow!("empty package name"));
    }
    let mut overlay = load_overlay()?;
    let in_user = overlay.packages.iter().any(|p| p.name == name);
    if in_user {
        overlay.packages.retain(|p| p.name != name);
        save_overlay(&overlay)?;
        return Ok(());
    }
    if !force {
        return Err(anyhow!(
            "`{name}` is not in user.toml (shipped/base). Use --force to hide it from lists"
        ));
    }
    if !overlay.excluded_packages.iter().any(|e| e == name) {
        overlay.excluded_packages.push(name.to_string());
    }
    save_overlay(&overlay)?;
    Ok(())
}

pub fn track_config(
    manifest: &Manifest,
    id: &str,
    path: &str,
    kind: Option<&str>,
) -> Result<ConfigDef> {
    let id = id.trim();
    let path = path.trim();
    if id.is_empty() || path.is_empty() {
        return Err(anyhow!("need id and path (example: nvim ~/.config/nvim)"));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(anyhow!("id `{id}` must be alphanumeric / '-' / '_'"));
    }

    let live = normalize_tilde_path(path);
    let abs = expand_tilde(&live);
    let kind = kind
        .map(|s| s.to_string())
        .unwrap_or_else(|| infer_kind(&abs));
    let repo_path = default_repo_path(manifest, &live);

    let cfg = ConfigDef {
        id: id.to_string(),
        label: id.to_string(),
        path: live,
        repo_path,
        kind,
        managed: true,
    };

    let mut overlay = load_overlay()?;
    overlay.excluded_configs.retain(|e| e != id);
    if let Some(existing) = overlay.configs.iter_mut().find(|c| c.id == id) {
        *existing = cfg.clone();
    } else if manifest.find_config(id).is_some() && !overlay.configs.iter().any(|c| c.id == id) {
        // Overriding a shipped config via user overlay
        overlay.configs.push(cfg.clone());
    } else {
        overlay.configs.push(cfg.clone());
    }
    save_overlay(&overlay)?;
    Ok(cfg)
}

pub fn untrack_config(id: &str, force: bool) -> Result<()> {
    let id = id.trim();
    if id.is_empty() {
        return Err(anyhow!("empty config id"));
    }
    let mut overlay = load_overlay()?;
    let in_user = overlay.configs.iter().any(|c| c.id == id);
    if in_user {
        overlay.configs.retain(|c| c.id != id);
        save_overlay(&overlay)?;
        return Ok(());
    }
    if !force {
        return Err(anyhow!(
            "`{id}` is not in user.toml (shipped/base). Use --force to hide it from lists"
        ));
    }
    if !overlay.excluded_configs.iter().any(|e| e == id) {
        overlay.excluded_configs.push(id.to_string());
    }
    save_overlay(&overlay)?;
    Ok(())
}

/// Parse `id path` or a bare path (id = last path component).
pub fn parse_config_add_spec(spec: &str) -> Result<(String, String)> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err(anyhow!("empty config spec"));
    }
    let parts: Vec<&str> = spec.split_whitespace().collect();
    if parts.len() >= 2 {
        let id = parts[0].to_string();
        let path = parts[1..].join(" ");
        return Ok((id, path));
    }
    let path = parts[0].to_string();
    let abs = expand_tilde(&path);
    let id = abs
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.trim_start_matches('.').replace('.', "_"))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("cannot infer id from path `{path}`"))?;
    Ok((id, path))
}

fn normalize_tilde_path(path: &str) -> String {
    let path = path.trim();
    if path.starts_with("~/") || path == "~" {
        return path.to_string();
    }
    let abs = PathBuf::from(path);
    let home = home_dir();
    if let Ok(rel) = abs.strip_prefix(&home) {
        return format!("~/{}", rel.display());
    }
    path.to_string()
}

fn default_repo_path(manifest: &Manifest, live_tilde: &str) -> String {
    let rice_cfg = manifest.rice_config();
    let rice = rice_cfg.path.trim_end_matches('/');
    let rel = live_tilde.strip_prefix("~/").unwrap_or(live_tilde);
    format!("{rice}/{rel}")
}

fn infer_kind(abs: &Path) -> String {
    if abs.is_dir() {
        "dir".into()
    } else if abs.is_file() || abs.extension().is_some() {
        "file".into()
    } else {
        "dir".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_spec_two_parts() {
        let (id, path) = parse_config_add_spec("nvim ~/.config/nvim").unwrap();
        assert_eq!(id, "nvim");
        assert_eq!(path, "~/.config/nvim");
    }

    #[test]
    fn parse_config_spec_bare_path() {
        let (id, path) = parse_config_add_spec("~/.config/helix").unwrap();
        assert_eq!(id, "helix");
        assert_eq!(path, "~/.config/helix");
    }

    #[test]
    fn merge_adds_and_excludes() {
        let mut m = Manifest {
            name: "t".into(),
            version: String::new(),
            manifest_version: 1,
            description: String::new(),
            rice: None,
            dotfiles: None,
            paths: crate::core::manifest::PathsConfig {
                config_home: "~/.config".into(),
                wallpaper_dir: "~/Wallpaper".into(),
                backup_dir: "~/.local/share/macmgr/backups".into(),
                ensure_spaces: "~/bin/ensure_spaces.sh".into(),
            },
            taps: vec![],
            services: vec![],
            packages: vec![PackageDef {
                name: "git".into(),
                kind: "formula".into(),
                group: "core".into(),
                core: true,
                optional: false,
                head: false,
            }],
            configs: vec![ConfigDef {
                id: "tmux".into(),
                label: "tmux".into(),
                path: "~/.tmux.conf".into(),
                repo_path: "~/.local/share/macmgr/rice/.tmux.conf".into(),
                kind: "file".into(),
                managed: true,
            }],
            defaults: vec![],
            presets: vec![],
            post_install: vec![],
        };
        let overlay = UserOverlay {
            packages: vec![PackageDef {
                name: "ripgrep".into(),
                kind: "formula".into(),
                group: "user".into(),
                core: false,
                optional: false,
                head: false,
            }],
            configs: vec![],
            excluded_packages: vec![],
            excluded_configs: vec!["tmux".into()],
        };
        merge_into(&mut m, &overlay);
        assert!(m.packages.iter().any(|p| p.name == "ripgrep"));
        assert!(m.packages.iter().any(|p| p.name == "git"));
        assert!(m.find_config("tmux").is_none());
    }
}
