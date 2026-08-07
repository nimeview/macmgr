use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

use crate::core::manifest::{ConfigDef, Manifest};
use crate::core::paths::{ensure_dir, expand_tilde};
use crate::util::editor::{edit_path, edit_path_simple, reveal_path};
use crate::util::shell::copy_recursive;
use chrono::Local;

#[derive(Debug, Clone)]
pub struct ConfigInfo {
    pub id: String,
    pub path: PathBuf,
    pub exists: bool,
    pub is_dir: bool,
    pub in_sync: Option<bool>,
}

/// Always resolve the LIVE config path (~/.config/...), never the repo copy.
pub fn live_path(cfg: &ConfigDef) -> PathBuf {
    expand_tilde(&cfg.path)
}

pub fn list_configs(manifest: &Manifest) -> Vec<ConfigInfo> {
    list_configs_with_sync(manifest, false)
}

/// Fast listing: existence only (no walkdir content compare).
pub fn list_configs_fast(manifest: &Manifest) -> Vec<ConfigInfo> {
    list_configs_with_sync(manifest, false)
}

/// Full listing with live↔repo sync heuristic (can be slow for sketchybar).
pub fn list_configs_with_sync(manifest: &Manifest, check_sync: bool) -> Vec<ConfigInfo> {
    manifest
        .configs
        .iter()
        .map(|c| {
            let path = live_path(c);
            let managed = c.managed && !c.repo_path.is_empty();
            let repo_path = if managed {
                Some(expand_tilde(&c.repo_path))
            } else {
                None
            };
            let exists = path.exists();
            let is_dir = path.is_dir();
            let in_sync = if !managed {
                None
            } else if check_sync {
                repo_path.as_ref().map(|r| paths_equal_cheap(&path, r))
            } else {
                repo_path.as_ref().map(|r| {
                    if let (Ok(ca), Ok(cb)) = (fs::canonicalize(&path), fs::canonicalize(r)) {
                        ca == cb
                    } else {
                        exists == r.exists()
                    }
                })
            };
            ConfigInfo {
                id: c.id.clone(),
                path,
                exists,
                is_dir,
                in_sync,
            }
        })
        .collect()
}

/// Cheap equality: same canonical path, or same file bytes for files, or same file *count* for dirs.
fn paths_equal_cheap(a: &Path, b: &Path) -> bool {
    if !a.exists() || !b.exists() {
        return false;
    }
    if let (Ok(ca), Ok(cb)) = (fs::canonicalize(a), fs::canonicalize(b)) {
        if ca == cb {
            return true;
        }
    }
    if a.is_file() && b.is_file() {
        return fs::read(a).ok() == fs::read(b).ok();
    }
    if a.is_dir() && b.is_dir() {
        let count = |p: &Path| -> usize {
            walkdir::WalkDir::new(p)
                .max_depth(3)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .count()
        };
        return count(a) == count(b);
    }
    false
}

fn resolve<'a>(manifest: &'a Manifest, id: &str) -> Result<&'a ConfigDef> {
    manifest
        .find_config(id)
        .ok_or_else(|| anyhow!("unknown config `{id}`"))
}

/// List files under a LIVE config (file → single entry; dir → recursive text-ish files).
pub fn list_live_files(manifest: &Manifest, id: &str) -> Result<Vec<PathBuf>> {
    let cfg = resolve(manifest, id)?;
    let path = live_path(cfg);
    if !path.exists() {
        return Err(anyhow!("live config missing: {}", path.display()));
    }
    if path.is_file() {
        return Ok(vec![path]);
    }
    let mut files: Vec<_> = walkdir::WalkDir::new(&path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .filter(|p| is_probably_text(p))
        .collect();
    files.sort();
    Ok(files)
}

fn is_probably_text(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()) {
        Some(ext) => matches!(
            ext.as_str(),
            "lua" | "sh" | "zsh" | "bash" | "toml" | "json" | "yml" | "yaml" | "conf"
                | "cfg" | "txt" | "md" | "rc" | "css" | "js" | "ts" | "c" | "h" | "cpp"
        ),
        None => {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();
            name.contains("rc")
                || name.starts_with('.')
                || name == "makefile"
                || !name.contains('.')
        }
    }
}

