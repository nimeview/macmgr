use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::core::config::live_path;
use crate::core::manifest::Manifest;
use crate::core::paths::{config_dir, ensure_dir, expand_tilde, home_dir};
use crate::core::plugins;
use crate::core::snapshot::{self, SnapshotKind};
use crate::util::shell::copy_recursive;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackMeta {
    pub format: String,
    pub created: String,
    pub host: String,
    pub rice_name: String,
    pub rice_version: String,
    pub configs: Vec<String>,
    pub packages: Vec<String>,
    pub plugins_enabled: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ImportOpts {
    pub install_packages: bool,
    pub reload: bool,
}

impl Default for ImportOpts {
    fn default() -> Self {
        Self {
            install_packages: false,
            reload: true,
        }
    }
}

fn hostname() -> String {
    Command::new("hostname")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

fn default_export_path() -> PathBuf {
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    home_dir().join(format!("Desktop/macmgr-rice-{stamp}.tar.gz"))
}

/// Stage LIVE rice into a temp dir, then `tar czf` to `dest` (.tar.gz).
pub fn export(manifest: &Manifest, dest: Option<&Path>) -> Result<PathBuf> {
    let dest = match dest {
        Some(p) => {
            let p = if p.as_os_str().is_empty() {
                default_export_path()
            } else {
                expand_tilde(p.to_str().unwrap_or(""))
            };
            if p.extension().and_then(|e| e.to_str()) == Some("gz")
                || p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.ends_with(".tar.gz"))
                    .unwrap_or(false)
            {
                p
            } else if p.is_dir() || !p.exists() && p.extension().is_none() {
                p.join(format!(
                    "macmgr-rice-{}.tar.gz",
                    Local::now().format("%Y%m%d-%H%M%S")
                ))
            } else {
                p
            }
        }
        None => default_export_path(),
    };

    if let Some(parent) = dest.parent() {
        ensure_dir(parent)?;
    }

    let staging = home_dir().join(format!(
        ".local/share/macmgr/export-staging-{}",
        Local::now().format("%Y%m%d%H%M%S")
    ));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    ensure_dir(&staging)?;

    let mut config_ids = Vec::new();
    let configs_dir = staging.join("configs");
    ensure_dir(&configs_dir)?;
    for cfg in &manifest.configs {
        let src = expand_tilde(&cfg.path);
        if !src.exists() {
            continue;
        }
        copy_recursive(&src, &configs_dir.join(&cfg.id))?;
        config_ids.push(cfg.id.clone());
    }

    // manifest.toml (prefer installed config, else CWD)
    let manifest_src = {
        let cfg = config_dir().join("manifest.toml");
        if cfg.exists() {
            cfg
        } else {
            PathBuf::from("manifest.toml")
        }
    };
    if manifest_src.exists() {
        fs::copy(&manifest_src, staging.join("manifest.toml"))?;
    }

    let packages: Vec<String> = manifest
        .packages
        .iter()
        .map(|p| format!("{}:{}", p.kind, p.name))
        .collect();
    fs::write(staging.join("packages.txt"), packages.join("\n") + "\n")?;

    let plugins_enabled: Vec<String> = plugins::list(manifest)
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p.enabled)
        .map(|p| p.id)
        .collect();
    fs::write(
        staging.join("plugins-enabled.txt"),
        plugins_enabled.join("\n") + "\n",
    )?;

    let meta = PackMeta {
        format: "macmgr-pack-v1".into(),
        created: Local::now().to_rfc3339(),
        host: hostname(),
        rice_name: manifest.name.clone(),
        rice_version: manifest.version.clone(),
        configs: config_ids,
        packages: packages.clone(),
        plugins_enabled,
    };
    fs::write(
        staging.join("PACK.toml"),
        toml::to_string_pretty(&meta).unwrap_or_default(),
    )?;

    // tar czf
    let status = Command::new("tar")
        .arg("-czf")
        .arg(&dest)
        .arg("-C")
        .arg(&staging)
        .arg(".")
        .status()
        .context("tar -czf")?;
    let _ = fs::remove_dir_all(&staging);
    if !status.success() {
        bail!("tar failed creating {}", dest.display());
    }
    Ok(dest)
}

