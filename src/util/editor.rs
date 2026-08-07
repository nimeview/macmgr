use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{anyhow, Context, Result};

use crate::core::paths::which;

/// Full editor for CLI: `$VISUAL` → `$EDITOR` → hx/nvim/vim/nano → `open -t`.
pub fn resolve_editor() -> String {
    for var in ["VISUAL", "EDITOR"] {
        if let Ok(editor) = std::env::var(var) {
            let editor = editor.trim();
            if !editor.is_empty() {
                return editor.to_string();
            }
        }
    }
    for candidate in ["hx", "helix", "nvim", "vim", "nano"] {
        if which(candidate).is_some() {
            return candidate.to_string();
        }
    }
    "open".to_string()
}

/// Simple TUI editor for `e`/`E`: `$MACMGR_EDITOR` → nano → pico → micro → `$EDITOR`.
pub fn resolve_simple_editor() -> String {
    if let Ok(editor) = std::env::var("MACMGR_EDITOR") {
        let editor = editor.trim();
        if !editor.is_empty() {
            return editor.to_string();
        }
    }
    for candidate in ["nano", "pico", "micro"] {
        if which(candidate).is_some() {
            return candidate.to_string();
        }
    }
    resolve_editor()
}

pub fn edit_path(path: &Path) -> Result<()> {
    edit_with(&resolve_editor(), path)
}

/// Open file in a simple terminal editor (nano-like) — used by TUI `e`/`E`.
pub fn edit_path_simple(path: &Path) -> Result<()> {
    edit_with(&resolve_simple_editor(), path)
}

fn edit_with(editor: &str, path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(anyhow!("file not found: {}", path.display()));
    }

    if editor == "open" {
        let status = Command::new("open")
            .arg("-t")
            .arg("-W")
            .arg(path)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context("open -t")?;
        if status.success() {
            return Ok(());
        }
        return Err(anyhow!("open exited with {status}"));
    }

    let mut parts = editor.split_whitespace();
    let bin = parts.next().ok_or_else(|| anyhow!("empty editor"))?;
    let mut cmd = Command::new(bin);
    for arg in parts {
        cmd.arg(arg);
    }
    cmd.arg(path);
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    println!("Opening {} with `{editor}`…", path.display());
    println!("(Ctrl+O save · Ctrl+X exit  — if nano)");
    let status = cmd
        .status()
        .with_context(|| format!("launching editor `{editor}`"))?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("editor `{editor}` exited with {status}"))
    }
}

pub fn reveal_path(path: &Path) -> Result<()> {
    Command::new("open")
        .arg("-R")
        .arg(path)
        .status()
        .context("open -R")?;
    Ok(())
}
