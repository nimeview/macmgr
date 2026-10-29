//! First-run / public onboarding: seed starter shell → rice → install core.

use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};

use crate::core::doctor;
use crate::core::install::{self, InstallMode};
use crate::core::manifest::Manifest;
use crate::core::paths::{ensure_dir, expand_tilde, home_dir};
use crate::core::snapshot::{self, SnapshotKind};
use crate::util::shell::copy_recursive;

/// Locate the repo `shell/` starter pack (dev tree or next to installed sources).
pub fn find_shell_pack() -> Option<PathBuf> {
    let mut candidates = vec![
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("shell"),
        home_dir().join("src/MacOS_Manager/shell"),
        home_dir().join("Documents/MacOS_Manager/shell"),
    ];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("../shell"));
            candidates.push(parent.join("shell"));
        }
    }
    candidates
        .into_iter()
        .find(|p| p.is_dir() && p.join(".config").is_dir())
}

/// Copy starter `shell/` into the rice mirror (does not overwrite existing files unless `force`).
pub fn seed_rice_from_shell(manifest: &Manifest, force: bool) -> Result<PathBuf> {
    let shell = find_shell_pack()
        .ok_or_else(|| anyhow!("shell/ starter pack not found (clone the repo next to macmgr)"))?;
    let rice = manifest.rice_path();
    ensure_dir(&rice)?;

    if rice.join(".config").exists() && !force {
        println!(
            "rice already has .config at {} — skip seed (use --force to overwrite)",
            rice.display()
        );
        return Ok(rice);
    }

    println!("Seeding rice from {} → {}", shell.display(), rice.display());
    // Copy .config and bin into rice root
    let src_config = shell.join(".config");
    if src_config.is_dir() {
        copy_recursive(&src_config, &rice.join(".config"))?;
    }
    let src_bin = shell.join("bin");
    if src_bin.is_dir() {
        copy_recursive(&src_bin, &rice.join("bin"))?;
    }
    // Root-level files like README optional
    Ok(rice)
}

/// Install ensure_spaces.sh to ~/bin (or path from manifest).
pub fn install_ensure_spaces(manifest: &Manifest) -> Result<PathBuf> {
    let dest = if manifest.paths.ensure_spaces.is_empty() {
        home_dir().join("bin/ensure_spaces.sh")
    } else {
        expand_tilde(&manifest.paths.ensure_spaces)
    };
    if let Some(parent) = dest.parent() {
        ensure_dir(parent)?;
    }

    let src = find_shell_pack()
        .map(|s| s.join("bin/ensure_spaces.sh"))
        .filter(|p| p.exists())
        .or_else(|| {
            let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/ensure_spaces.sh");
            p.exists().then_some(p)
        })
        .ok_or_else(|| anyhow!("ensure_spaces.sh not found in shell/ or scripts/"))?;

    fs::copy(&src, &dest).with_context(|| format!("copy → {}", dest.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest, perms)?;
    }
    println!("ensure_spaces → {}", dest.display());
    Ok(dest)
}

/// Full first-run wizard used by `macmgr setup`.
pub fn run(manifest: &Manifest, yes: bool, force_seed: bool) -> Result<()> {
    println!("macmgr setup\n{}", "─".repeat(48));

    // 1) Snapshot safety net if LIVE already has configs
    let has_live = manifest
        .configs
        .iter()
        .any(|c| expand_tilde(&c.path).exists());
    if has_live {
        match snapshot::create(manifest, Some("before-setup"), SnapshotKind::Manual) {
            Ok(info) => println!("snapshot → {} (undo ready)", info.meta.id),
            Err(e) => println!("snapshot skipped: {e}"),
        }
    }

    // 2) Seed rice from shell/ when missing
    let rice = manifest.rice_path();
    if !rice.join(".config").exists() || force_seed {
        seed_rice_from_shell(manifest, force_seed)?;
    } else {
        println!("rice present at {}", rice.display());
    }

    // 3) Helper script
    let _ = install_ensure_spaces(manifest);

    // 4) Install core stack (brew + plant + services)
    if !yes {
        println!("About to run: macmgr install --scope core");
        println!("Pass --yes to run non-interactively.");
    }
    install::install(manifest, InstallMode::Core, yes)?;

    // 5) Doctor report
    println!("\n{}", doctor::format_report(&doctor::run_checks(manifest)));
    println!("Next:");
    println!("  • System Settings → Privacy → Accessibility → enable yabai & skhd");
    println!("  • Read docs/INSTALL.md if spaces/hotkeys fail (SIP / scripting addition)");
    println!("  • macmgr          # open TUI");
    Ok(())
}

#[allow(dead_code)]
pub fn shell_pack_exists() -> bool {
    find_shell_pack().is_some()
}

pub fn describe_shell_pack() -> String {
    match find_shell_pack() {
        Some(p) => format!("shell pack: {}", p.display()),
        None => "shell pack: not found".into(),
    }
}
