use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use anyhow::{anyhow, Context, Result};

fn brew_quiet_cmd(program: &str) -> Command {
    let mut cmd = Command::new(program);
    cmd.env("HOMEBREW_NO_ENV_HINTS", "1")
        .env("HOMEBREW_NO_INSTALL_CLEANUP", "1")
        .env("HOMEBREW_NO_AUTO_UPDATE", "1")
        .env("HOMEBREW_COLOR", "0")
        .env("HOMEBREW_NO_COLOR", "1");
    cmd
}

/// Run capturing all output (safe inside TUI — never paints the screen).
pub fn run(program: &str, args: &[&str]) -> Result<Output> {
    brew_quiet_cmd(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("failed to run `{program}`"))
}

/// Run and require success; stdout/stderr are captured (TUI-safe).
pub fn run_status(program: &str, args: &[&str]) -> Result<()> {
    let output = run(program, args)?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else {
            stdout.trim().to_string()
        };
        Err(anyhow!("`{program} {}` failed: {detail}", args.join(" ")))
    }
}

/// Run with inherited stdio (only when TUI is suspended / CLI).
pub fn run_status_inherit(program: &str, args: &[&str]) -> Result<()> {
    let status = brew_quiet_cmd(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to run `{program}`"))?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "`{program} {}` exited with {}",
            args.join(" "),
            status
        ))
    }
}

pub fn run_capture(program: &str, args: &[&str]) -> Result<String> {
    let output = run(program, args)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "`{program} {}` failed: {}",
            args.join(" "),
            stderr.trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn run_capture_lossy(program: &str, args: &[&str]) -> String {
    run(program, args)
        .map(|o| {
            let mut s = String::from_utf8_lossy(&o.stdout).to_string();
            if s.trim().is_empty() {
                s = String::from_utf8_lossy(&o.stderr).to_string();
            }
            s
        })
        .unwrap_or_default()
}

pub fn run_shell_inherit(script: &str) -> Result<()> {
    let status = Command::new("zsh")
        .arg("-lc")
        .arg(script)
        .status()
        .context("failed to spawn zsh")?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("shell command failed"))
    }
}

pub fn copy_recursive(src: &Path, dst: &Path) -> Result<()> {
    if src.is_file() {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
        return Ok(());
    }
    fs::create_dir_all(dst)?;
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(src)?;
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
