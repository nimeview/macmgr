use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use anyhow::{anyhow, Result};

use crate::core::manifest::Manifest;
use crate::core::paths::expand_tilde;
use crate::core::plugins;

/// Poll LIVE sketchybar dir; reload bar when any tracked file mtime changes.
pub struct WatchHandle {
    stop: Arc<Mutex<bool>>,
}

impl WatchHandle {
    pub fn stop(&self) {
        if let Ok(mut g) = self.stop.lock() {
            *g = true;
        }
    }
}

pub fn start(manifest: &Manifest, interval_ms: u64) -> Result<WatchHandle> {
    let cfg = manifest
        .find_config("sketchybar")
        .ok_or_else(|| anyhow!("no sketchybar config"))?;
    let root = expand_tilde(&cfg.path);
    if !root.is_dir() {
        return Err(anyhow!("sketchybar LIVE dir missing"));
    }
    let stop = Arc::new(Mutex::new(false));
    let stop_t = Arc::clone(&stop);
    let root_t = root.clone();
    let manifest = manifest.clone();

    thread::spawn(move || {
        let mut last: HashMap<PathBuf, SystemTime> = HashMap::new();
        scan_mtimes(&root_t, &mut last);
        loop {
            if stop_t.lock().map(|g| *g).unwrap_or(true) {
                break;
            }
            thread::sleep(Duration::from_millis(interval_ms.max(300)));
            let mut now = HashMap::new();
            scan_mtimes(&root_t, &mut now);
            let changed = now.iter().any(|(p, t)| last.get(p) != Some(t))
                || last.keys().any(|p| !now.contains_key(p));
            if changed {
                match plugins::reload_bar_fast(&manifest) {
                    Ok(()) => eprintln!("macmgr watch: validated + reloaded"),
                    Err(e) => eprintln!("macmgr watch: reload blocked:\n{e}"),
                }
                last = now;
            }
        }
    });

    Ok(WatchHandle { stop })
}

fn scan_mtimes(root: &Path, into: &mut HashMap<PathBuf, SystemTime>) {
    let walker = walkdir::WalkDir::new(root).into_iter().filter_map(|e| e.ok());
    for entry in walker {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("lua")
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with("rc"))
                != Some(true)
        {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if let Ok(mtime) = meta.modified() {
                into.insert(path.to_path_buf(), mtime);
            }
        }
    }
}
