use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::core::config::live_path;
use crate::core::manifest::Manifest;
use crate::core::paths::{ensure_dir, expand_tilde, home_dir};
use crate::util::shell::copy_recursive;

const UNDO_STACK: &str = "undo-stack.json";
const META_FILE: &str = "SNAPSHOT.toml";
const MAX_UNDO: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotKind {
    Manual,
    Undo,
    Auto,
}

impl SnapshotKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SnapshotKind::Manual => "manual",
            SnapshotKind::Undo => "undo",
            SnapshotKind::Auto => "auto",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub id: String,
    pub label: String,
    pub kind: SnapshotKind,
    pub created: String,
    pub host: String,
    pub configs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SnapshotInfo {
    pub meta: SnapshotMeta,
    pub path: PathBuf,
}

pub fn snapshots_dir() -> PathBuf {
    home_dir().join(".local/share/macmgr/snapshots")
}

fn undo_stack_path() -> PathBuf {
    home_dir().join(".local/share/macmgr").join(UNDO_STACK)
}

fn hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

fn sanitize_label(label: &str) -> String {
    let s: String = label
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '_'
            }
        })
        .collect();
    let s = s.trim_matches('-').trim_matches('_').to_string();
    if s.is_empty() {
        "snap".into()
    } else {
        s.chars().take(48).collect()
    }
}

fn write_live_configs(manifest: &Manifest, dest: &Path) -> Result<Vec<String>> {
    let mut configs = Vec::new();
    for cfg in &manifest.configs {
        let src = expand_tilde(&cfg.path);
        if !src.exists() {
            continue;
        }
        let target = dest.join(&cfg.id);
        copy_recursive(&src, &target).with_context(|| format!("snapshot {}", cfg.id))?;
        configs.push(cfg.id.clone());
    }
    Ok(configs)
}

/// Create a snapshot. Optional `label` → dir `TIMESTAMP_label`.
pub fn create(
    manifest: &Manifest,
    label: Option<&str>,
    kind: SnapshotKind,
) -> Result<SnapshotInfo> {
    let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let id = match label.map(str::trim).filter(|s| !s.is_empty()) {
        Some(l) => format!("{stamp}_{}", sanitize_label(l)),
        None => stamp.clone(),
    };
    let dest = snapshots_dir().join(&id);
    if dest.exists() {
        bail!("snapshot already exists: {id}");
    }
    ensure_dir(&dest)?;
    let configs = write_live_configs(manifest, &dest)?;
    let meta = SnapshotMeta {
        id: id.clone(),
        label: label
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(stamp.as_str())
            .to_string(),
        kind,
        created: Local::now().to_rfc3339(),
        host: hostname(),
        configs,
    };
    let toml = toml::to_string_pretty(&meta)
        .unwrap_or_else(|_| format!("id = \"{}\"\nlabel = \"{}\"\n", meta.id, meta.label));
    fs::write(dest.join(META_FILE), toml)?;
    // legacy readable note
    fs::write(
        dest.join("SNAPSHOT.txt"),
        format!(
            "macmgr snapshot {}\nlabel={}\nkind={}\nhost={}\n",
            meta.id,
            meta.label,
            meta.kind.as_str(),
            meta.host
        ),
    )?;
    Ok(SnapshotInfo { meta, path: dest })
}

/// Snapshot current LIVE state and push onto undo stack (before restore/import/sync).
pub fn push_undo(manifest: &Manifest, reason: &str) -> Result<SnapshotInfo> {
    let label = format!("undo-{reason}");
    let info = create(manifest, Some(&label), SnapshotKind::Undo)?;
    let mut stack = load_undo_stack();
    stack.push(info.meta.id.clone());
    while stack.len() > MAX_UNDO {
        if let Some(old) = stack.first().cloned() {
            stack.remove(0);
            let _ = delete(&old);
        } else {
            break;
        }
    }
    save_undo_stack(&stack)?;
    Ok(info)
}

