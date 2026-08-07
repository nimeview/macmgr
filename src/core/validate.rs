use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

use crate::core::manifest::Manifest;
use crate::core::paths::{expand_tilde, which};

/// Syntax-check a single Lua file (`luac -p`, fallback `lua -e assert(loadfile(...))`).
pub fn lua_file(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(anyhow!("file not found: {}", path.display()));
    }
    if path.extension().and_then(|e| e.to_str()) != Some("lua") {
        return Ok(());
    }

    if which("luac").is_some() {
        let out = Command::new("luac")
            .arg("-p")
            .arg(path)
            .output()
            .with_context(|| format!("running luac -p {}", path.display()))?;
        if out.status.success() {
            return Ok(());
        }
        let err = String::from_utf8_lossy(&out.stderr);
        let err = err.trim();
        bail!("Lua syntax error in {}:\n{err}", path.display());
    }

    if which("lua").is_some() {
        let expr = format!(
            "local f,e=loadfile([=[{}]=]); if not f then error(e) end",
            path.display()
        );
        let out = Command::new("lua")
            .arg("-e")
            .arg(&expr)
            .output()
            .with_context(|| format!("running lua loadfile {}", path.display()))?;
        if out.status.success() {
            return Ok(());
        }
        let err = String::from_utf8_lossy(&out.stderr);
        bail!(
            "Lua syntax error in {}:\n{}",
            path.display(),
            err.trim()
        );
    }

    // No checker installed — don't block reload, but warn via Err? Soft-pass.
    // Prefer failing closed only when we can check. Soft OK with note.
    Ok(())
}

/// Collect all `.lua` under LIVE sketchybar (skip hidden / `.macmgr.bak`).
pub fn sketchybar_lua_files(manifest: &Manifest) -> Result<Vec<PathBuf>> {
    let Some(cfg) = manifest.find_config("sketchybar") else {
        return Ok(Vec::new());
    };
    let root = expand_tilde(&cfg.path);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("lua") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if name.starts_with('.') || name.contains(".macmgr.bak") {
            continue;
        }
        files.push(path.to_path_buf());
    }
    files.sort();
    Ok(files)
}

/// Validate LIVE sketchybar Lua before reload. Fails on first syntax error.
pub fn sketchybar_live(manifest: &Manifest) -> Result<()> {
    if which("luac").is_none() && which("lua").is_none() {
        eprintln!("warn: luac/lua not found — skip Lua validation before reload");
        return Ok(());
    }
    let files = sketchybar_lua_files(manifest)?;
    if files.is_empty() {
        return Ok(());
    }
    let mut errors = Vec::new();
    for path in &files {
        if let Err(e) = lua_file(path) {
            errors.push(e.to_string());
        }
    }
    if errors.is_empty() {
        return Ok(());
    }
    bail!(
        "reload blocked — Lua validation failed ({} file{}):\n\n{}",
        errors.len(),
        if errors.len() == 1 { "" } else { "s" },
        errors.join("\n\n")
    );
}

/// Validate one path (and full bar tree if it's under sketchybar) before reload.
pub fn before_reload(manifest: &Manifest, changed: Option<&Path>) -> Result<()> {
    if let Some(path) = changed {
        if path.extension().and_then(|e| e.to_str()) == Some("lua") {
            lua_file(path)?;
        }
    }
    sketchybar_live(manifest)
}
