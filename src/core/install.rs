use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};

use crate::core::brew;
use crate::core::manifest::Manifest;
use crate::core::paths::{ensure_dir, expand_tilde, home_dir, which};
use crate::core::service;
use crate::util::shell::{run_capture_lossy, run_shell_inherit, run_status};

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

type InstallStep = (String, Box<dyn FnOnce() -> Result<()>>);

fn build_steps(manifest: &Manifest, full: bool) -> Vec<InstallStep> {
    let mut steps: Vec<InstallStep> = Vec::new();

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
        steps.push((format!("tap {name}"), Box::new(move || brew::tap(&name))));
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
        steps.push((label, Box::new(move || run_shell_inherit(&cmd))));
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

    if !rice.exists() || !rice.join(".config").exists() {
        // Public onboarding: seed starter shell/ into rice when empty
        match crate::core::setup::seed_rice_from_shell(manifest, false) {
            Ok(_) => {}
            Err(e) => {
                let any_live = manifest
                    .managed_configs()
                    .any(|c| expand_tilde(&c.path).exists());
                if any_live {
                    println!(
                        "rice mirror missing at {} — keeping existing LIVE configs.\n  Tip: macmgr rice init --git · ({e})",
                        rice.display()
                    );
                    return Ok(());
                }
                println!(
                    "No rice at {} and no LIVE configs yet.\n  Tip: macmgr setup --yes · ({e})",
                    rice.display()
                );
                return Ok(());
            }
        }
    }

    for cfg in manifest.managed_configs() {
        let repo = expand_tilde(&cfg.repo_path);
        let live = expand_tilde(&cfg.path);
        if !repo.exists() {
            // repo_path may point at a file inside rice; also try shell-relative after seed
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

    // Install ensure_spaces helper for new users
    let _ = crate::core::setup::install_ensure_spaces(manifest);
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
    symlink(repo, live)
        .with_context(|| format!("symlink {} -> {}", live.display(), repo.display()))?;
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
            if live
                .symlink_metadata()
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false)
            {
                fs::remove_file(&live).ok();
            }
        }
    }

    println!("Uninstall complete.");
    let _ = home_dir();
    Ok(())
}

/// Options for a full environment wipe (`macmgr purge`).
#[derive(Debug, Clone)]
pub struct PurgeOptions {
    /// Also `brew uninstall` every package listed in the manifest (not only core).
    pub remove_packages: bool,
    /// Delete LIVE config paths from the manifest.
    pub remove_live_configs: bool,
    /// Delete My rice mirror (`~/.local/share/macmgr/rice`).
    pub remove_rice: bool,
    /// Delete macmgr data: snapshots, backups, themes, user.toml, config dir.
    pub remove_macmgr_data: bool,
    /// `cargo uninstall macmgr` and remove the binary if found.
    pub remove_cli: bool,
}

impl Default for PurgeOptions {
    fn default() -> Self {
        Self {
            remove_packages: true,
            remove_live_configs: true,
            remove_rice: true,
            remove_macmgr_data: true,
            remove_cli: false,
        }
    }
}

