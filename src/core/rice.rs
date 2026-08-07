use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

use crate::core::config::{self, live_path};
use crate::core::manifest::{ConfigDef, Manifest};
use crate::core::paths::{ensure_dir, expand_tilde, which};
use crate::core::snapshot;
use crate::util::shell::copy_recursive;

#[derive(Debug, Clone)]
pub struct RiceStatus {
    pub id: String,
    pub live_exists: bool,
    pub rice_exists: bool,
    pub in_sync: Option<bool>,
    #[allow(dead_code)]
    pub live: PathBuf,
    #[allow(dead_code)]
    pub rice: PathBuf,
}

pub fn rice_dir(manifest: &Manifest) -> PathBuf {
    manifest.rice_path()
}

pub fn exists(manifest: &Manifest) -> bool {
    let p = rice_dir(manifest);
    p.is_dir()
}

fn resolve_cfg<'a>(manifest: &'a Manifest, id: &str) -> Result<&'a ConfigDef> {
    manifest
        .find_config(id)
        .ok_or_else(|| anyhow!("unknown config `{id}`"))
}

fn rice_path_for(cfg: &ConfigDef) -> PathBuf {
    expand_tilde(&cfg.repo_path)
}

fn paths_equal(a: &Path, b: &Path) -> bool {
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
        return dir_fingerprint(a) == dir_fingerprint(b);
    }
    false
}

fn dir_fingerprint(root: &Path) -> Vec<(String, u64)> {
    let mut entries: Vec<_> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let rel = e
                .path()
                .strip_prefix(root)
                .ok()?
                .to_string_lossy()
                .to_string();
            if rel.contains(".macmgr.bak") || rel.starts_with('.') {
                return None;
            }
            let len = e.metadata().ok()?.len();
            Some((rel, len))
        })
        .collect();
    entries.sort();
    entries
}

/// Create rice dir, optional `git init`, seed from LIVE (push all managed).
pub fn init(manifest: &Manifest, with_git: bool) -> Result<PathBuf> {
    let root = rice_dir(manifest);
    ensure_dir(&root)?;

    if with_git && which("git").is_some() && !root.join(".git").exists() {
        let status = Command::new("git")
            .arg("init")
            .current_dir(&root)
            .status()
            .context("git init")?;
        if !status.success() {
            bail!("git init failed in {}", root.display());
        }
        let gitignore = root.join(".gitignore");
        if !gitignore.exists() {
            fs::write(
                &gitignore,
                "*.macmgr.bak\n.DS_Store\n",
            )?;
        }
    }

    let n = push(manifest, None, None)?;
    let cfg = manifest.rice_config();
    if !cfg.git_url.is_empty() {
        // record hint only — do not auto-add remote (user controls remotes)
        fs::write(
            root.join("REMOTE.txt"),
            format!(
                "Suggested remote:\n  git remote add origin {}\n  git add -A && git commit -m 'init rice' && git push -u origin main\n",
                cfg.git_url
            ),
        )?;
    }

    fs::write(
        root.join("README.md"),
        format!(
            "# {} rice mirror\n\nManaged by **macmgr**.\n\n- LIVE (`~/.config/...`) is the daily source of truth.\n- This directory is the publish/sync mirror.\n- Use `macmgr rice push|pull|diff` for selective sync.\n\nSeeded {} managed config(s).\n",
            manifest.name, n
        ),
    )?;

    Ok(root)
}

pub fn status(manifest: &Manifest) -> Vec<RiceStatus> {
    manifest
        .managed_configs()
        .map(|c| {
            let live = live_path(c);
            let rice = rice_path_for(c);
            let live_exists = live.exists();
            let rice_exists = rice.exists();
            let in_sync = if live_exists && rice_exists {
                Some(paths_equal(&live, &rice))
            } else if !live_exists && !rice_exists {
                Some(true)
            } else {
                Some(false)
            };
            RiceStatus {
                id: c.id.clone(),
                live_exists,
                rice_exists,
                in_sync,
                live,
                rice,
            }
        })
        .collect()
}

#[allow(dead_code)]
pub fn list_files(manifest: &Manifest, id: &str) -> Result<Vec<PathBuf>> {
    let cfg = resolve_cfg(manifest, id)?;
    let live = live_path(cfg);
    if live.is_dir() {
        config::list_live_files(manifest, id)
    } else if live.is_file() {
        Ok(vec![live])
    } else {
        Ok(Vec::new())
    }
}

fn copy_one(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Err(anyhow!("missing source {}", src.display()));
    }
    if let Some(parent) = dst.parent() {
        ensure_dir(parent)?;
    }
    if dst.exists() {
        if dst.is_dir() {
            fs::remove_dir_all(dst)?;
        } else {
            fs::remove_file(dst)?;
        }
    }
    copy_recursive(src, dst)?;
    Ok(())
}

