use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};

use crate::core::brew;
use crate::core::manifest::Manifest;
use crate::core::paths::{ensure_dir, expand_tilde, home_dir, which};
use crate::core::service;
use crate::util::shell::{run_shell_inherit, run_status};

pub enum InstallMode {
    Core,
    Full,
}

pub fn install(manifest: &Manifest, mode: InstallMode, yes: bool) -> Result<()> {
    let _ = yes;
    let full = matches!(mode, InstallMode::Full);
    let steps = build_steps(manifest, full);
    let pb = ProgressBar::new(steps.len() as u64);
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("##-"),
    );

    for (label, action) in steps {
        pb.set_message(label.clone());
        action()?;
        pb.inc(1);
    }
    pb.finish_with_message("install complete");
    println!("\nNext steps:");
    println!("  1. Grant Accessibility to yabai and skhd (System Settings → Privacy)");
    println!("  2. Optionally disable SIP for advanced yabai features");
    println!("  3. Run: macmgr doctor");
    Ok(())
}

fn build_steps(
    manifest: &Manifest,
    full: bool,
) -> Vec<(String, Box<dyn FnOnce() -> Result<()>>)> {
    let mut steps: Vec<(String, Box<dyn FnOnce() -> Result<()>>)> = Vec::new();

    // xcode CLT
    steps.push((
        "Xcode CLT".into(),
        Box::new(|| {
            if which("xcode-select").is_some() {
                let _ = run_status("xcode-select", &["-p"]);
            }
            Ok(())
        }),
    ));

    // brew
    let need_brew = !brew::brew_available();
    if need_brew {
        steps.push((
            "Install Homebrew".into(),
            Box::new(|| {
                run_shell_inherit(
                    "/bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"",
                )
            }),
        ));
    }

    for tap in manifest.taps.clone() {
        let name = tap.name.clone();
        steps.push((
            format!("tap {name}"),
            Box::new(move || brew::tap(&name)),
        ));
    }

    for pkg in manifest.install_packages(full) {
        let pkg = pkg.clone();
        steps.push((
            format!("install {} ({})", pkg.name, pkg.kind),
            Box::new(move || brew::install_package_visible(&pkg)),
        ));
    }

    // plant configs
    let m = manifest.clone();
    steps.push((
        "Plant configuration files".into(),
        Box::new(move || plant_configs(&m)),
    ));

    for post in manifest.post_install.clone() {
        let cmd = post.command.clone();
        let label = post.label.clone();
        steps.push((
            label,
            Box::new(move || run_shell_inherit(&cmd)),
        ));
    }

    // defaults (core only for both modes; full applies all core=true)
    for def in manifest.defaults.clone().into_iter().filter(|d| d.core) {
        steps.push((
            format!("defaults {}", def.id),
            Box::new(move || apply_default(&def.domain, &def.key, &def.value, &def.r#type)),
        ));
    }

    let m2 = manifest.clone();
    steps.push((
        "Start core services".into(),
        Box::new(move || service::act_all(&m2, "start", true)),
    ));

    steps
}

fn plant_configs(manifest: &Manifest) -> Result<()> {
    let rice = manifest.rice_path();
    let strategy = manifest.rice_config().strategy;

    if !rice.exists() {
        let any_live = manifest
            .managed_configs()
            .any(|c| expand_tilde(&c.path).exists());
        if any_live {
            println!(
                "rice mirror missing at {} — keeping existing LIVE configs.\n  Tip: macmgr rice init --git",
                rice.display()
            );
            return Ok(());
        }
        println!(
            "No rice at {} and no LIVE configs yet.\n  Tip: macmgr rice init · macmgr import <pack.tar.gz>",
            rice.display()
        );
        return Ok(());
    }

    for cfg in manifest.managed_configs() {
        let repo = expand_tilde(&cfg.repo_path);
        let live = expand_tilde(&cfg.path);
        if !repo.exists() {
            continue;
        }
        if strategy == "symlink" {
            link_path(&repo, &live)?;
        } else {
            if live.exists() {
                if live.is_dir() {
                    fs::remove_dir_all(&live)?;
                } else {
                    fs::remove_file(&live)?;
                }
            }
            crate::util::shell::copy_recursive(&repo, &live)?;
        }
    }
    Ok(())
}

fn link_path(repo: &PathBuf, live: &PathBuf) -> Result<()> {
    if let Some(parent) = live.parent() {
        ensure_dir(parent)?;
    }
    if live.exists() || live.symlink_metadata().is_ok() {
        // already linked?
        if let (Ok(a), Ok(b)) = (fs::canonicalize(live), fs::canonicalize(repo)) {
            if a == b {
                return Ok(());
            }
        }
        // don't destroy live sketchybar blindly if different — backup first
        let bak = live.with_extension("macmgr.bak");
        if live.is_dir() && !live.is_symlink() {
            if bak.exists() {
                fs::remove_dir_all(&bak).ok();
            }
            fs::rename(live, &bak).ok();
        } else {
            fs::remove_file(live).or_else(|_| fs::remove_dir_all(live))?;
        }
    }
    symlink(repo, live).with_context(|| {
        format!(
            "symlink {} -> {}",
            live.display(),
            repo.display()
        )
    })?;
    Ok(())
}

fn apply_default(domain: &str, key: &str, value: &str, ty: &str) -> Result<()> {
    let flag = match ty {
        "bool" => "-bool",
        "int" => "-int",
        "float" => "-float",
        _ => "-string",
    };
    run_status("defaults", &["write", domain, key, flag, value])
}

pub fn uninstall(manifest: &Manifest, keep_configs: bool, yes: bool) -> Result<()> {
    let _ = yes;
    println!("Stopping core services...");
    let _ = service::act_all(manifest, "stop", true);

    println!("Uninstalling core brew packages...");
    for pkg in manifest.core_packages() {
        let _ = brew::uninstall_package(&pkg.name, &pkg.kind);
    }

    if !keep_configs {
        println!("Removing planted config symlinks (rice mirror kept)...");
        for cfg in &manifest.configs {
            let live = expand_tilde(&cfg.path);
            if live.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
                fs::remove_file(&live).ok();
            }
        }
    }

    println!("Uninstall complete.");
    let _ = home_dir();
    Ok(())
}

pub fn self_install() -> Result<PathBuf> {
    let status = std::process::Command::new("cargo")
        .args(["install", "--path", ".", "--force"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .context("cargo install")?;
    if !status.success() {
        return Err(anyhow!("cargo install failed"));
    }
    // copy manifest to config dir
    let dest_dir = crate::core::paths::config_dir();
    ensure_dir(&dest_dir)?;
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("manifest.toml");
    let dst = dest_dir.join("manifest.toml");
    fs::copy(&src, &dst)?;
    println!("Installed macmgr and wrote {}", dst.display());
    Ok(dst)
}
