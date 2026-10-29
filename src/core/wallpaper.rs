use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Result};

use crate::core::manifest::Manifest;

pub fn list(manifest: &Manifest) -> Result<Vec<PathBuf>> {
    let dir = manifest.wallpaper_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files: Vec<_> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| {
                    matches!(
                        e.to_lowercase().as_str(),
                        "jpg" | "jpeg" | "png" | "heic" | "webp"
                    )
                })
                .unwrap_or(false)
        })
        .collect();
    files.sort();
    Ok(files)
}

pub fn set(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(anyhow!("wallpaper not found: {}", path.display()));
    }
    let path_str = path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let script = format!(
        r#"tell application "System Events" to tell every desktop to set picture to "{path_str}""#
    );
    let status = Command::new("osascript").args(["-e", &script]).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("failed to set wallpaper"))
    }
}

pub fn set_by_index(manifest: &Manifest, index: usize) -> Result<()> {
    let files = list(manifest)?;
    let path = files
        .get(index)
        .ok_or_else(|| anyhow!("index {index} out of range (0..{})", files.len()))?;
    set(path)
}

pub fn set_by_name(manifest: &Manifest, name: &str) -> Result<()> {
    let files = list(manifest)?;
    let path = files
        .iter()
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == name || n.contains(name))
                .unwrap_or(false)
        })
        .ok_or_else(|| anyhow!("wallpaper `{name}` not found"))?;
    set(path)
}