/// Full teardown of the tiling rice environment and its brew libraries.
pub fn purge(manifest: &Manifest, opts: &PurgeOptions) -> Result<()> {
    println!("macmgr purge — full environment teardown");
    println!("{}", "─".repeat(48));

    // 1) Stop services first
    println!("→ Stopping services…");
    let _ = service::act_all(manifest, "stop", false);
    for svc in &manifest.services {
        let _ = service::stop(svc);
    }

    // 2) Unload / remove LaunchAgents
    println!("→ Removing LaunchAgents…");
    let agents = launch_agents_dir();
    for svc in &manifest.services {
        if svc.label.is_empty() {
            continue;
        }
        let plist = agents.join(format!("{}.plist", svc.label));
        if plist.exists() {
            let uid = run_capture_lossy("id", &["-u"]).trim().to_string();
            let domain = format!("gui/{uid}");
            let _ = std::process::Command::new("launchctl")
                .args(["bootout", &domain, plist.to_str().unwrap_or("")])
                .status();
            // Fallback for older launchctl
            let _ = std::process::Command::new("launchctl")
                .args(["unload", "-w", plist.to_str().unwrap_or("")])
                .status();
            let _ = fs::remove_file(&plist);
            println!("  removed {}", plist.display());
        }
        // Also common brew labels
        for extra in [
            format!("homebrew.mxcl.{}", svc.brew),
            format!("com.koekeishiya.{}", svc.brew),
            format!("com.asmvik.{}", svc.brew),
        ] {
            let p = agents.join(format!("{extra}.plist"));
            if p.exists() {
                let _ = fs::remove_file(&p);
                println!("  removed {}", p.display());
            }
        }
    }

    // 3) Brew uninstall packages (libraries / apps for this rice)
    if opts.remove_packages {
        println!("→ Uninstalling brew packages from manifest…");
        // Core first, then the rest (dedupe by name)
        let mut seen = std::collections::HashSet::new();
        let mut order: Vec<&crate::core::manifest::PackageDef> = Vec::new();
        for p in manifest.core_packages() {
            if seen.insert(p.name.as_str()) {
                order.push(p);
            }
        }
        for p in &manifest.packages {
            if seen.insert(p.name.as_str()) {
                order.push(p);
            }
        }
        for pkg in order {
            print!("  brew uninstall {} ({})… ", pkg.name, pkg.kind);
            match brew::uninstall_package(&pkg.name, &pkg.kind) {
                Ok(()) => println!("ok"),
                Err(e) => println!("skip ({e})"),
            }
        }
    }

    // 4) LIVE configs
    if opts.remove_live_configs {
        println!("→ Removing LIVE configs…");
        for cfg in &manifest.configs {
            let live = expand_tilde(&cfg.path);
            if !live.exists() && live.symlink_metadata().is_err() {
                continue;
            }
            // Never delete bare home or ~/.config itself
            if live == home_dir() || live == expand_tilde("~/.config") {
                println!("  refuse to delete {}", live.display());
                continue;
            }
            match remove_path(&live) {
                Ok(()) => println!("  deleted {}", live.display()),
                Err(e) => println!("  skip {}: {e}", live.display()),
            }
        }
        // kitty theme leftovers
        let kitty_theme = expand_tilde("~/.config/kitty/current-theme.conf");
        let _ = remove_path(&kitty_theme);
    }

    // 5) ensure_spaces helper
    if !manifest.paths.ensure_spaces.is_empty() {
        let p = expand_tilde(&manifest.paths.ensure_spaces);
        if p.exists() {
            let _ = fs::remove_file(&p);
            println!("→ removed {}", p.display());
        }
    }

    // 6) Rice mirror
    if opts.remove_rice {
        let rice = manifest.rice_path();
        if rice.exists() {
            println!("→ Removing rice mirror {}…", rice.display());
            let _ = remove_path(&rice);
        }
        // Parent ~/.local/share/macmgr if only rice/snapshots left — handled below
    }

    // 7) macmgr data dirs
    if opts.remove_macmgr_data {
        println!("→ Removing macmgr data…");
        let share = home_dir().join(".local/share/macmgr");
        for sub in ["rice", "snapshots", "backups"] {
            let p = share.join(sub);
            if p.exists() {
                let _ = remove_path(&p);
                println!("  deleted {}", p.display());
            }
        }
        // empty share dir
        if share.is_dir() {
            let _ = fs::remove_dir(&share);
        }

        let cfg = crate::core::paths::config_dir();
        if cfg.exists() {
            let _ = remove_path(&cfg);
            println!("  deleted {}", cfg.display());
        }
        // directories crate also uses Application Support
        if let Some(pd) = crate::core::paths::project_dirs() {
            for d in [pd.config_dir(), pd.data_dir(), pd.cache_dir()] {
                if d.exists() {
                    let _ = remove_path(d);
                    println!("  deleted {}", d.display());
                }
            }
        }
        // Legacy ~/.config/macmgr if different from project_dirs
        let legacy = home_dir().join(".config/macmgr");
        if legacy.exists() {
            let _ = remove_path(&legacy);
            println!("  deleted {}", legacy.display());
        }
    }

    // 8) Optional CLI removal
    if opts.remove_cli {
        println!("→ Removing macmgr CLI…");
        let _ = std::process::Command::new("cargo")
            .args(["uninstall", "macmgr"])
            .status();
        if let Some(bin) = which("macmgr") {
            // Only remove if under cargo/homebrew paths — never system
            let s = bin.to_string_lossy();
            if s.contains(".cargo") || s.contains("Homebrew") || s.contains("/usr/local") {
                let _ = fs::remove_file(&bin);
                println!("  removed {}", bin.display());
            }
        }
    }

    println!("{}", "─".repeat(48));
    println!("Purge finished.");
    println!("Note: Homebrew taps were kept. SIP/Accessibility settings were not changed.");
    if !opts.remove_cli {
        println!(
            "macmgr CLI is still installed — run: macmgr self-update / cargo uninstall macmgr"
        );
    }
    Ok(())
}

fn remove_path(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if meta.file_type().is_symlink() || meta.is_file() {
        fs::remove_file(path).with_context(|| format!("remove file {}", path.display()))?;
    } else if meta.is_dir() {
        fs::remove_dir_all(path).with_context(|| format!("remove dir {}", path.display()))?;
    }
    Ok(())
}

fn launch_agents_dir() -> PathBuf {
    crate::core::paths::launch_agents_dir()
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
