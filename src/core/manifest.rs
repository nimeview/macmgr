use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::core::paths::{config_dir, expand_tilde};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    /// My rice mirror (LIVE ↔ selective sync). Preferred over legacy `dotfiles`.
    #[serde(default)]
    pub rice: Option<RiceConfig>,
    /// Legacy Felix-style section; used only if `rice` is absent.
    #[serde(default)]
    pub dotfiles: Option<Dotfiles>,
    pub paths: PathsConfig,
    #[serde(default)]
    pub taps: Vec<Tap>,
    #[serde(default)]
    pub services: Vec<ServiceDef>,
    #[serde(default)]
    pub packages: Vec<PackageDef>,
    #[serde(default)]
    pub configs: Vec<ConfigDef>,
    #[serde(default)]
    pub defaults: Vec<DefaultDef>,
    #[serde(default)]
    pub presets: Vec<PresetDef>,
    #[serde(default)]
    pub post_install: Vec<PostInstall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiceConfig {
    #[serde(default = "default_rice_path")]
    pub path: String,
    #[serde(default)]
    pub git_url: String,
    /// `copy` | `symlink` when planting LIVE from rice on install
    #[serde(default = "default_rice_strategy")]
    pub strategy: String,
}

fn default_rice_path() -> String {
    "~/.local/share/macmgr/rice".into()
}

fn default_rice_strategy() -> String {
    "copy".into()
}

impl Default for RiceConfig {
    fn default() -> Self {
        Self {
            path: default_rice_path(),
            git_url: String::new(),
            strategy: default_rice_strategy(),
        }
    }
}

/// Legacy `[dotfiles]` (Felix-era). Prefer `[rice]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dotfiles {
    pub path: String,
    #[serde(default)]
    pub git_url: String,
    #[serde(default = "default_legacy_strategy")]
    pub strategy: String,
}

fn default_legacy_strategy() -> String {
    "symlink".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub config_home: String,
    pub wallpaper_dir: String,
    pub backup_dir: String,
    #[serde(default)]
    pub ensure_spaces: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tap {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDef {
    pub id: String,
    pub brew: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub core: bool,
    /// `brew` (brew services) or `native` (yabai/skhd --*-service)
    #[serde(default = "default_control")]
    pub control: String,
}

fn default_control() -> String {
    "brew".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDef {
    pub name: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub core: bool,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub head: bool,
}

fn default_kind() -> String {
    "formula".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigDef {
    pub id: String,
    pub label: String,
    pub path: String,
    #[serde(default)]
    pub repo_path: String,
    #[serde(default = "default_config_kind")]
    pub kind: String,
    /// Participate in rice sync (default true when `repo_path` set).
    #[serde(default = "default_managed")]
    pub managed: bool,
}

fn default_config_kind() -> String {
    "file".into()
}

fn default_managed() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultDef {
    pub id: String,
    pub domain: String,
    pub key: String,
    pub value: String,
    #[serde(default = "default_type")]
    pub r#type: String,
    #[serde(default)]
    pub core: bool,
}

fn default_type() -> String {
    "bool".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetDef {
    pub id: String,
    pub label: String,
    pub padding: i32,
    pub gap: i32,
    pub opacity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostInstall {
    pub id: String,
    pub label: String,
    pub command: String,
}

impl Manifest {
    pub fn load() -> Result<Self> {
        let candidates = [
            config_dir().join("manifest.toml"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("manifest.toml"),
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("manifest.toml")))
                .unwrap_or_default(),
            PathBuf::from("manifest.toml"),
        ];

        for path in candidates {
            if path.exists() {
                return Self::from_file(&path);
            }
        }

        anyhow::bail!(
            "manifest.toml not found. Place it next to the binary, in the repo, or in {}",
            config_dir().display()
        )
    }

    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("reading manifest {}", path.display()))?;
        let manifest: Manifest = toml::from_str(&raw)
            .with_context(|| format!("parsing manifest {}", path.display()))?;
        Ok(manifest)
    }

    /// Resolved rice config (from `[rice]` or legacy `[dotfiles]` mapped in).
    pub fn rice_config(&self) -> RiceConfig {
        if let Some(r) = &self.rice {
            return r.clone();
        }
        if let Some(d) = &self.dotfiles {
            return RiceConfig {
                path: d.path.clone(),
                git_url: String::new(), // never auto-clone Felix
                strategy: if d.strategy == "symlink" {
                    "symlink".into()
                } else {
                    "copy".into()
                },
            };
        }
        RiceConfig::default()
    }

    pub fn rice_path(&self) -> PathBuf {
        expand_tilde(&self.rice_config().path)
    }

    /// Deprecated alias — returns rice path.
    #[allow(dead_code)]
    pub fn dotfiles_path(&self) -> PathBuf {
        self.rice_path()
    }

    pub fn wallpaper_dir(&self) -> PathBuf {
        expand_tilde(&self.paths.wallpaper_dir)
    }

    pub fn backup_dir(&self) -> PathBuf {
        expand_tilde(&self.paths.backup_dir)
    }

    pub fn managed_configs(&self) -> impl Iterator<Item = &ConfigDef> {
        self.configs
            .iter()
            .filter(|c| c.managed && !c.repo_path.is_empty())
    }

    pub fn core_services(&self) -> impl Iterator<Item = &ServiceDef> {
        self.services.iter().filter(|s| s.core)
    }

    pub fn core_packages(&self) -> impl Iterator<Item = &PackageDef> {
        self.packages.iter().filter(|p| p.core && !p.optional)
    }

    pub fn install_packages(&self, full: bool) -> Vec<&PackageDef> {
        self.packages
            .iter()
            .filter(|p| {
                if p.optional {
                    return false;
                }
                if full {
                    true
                } else {
                    p.core || p.group == "core"
                }
            })
            .collect()
    }

    pub fn find_config(&self, id: &str) -> Option<&ConfigDef> {
        self.configs
            .iter()
            .find(|c| c.id == id || c.label.eq_ignore_ascii_case(id))
    }

    pub fn find_service(&self, id: &str) -> Option<&ServiceDef> {
        self.services
            .iter()
            .find(|s| s.id == id || s.brew == id)
    }

    pub fn find_package(&self, name: &str) -> Option<&PackageDef> {
        self.packages.iter().find(|p| p.name == name)
    }

    pub fn find_preset(&self, id: &str) -> Option<&PresetDef> {
        self.presets
            .iter()
            .find(|p| p.id == id || p.label.eq_ignore_ascii_case(id))
    }
}