fn load_undo_stack() -> Vec<String> {
    let path = undo_stack_path();
    let Ok(raw) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

fn save_undo_stack(stack: &[String]) -> Result<()> {
    let path = undo_stack_path();
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    fs::write(
        path,
        stack.join("\n") + if stack.is_empty() { "" } else { "\n" },
    )?;
    Ok(())
}

pub fn undo_available() -> Option<String> {
    load_undo_stack().last().cloned()
}

/// Restore the latest undo snapshot (pops stack). Does not push another undo.
pub fn undo(manifest: &Manifest) -> Result<SnapshotInfo> {
    let mut stack = load_undo_stack();
    let id = stack
        .pop()
        .ok_or_else(|| anyhow!("nothing to undo — stack empty"))?;
    save_undo_stack(&stack)?;
    let info = get(&id)?;
    restore_from_path(manifest, &info.path)?;
    Ok(info)
}

fn read_meta(dir: &Path) -> SnapshotMeta {
    let id = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?")
        .to_string();
    let meta_path = dir.join(META_FILE);
    if let Ok(raw) = fs::read_to_string(&meta_path) {
        if let Ok(m) = toml::from_str::<SnapshotMeta>(&raw) {
            return m;
        }
    }
    // legacy SNAPSHOT.txt / stamp-only dirs
    let label = id.clone();
    let kind = if id.contains("undo") {
        SnapshotKind::Undo
    } else {
        SnapshotKind::Manual
    };
    SnapshotMeta {
        id,
        label,
        kind,
        created: String::new(),
        host: String::new(),
        configs: Vec::new(),
    }
}

pub fn list() -> Result<Vec<SnapshotInfo>> {
    let dir = snapshots_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out: Vec<_> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .map(|path| {
            let meta = read_meta(&path);
            SnapshotInfo { meta, path }
        })
        .collect();
    out.sort_by(|a, b| b.meta.id.cmp(&a.meta.id)); // newest first
    Ok(out)
}

pub fn get(id: &str) -> Result<SnapshotInfo> {
    let path = snapshots_dir().join(id);
    if !path.is_dir() {
        // allow prefix match
        let all = list()?;
        let matches: Vec<_> = all
            .into_iter()
            .filter(|s| s.meta.id == id || s.meta.id.starts_with(id) || s.meta.label == id)
            .collect();
        match matches.len() {
            0 => bail!("snapshot not found: {id}"),
            1 => return Ok(matches.into_iter().next().unwrap()),
            _ => bail!("ambiguous snapshot id `{id}` — use full id"),
        }
    }
    Ok(SnapshotInfo {
        meta: read_meta(&path),
        path,
    })
}

fn restore_from_path(manifest: &Manifest, src_root: &Path) -> Result<()> {
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
        if let Some(parent) = dst.parent() {
            ensure_dir(parent)?;
        }
        copy_recursive(&src, &dst).with_context(|| format!("restoring {}", cfg.id))?;
    }
    Ok(())
}

/// Restore snapshot by id. Automatically pushes undo of current LIVE first.
pub fn restore(manifest: &Manifest, id: &str) -> Result<SnapshotInfo> {
    let target = get(id)?;
    let _ = push_undo(manifest, &format!("before-restore-{}", target.meta.id));
    restore_from_path(manifest, &target.path)?;
    Ok(target)
}

pub fn delete(id: &str) -> Result<()> {
    let info = get(id)?;
    fs::remove_dir_all(&info.path).with_context(|| format!("deleting {}", info.path.display()))?;
    let mut stack = load_undo_stack();
    stack.retain(|x| x != &info.meta.id);
    let _ = save_undo_stack(&stack);
    Ok(())
}

pub fn format_line(s: &SnapshotInfo) -> String {
    let kind = match s.meta.kind {
        SnapshotKind::Manual => "MAN",
        SnapshotKind::Undo => "UND",
        SnapshotKind::Auto => "AUT",
    };
    format!(
        "[{kind}] {:<28} {}",
        s.meta.id.chars().take(28).collect::<String>(),
        s.meta.label
    )
}
