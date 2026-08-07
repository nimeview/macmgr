use std::path::{Path, PathBuf};

use directories::{BaseDirs, ProjectDirs};

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(base) = BaseDirs::new() {
            return base.home_dir().join(rest);
        }
    }
    if path == "~" {
        if let Some(base) = BaseDirs::new() {
            return base.home_dir().to_path_buf();
        }
    }
    PathBuf::from(path)
}

pub fn home_dir() -> PathBuf {
    BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

pub fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("dev", "macmgr", "macmgr")
}

pub fn config_dir() -> PathBuf {
    project_dirs()
        .map(|p| p.config_dir().to_path_buf())
        .unwrap_or_else(|| home_dir().join(".config/macmgr"))
}

pub fn launch_agents_dir() -> PathBuf {
    home_dir().join("Library/LaunchAgents")
}

pub fn which(bin: &str) -> Option<PathBuf> {
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let candidate = Path::new(dir).join(bin);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn ensure_dir(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}