fn resolve_pack_root(src: &Path) -> Result<(PathBuf, Option<PathBuf>)> {
    // Returns (root_with_PACK_or_configs, optional_temp_to_cleanup)
    if src.is_dir() {
        return Ok((src.to_path_buf(), None));
    }
    if !src.exists() {
        return Err(anyhow!("pack not found: {}", src.display()));
    }
    let tmp = home_dir().join(format!(
        ".local/share/macmgr/import-{}",
        Local::now().format("%Y%m%d%H%M%S")
    ));
    ensure_dir(&tmp)?;
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(src)
        .arg("-C")
        .arg(&tmp)
        .status()
        .context("tar -xzf")?;
    if !status.success() {
        let _ = fs::remove_dir_all(&tmp);
        bail!("failed to extract {}", src.display());
    }
    Ok((tmp.clone(), Some(tmp)))
}

/// Import pack: push undo, restore configs from pack, optionally note packages.
pub fn import(manifest: &Manifest, src: &Path, opts: ImportOpts) -> Result<String> {
    let src = expand_tilde(src.to_str().unwrap_or(""));
    let (root, cleanup) = resolve_pack_root(&src)?;
    let result = (|| {
        let configs_dir = if root.join("configs").is_dir() {
            root.join("configs")
        } else if root.join("PACK.toml").exists() || root.join("yabai").exists() {
            // maybe flat configs at root (snapshot-like)
            root.clone()
        } else {
            // nested single folder
            let kids: Vec<_> = fs::read_dir(&root)?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            if kids.len() == 1 && kids[0].join("configs").is_dir() {
                kids[0].join("configs")
            } else if kids.len() == 1 {
                kids[0].clone()
            } else {
                root.join("configs")
            }
        };

        if !configs_dir.exists() {
            bail!("no configs/ in pack {}", src.display());
        }

        let _undo = snapshot::push_undo(manifest, "before-import")?;

        let mut restored = Vec::new();
        for cfg in &manifest.configs {
            let from = configs_dir.join(&cfg.id);
            if !from.exists() {
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
            copy_recursive(&from, &dst)?;
            restored.push(cfg.id.clone());
        }

        // optional: copy pack manifest into macmgr config (don't overwrite silently — side file)
        let pack_manifest = root.join("manifest.toml");
        if pack_manifest.exists() {
            let side = config_dir().join("manifest.imported.toml");
            ensure_dir(&config_dir())?;
            let _ = fs::copy(&pack_manifest, &side);
        }

        let mut msg = format!(
            "imported {} config(s): {}",
            restored.len(),
            restored.join(", ")
        );

        if opts.install_packages {
            if let Ok(raw) = fs::read_to_string(root.join("packages.txt")) {
                for line in raw.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    let name = line.split(':').next_back().unwrap_or(line);
                    if let Some(def) = manifest.find_package(name) {
                        match crate::core::brew::install_package(def) {
                            Ok(()) => msg.push_str(&format!("\n+ brew {}", def.name)),
                            Err(e) => msg.push_str(&format!("\n! {}: {e}", def.name)),
                        }
                    }
                }
            }
        } else if root.join("packages.txt").exists() {
            msg.push_str("\n(packages listed in pack — use --install-packages to brew install)");
        }

        if opts.reload {
            match crate::core::plugins::reload_bar(manifest) {
                Ok(()) => msg.push_str("\nbar reloaded"),
                Err(e) => msg.push_str(&format!("\nreload skipped: {e}")),
            }
        }

        // also keep a named snapshot of what we imported
        let _ = snapshot::create(manifest, Some("after-import"), SnapshotKind::Auto);

        Ok(msg)
    })();

    if let Some(tmp) = cleanup {
        let _ = fs::remove_dir_all(tmp);
    }
    result
}