fn relative_under(root: &Path, file: &Path) -> Result<PathBuf> {
    let file = if file.exists() {
        fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf())
    } else {
        file.to_path_buf()
    };
    let root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    file.strip_prefix(&root)
        .map(|p| p.to_path_buf())
        .map_err(|_| {
            anyhow!(
                "{} is not under {}",
                file.display(),
                root.display()
            )
        })
}

/// LIVE → rice. Optional `id` / relative-or-absolute `file` under that config.
/// Returns number of configs (or files) touched.
pub fn push(manifest: &Manifest, id: Option<&str>, file: Option<&Path>) -> Result<usize> {
    ensure_dir(&rice_dir(manifest))?;
    let configs: Vec<&ConfigDef> = match id {
        Some(id) => {
            let c = resolve_cfg(manifest, id)?;
            if !c.managed || c.repo_path.is_empty() {
                bail!("config `{id}` is not rice-managed");
            }
            vec![c]
        }
        None => manifest.managed_configs().collect(),
    };

    let mut n = 0;
    for cfg in configs {
        let live = live_path(cfg);
        let rice = rice_path_for(cfg);
        if let Some(f) = file {
            if id.is_none() {
                bail!("--file requires a config id");
            }
            let live_root = &live;
            let rel = if f.is_absolute() {
                relative_under(live_root, f)?
            } else {
                PathBuf::from(f)
            };
            let src = live_root.join(&rel);
            let dst = rice.join(&rel);
            if !src.exists() {
                bail!("live file missing: {}", src.display());
            }
            copy_one(&src, &dst)?;
            n += 1;
        } else {
            if !live.exists() {
                continue;
            }
            copy_one(&live, &rice)?;
            n += 1;
        }
    }
    Ok(n)
}

/// rice → LIVE. Creates undo snapshot first. Returns number touched.
pub fn pull(manifest: &Manifest, id: Option<&str>, file: Option<&Path>) -> Result<usize> {
    let _ = snapshot::push_undo(manifest, "before-rice-pull");
    let configs: Vec<&ConfigDef> = match id {
        Some(id) => {
            let c = resolve_cfg(manifest, id)?;
            if !c.managed || c.repo_path.is_empty() {
                bail!("config `{id}` is not rice-managed");
            }
            vec![c]
        }
        None => manifest.managed_configs().collect(),
    };

    let mut n = 0;
    for cfg in configs {
        let live = live_path(cfg);
        let rice = rice_path_for(cfg);
        if let Some(f) = file {
            if id.is_none() {
                bail!("--file requires a config id");
            }
            let rel = if f.is_absolute() {
                // try relative to rice or live
                relative_under(&rice, f)
                    .or_else(|_| relative_under(&live, f))?
            } else {
                PathBuf::from(f)
            };
            let src = rice.join(&rel);
            let dst = live.join(&rel);
            if !src.exists() {
                bail!("rice file missing: {}", src.display());
            }
            copy_one(&src, &dst)?;
            n += 1;
        } else {
            if !rice.exists() {
                continue;
            }
            copy_one(&rice, &live)?;
            n += 1;
        }
    }
    Ok(n)
}

pub fn diff(manifest: &Manifest, id: Option<&str>, file: Option<&Path>) -> Result<String> {
    let mut out = String::new();
    let configs: Vec<&ConfigDef> = match id {
        Some(id) => vec![resolve_cfg(manifest, id)?],
        None => manifest.managed_configs().collect(),
    };

    for cfg in configs {
        let live = live_path(cfg);
        let rice = rice_path_for(cfg);

        if let Some(f) = file {
            let rel = if f.is_absolute() {
                relative_under(&live, f)
                    .or_else(|_| relative_under(&rice, f))?
            } else {
                PathBuf::from(f)
            };
            let a = live.join(&rel);
            let b = rice.join(&rel);
            out.push_str(&format!("[{}] {}\n", cfg.id, rel.display()));
            out.push_str(&diff_paths(&a, &b)?);
            continue;
        }

        out.push_str(&format!("=== {} ===\n", cfg.id));
        if !live.exists() && !rice.exists() {
            out.push_str("(neither exists)\n");
            continue;
        }
        if !live.exists() {
            out.push_str(&format!("LIVE missing · rice={}\n", rice.display()));
            continue;
        }
        if !rice.exists() {
            out.push_str(&format!(
                "rice missing · LIVE={} · run: macmgr rice push {}\n",
                live.display(),
                cfg.id
            ));
            continue;
        }
        if paths_equal(&live, &rice) {
            out.push_str("in sync\n");
            continue;
        }
        if live.is_file() && rice.is_file() {
            out.push_str(&diff_paths(&live, &rice)?);
        } else {
            out.push_str(&diff_dirs(&live, &rice)?);
        }
    }
    Ok(out)
}

