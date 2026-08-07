use std::fs;
use std::path::PathBuf;

use crate::core::paths::home_dir;

#[derive(Debug, Clone)]
pub struct LogSource {
    pub id: String,
    pub label: String,
    pub path: PathBuf,
}

pub fn sources() -> Vec<LogSource> {
    let user = home_dir()
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("user")
        .to_string();
    let tmp = PathBuf::from("/tmp");
    vec![
        LogSource {
            id: "yabai-err".into(),
            label: "yabai stderr".into(),
            path: tmp.join(format!("yabai_{user}.err.log")),
        },
        LogSource {
            id: "yabai-out".into(),
            label: "yabai stdout".into(),
            path: tmp.join(format!("yabai_{user}.out.log")),
        },
        LogSource {
            id: "skhd-err".into(),
            label: "skhd stderr".into(),
            path: tmp.join(format!("skhd_{user}.err.log")),
        },
        LogSource {
            id: "skhd-out".into(),
            label: "skhd stdout".into(),
            path: tmp.join(format!("skhd_{user}.out.log")),
        },
        LogSource {
            id: "sketchybar".into(),
            label: "sketchybar (brew)".into(),
            path: home_dir().join("Library/Logs/Homebrew/sketchybar"),
        },
    ]
}

pub fn read_tail(path: &PathBuf, max_lines: usize) -> String {
    if !path.exists() {
        return format!("(missing) {}", path.display());
    }
    if path.is_dir() {
        return format!("(directory) {}", path.display());
    }
    match fs::read_to_string(path) {
        Ok(raw) => {
            let lines: Vec<_> = raw.lines().collect();
            let start = lines.len().saturating_sub(max_lines);
            lines[start..].join("\n")
        }
        Err(e) => format!("error reading {}: {e}", path.display()),
    }
}

pub fn clear(path: &PathBuf) -> std::io::Result<()> {
    if path.is_file() {
        fs::write(path, "")?;
    }
    Ok(())
}