/// Open LIVE config in $EDITOR (CLI).
pub fn edit_external(manifest: &Manifest, id: &str) -> Result<()> {
    let cfg = resolve(manifest, id)?;
    let path = live_path(cfg);
    if !path.exists() {
        return Err(anyhow!("config path does not exist: {}", path.display()));
    }
    let target = if path.is_dir() {
        ["init.lua", "sketchybarrc", "yabairc", "skhdrc", "bordersrc"]
            .iter()
            .map(|f| path.join(f))
            .find(|p| p.exists())
            .unwrap_or(path)
    } else {
        path
    };
    edit_path(&target)
}

/// Open LIVE config in simple editor (nano) — TUI `e`/`E`.
pub fn edit_external_simple(manifest: &Manifest, id: &str) -> Result<()> {
    let cfg = resolve(manifest, id)?;
    let path = live_path(cfg);
    if !path.exists() {
        return Err(anyhow!("config path does not exist: {}", path.display()));
    }
    let target = if path.is_dir() {
        ["init.lua", "sketchybarrc", "yabairc", "skhdrc", "bordersrc"]
            .iter()
            .map(|f| path.join(f))
            .find(|p| p.exists())
            .unwrap_or(path)
    } else {
        path
    };
    edit_path_simple(&target)
}

#[allow(dead_code)]
pub fn edit_file_external(path: &Path) -> Result<()> {
    edit_path(path)
}

pub fn edit_file_simple(path: &Path) -> Result<()> {
    edit_path_simple(path)
}

pub fn open(manifest: &Manifest, id: &str) -> Result<()> {
    let cfg = resolve(manifest, id)?;
    let path = live_path(cfg);
    if !path.exists() {
        return Err(anyhow!("config path does not exist: {}", path.display()));
    }
    reveal_path(&path)
}

pub fn edit(manifest: &Manifest, id: &str) -> Result<()> {
    // CLI default: external editor on live path
    edit_external(manifest, id)
}

pub fn backup_all(manifest: &Manifest) -> Result<PathBuf> {
    let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let dest = manifest.backup_dir().join(&stamp);
    ensure_dir(&dest)?;
    for cfg in &manifest.configs {
        let src = live_path(cfg);
        if !src.exists() {
            continue;
        }
        let target = dest.join(&cfg.id);
        copy_recursive(&src, &target).with_context(|| format!("backing up {}", cfg.id))?;
    }
    Ok(dest)
}

pub fn restore_latest(manifest: &Manifest) -> Result<PathBuf> {
    let dir = manifest.backup_dir();
    if !dir.exists() {
        return Err(anyhow!("no backups in {}", dir.display()));
    }
    let mut entries: Vec<_> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    let latest = entries
        .last()
        .ok_or_else(|| anyhow!("no backup snapshots found"))?;
    let src_root = latest.path();
    for cfg in &manifest.configs {
        let src = src_root.join(&cfg.id);
        if !src.exists() {
            continue;
        }
        let dst = live_path(cfg);
        if dst.exists() {
            if dst.is_dir() {
                fs::remove_dir_all(&dst)?;
            } else {
                fs::remove_file(&dst)?;
            }
        }
        copy_recursive(&src, &dst).with_context(|| format!("restoring {}", cfg.id))?;
    }
    Ok(src_root)
}

pub fn diff_all(manifest: &Manifest) -> Result<String> {
    crate::core::rice::diff(manifest, None, None)
}

/// LIVE → rice (all managed). Prefer `macmgr rice push`.
#[allow(dead_code)]
pub fn sync_to_repo(manifest: &Manifest) -> Result<()> {
    crate::core::rice::push(manifest, None, None).map(|_| ())
}

/// rice → LIVE (all managed, with undo). Prefer `macmgr rice pull`.
#[allow(dead_code)]
pub fn sync_from_repo(manifest: &Manifest) -> Result<()> {
    crate::core::rice::pull(manifest, None, None).map(|_| ())
}