fn diff_paths(a: &Path, b: &Path) -> Result<String> {
    if which("diff").is_none() {
        let same = paths_equal(a, b);
        return Ok(if same {
            "identical\n".into()
        } else {
            format!("DIFFER · {} vs {}\n", a.display(), b.display())
        });
    }
    let output = Command::new("diff")
        .args(["-u", "--label", "LIVE", "--label", "rice"])
        .arg(a)
        .arg(b)
        .output()
        .context("diff")?;
    let text = String::from_utf8_lossy(&output.stdout);
    if text.trim().is_empty() {
        Ok("identical\n".into())
    } else {
        Ok(text.into_owned())
    }
}

fn diff_dirs(a: &Path, b: &Path) -> Result<String> {
    if which("diff").is_some() {
        let output = Command::new("diff")
            .args(["-ru", "--exclude", ".git", "--exclude", "*.macmgr.bak"])
            .arg(a)
            .arg(b)
            .output()
            .context("diff -ru")?;
        let text = String::from_utf8_lossy(&output.stdout);
        if text.trim().is_empty() {
            return Ok("in sync (no textual diff)\n".into());
        }
        // truncate very large diffs
        let lines: Vec<_> = text.lines().collect();
        if lines.len() > 200 {
            return Ok(format!(
                "{}\n… ({} more lines) · use: macmgr rice diff <id> --file …\n",
                lines[..200].join("\n"),
                lines.len() - 200
            ));
        }
        return Ok(text.into_owned());
    }
    let fa = dir_fingerprint(a);
    let fb = dir_fingerprint(b);
    Ok(format!(
        "DIFFER · LIVE {} files · rice {} files\n",
        fa.len(),
        fb.len()
    ))
}

pub fn format_status_line(s: &RiceStatus) -> String {
    let mark = match s.in_sync {
        Some(true) => "sync",
        Some(false) => "DIFF",
        None => "—",
    };
    format!(
        "{:<12} {:<4} live={} rice={}",
        s.id,
        mark,
        if s.live_exists { "yes" } else { "no" },
        if s.rice_exists { "yes" } else { "no" }
    )
}

fn ensure_git_repo(manifest: &Manifest) -> Result<PathBuf> {
    let root = rice_dir(manifest);
    if !root.is_dir() {
        bail!("rice missing — run: macmgr rice init --git");
    }
    if which("git").is_none() {
        bail!("git not found");
    }
    if !root.join(".git").exists() {
        bail!(
            "rice at {} is not a git repo — run: macmgr rice init --git",
            root.display()
        );
    }
    Ok(root)
}

/// `git status --short` inside My rice.
pub fn git_status(manifest: &Manifest) -> Result<String> {
    let root = ensure_git_repo(manifest)?;
    let out = Command::new("git")
        .args(["status", "--short", "--branch"])
        .current_dir(&root)
        .output()
        .context("git status")?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Stage all + commit in My rice. Returns commit summary.
pub fn git_commit(manifest: &Manifest, message: Option<&str>) -> Result<String> {
    let root = ensure_git_repo(manifest)?;
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&root)
        .output()
        .context("git status")?;
    let dirty = String::from_utf8_lossy(&status.stdout);
    if dirty.trim().is_empty() {
        return Ok("nothing to commit (rice clean)".into());
    }

    let msg = message
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M");
            format!("macmgr: sync LIVE → rice ({stamp})")
        });

    let add = Command::new("git")
        .args(["add", "-A"])
        .current_dir(&root)
        .status()
        .context("git add")?;
    if !add.success() {
        bail!("git add failed");
    }

    let commit = Command::new("git")
        .args(["commit", "-m", &msg])
        .current_dir(&root)
        .output()
        .context("git commit")?;
    if !commit.status.success() {
        let err = String::from_utf8_lossy(&commit.stderr);
        bail!("git commit failed: {}", err.trim());
    }
    Ok(format!("committed in {} · {msg}", root.display()))
}

/// `git push` My rice to origin (needs remote).
pub fn git_push(manifest: &Manifest) -> Result<String> {
    let root = ensure_git_repo(manifest)?;
    let remotes = Command::new("git")
        .args(["remote"])
        .current_dir(&root)
        .output()
        .context("git remote")?;
    let remotes = String::from_utf8_lossy(&remotes.stdout);
    if remotes.trim().is_empty() {
        bail!(
            "no git remote — cd {} && git remote add origin <url>",
            root.display()
        );
    }
    let status = Command::new("git")
        .args(["push", "-u", "origin", "HEAD"])
        .current_dir(&root)
        .status()
        .context("git push")?;
    if !status.success() {
        bail!("git push failed (check remote/auth)");
    }
    Ok(format!("pushed {}", root.display()))
}
