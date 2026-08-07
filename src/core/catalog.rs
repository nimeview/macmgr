use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::core::manifest::Manifest;
use crate::core::paths::{config_dir, ensure_dir, home_dir, which};
use crate::core::plugins::{self, PluginKind};
use crate::util::shell::run_status;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub label: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub description: String,
    /// `builtin:name` | raw https://…lua | git URL | github.com/… repo
    #[serde(default)]
    pub source: String,
    /// Subpath inside a git repo (e.g. `items/widgets/battery.lua`)
    #[serde(default)]
    pub path: String,
}

fn default_kind() -> String {
    "widget".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogOrigin {
    Local,
    Community,
}

#[derive(Debug, Clone)]
pub struct CatalogItem {
    pub origin: CatalogOrigin,
    pub entry: CatalogEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CatalogFile {
    #[serde(default)]
    pub plugins: Vec<CatalogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedDef {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct FeedsFile {
    #[serde(default)]
    pub feeds: Vec<FeedDef>,
}

#[derive(Debug, Clone)]
pub struct PullReport {
    pub feeds: usize,
    pub plugins: usize,
    pub path: PathBuf,
}

const BUILTIN_COMMUNITY: &str = include_str!("../../feeds/community.toml");

fn catalog_path() -> PathBuf {
    config_dir().join("catalog.toml")
}

fn feeds_path() -> PathBuf {
    config_dir().join("feeds.toml")
}

fn community_cache_path() -> PathBuf {
    home_dir().join(".local/share/macmgr/community-catalog.toml")
}

pub fn ensure_defaults() -> Result<()> {
    ensure_dir(&config_dir())?;
    let path = catalog_path();
    if path.exists() {
        return Ok(());
    }
    let cat = CatalogFile {
        plugins: vec![
            CatalogEntry {
                id: "clock".into(),
                label: "Simple clock".into(),
                kind: "widget".into(),
                description: "Shows HH:MM, updates every minute".into(),
                source: "builtin:clock".into(),
                path: String::new(),
            },
            CatalogEntry {
                id: "spacer".into(),
                label: "Spacer".into(),
                kind: "item".into(),
                description: "Empty padding item".into(),
                source: "builtin:spacer".into(),
                path: String::new(),
            },
            CatalogEntry {
                id: "uptime".into(),
                label: "Uptime".into(),
                kind: "widget".into(),
                description: "Host uptime via `uptime`".into(),
                source: "builtin:uptime".into(),
                path: String::new(),
            },
        ],
    };
    fs::write(&path, toml::to_string_pretty(&cat)?)?;
    Ok(())
}

fn load_file() -> Result<CatalogFile> {
    ensure_defaults()?;
    let raw = fs::read_to_string(catalog_path())?;
    Ok(toml::from_str(&raw)?)
}

fn save_file(cat: &CatalogFile) -> Result<()> {
    ensure_dir(&config_dir())?;
    fs::write(catalog_path(), toml::to_string_pretty(cat)?)?;
    Ok(())
}

pub fn list() -> Result<Vec<CatalogEntry>> {
    Ok(load_file()?.plugins)
}

/// Local catalog + community cache (local wins on duplicate ids).
pub fn list_all() -> Result<Vec<CatalogItem>> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for e in list()? {
        seen.insert(e.id.clone());
        out.push(CatalogItem {
            origin: CatalogOrigin::Local,
            entry: e,
        });
    }
    if let Ok(community) = load_community() {
        for e in community.plugins {
            if seen.insert(e.id.clone()) {
                out.push(CatalogItem {
                    origin: CatalogOrigin::Community,
                    entry: e,
                });
            }
        }
    }
    Ok(out)
}

pub fn add_entry(entry: CatalogEntry) -> Result<()> {
    let mut cat = load_file()?;
    if cat.plugins.iter().any(|e| e.id == entry.id) {
        bail!("catalog entry `{}` already exists", entry.id);
    }
    cat.plugins.push(entry);
    save_file(&cat)
}

fn load_community() -> Result<CatalogFile> {
    let path = community_cache_path();
    if !path.exists() {
        return Ok(CatalogFile::default());
    }
    let raw = fs::read_to_string(&path)?;
    Ok(toml::from_str(&raw)?)
}

fn save_community(cat: &CatalogFile) -> Result<()> {
    let path = community_cache_path();
    ensure_dir(path.parent().unwrap_or(Path::new("/")))?;
    fs::write(&path, toml::to_string_pretty(cat)?)?;
    Ok(())
}

pub fn list_feeds() -> Result<Vec<FeedDef>> {
    Ok(load_feeds()?.feeds)
}

fn load_feeds() -> Result<FeedsFile> {
    ensure_dir(&config_dir())?;
    let path = feeds_path();
    if !path.exists() {
        // Seed with builtin feed id pointing at embedded community index
        let seeded = FeedsFile {
            feeds: vec![FeedDef {
                id: "builtin".into(),
                url: "builtin:community".into(),
            }],
        };
        fs::write(&path, toml::to_string_pretty(&seeded)?)?;
        return Ok(seeded);
    }
    let raw = fs::read_to_string(&path)?;
    Ok(toml::from_str(&raw)?)
}

fn save_feeds(f: &FeedsFile) -> Result<()> {
    ensure_dir(&config_dir())?;
    fs::write(feeds_path(), toml::to_string_pretty(f)?)?;
    Ok(())
}

pub fn add_feed(id: &str, url: &str) -> Result<()> {
    let id = id.trim();
    let url = url.trim();
    if id.is_empty() || url.is_empty() {
        bail!("feed id and url required");
    }
    let mut f = load_feeds()?;
    if f.feeds.iter().any(|x| x.id == id) {
        bail!("feed `{id}` already exists");
    }
    f.feeds.push(FeedDef {
        id: id.to_string(),
        url: url.to_string(),
    });
    save_feeds(&f)
}

/// Fetch all configured feeds into the community cache.
pub fn pull_feeds() -> Result<PullReport> {
    let feeds = load_feeds()?.feeds;
    let mut merged = CatalogFile::default();
    let mut seen = std::collections::HashSet::new();

    for feed in &feeds {
        let body = fetch_feed_body(&feed.url)?;
        let cat: CatalogFile = toml::from_str(&body)
            .with_context(|| format!("parse feed `{}`", feed.id))?;
        for e in cat.plugins {
            if seen.insert(e.id.clone()) {
                merged.plugins.push(e);
            }
        }
    }

    let n = merged.plugins.len();
    let path = community_cache_path();
    save_community(&merged)?;
    Ok(PullReport {
        feeds: feeds.len(),
        plugins: n,
        path,
    })
}

fn fetch_feed_body(url: &str) -> Result<String> {
    let url = url.trim();
    if url == "builtin:community" || url == "builtin" {
        return Ok(BUILTIN_COMMUNITY.to_string());
    }
    if let Some(path) = url.strip_prefix("file:") {
        let p = expand_feed_path(path);
        return fs::read_to_string(&p)
            .with_context(|| format!("read feed file {}", p.display()));
    }
    if url.starts_with('/') || url.starts_with('~') {
        let p = expand_feed_path(url);
        return fs::read_to_string(&p)
            .with_context(|| format!("read feed file {}", p.display()));
    }
    if url.starts_with("http://") || url.starts_with("https://") {
        let tmp = std::env::temp_dir().join(format!(
            "macmgr-feed-{}",
            Local::now().format("%Y%m%d%H%M%S")
        ));
        run_status("curl", &["-fsSL", url, "-o", tmp.to_str().unwrap_or("")])
            .with_context(|| format!("download feed {url}"))?;
        let body = fs::read_to_string(&tmp)?;
        let _ = fs::remove_file(tmp);
        return Ok(body);
    }
    bail!("unsupported feed url: {url} (use https://, file:, ~/, or builtin:community)");
}

fn expand_feed_path(path: &str) -> PathBuf {
    crate::core::paths::expand_tilde(path)
}

fn kind_from(s: &str) -> PluginKind {
    if s == "item" {
        PluginKind::Item
    } else {
        PluginKind::Widget
    }
}

/// Install by catalog id (local first, then community cache).
pub fn install(manifest: &Manifest, id: &str, reload: bool) -> Result<PathBuf> {
    let entry = list_all()?
        .into_iter()
        .find(|i| i.entry.id == id)
        .map(|i| i.entry)
        .ok_or_else(|| anyhow!("catalog entry `{id}` not found — try: macmgr catalog pull"))?;
    install_entry(manifest, &entry, reload)
}

/// One-shot install from URL/git without a catalog row.
pub fn install_url(
    manifest: &Manifest,
    url: &str,
    id: &str,
    kind: &str,
    subpath: Option<&str>,
    reload: bool,
) -> Result<PathBuf> {
    let entry = CatalogEntry {
        id: id.to_string(),
        label: id.to_string(),
        kind: kind.to_string(),
        description: format!("installed from {url}"),
        source: url.to_string(),
        path: subpath.unwrap_or("").to_string(),
    };
    install_entry(manifest, &entry, reload)
}

fn install_entry(manifest: &Manifest, entry: &CatalogEntry, reload: bool) -> Result<PathBuf> {
    let kind = kind_from(&entry.kind);
    let source = entry.source.trim();

    if source.starts_with("builtin:") {
        let path = plugins::add(manifest, &entry.id, kind)?;
        let body = builtin_source(
            source.strip_prefix("builtin:").unwrap_or("clock"),
            &entry.id,
        );
        fs::write(&path, body)?;
        if reload {
            plugins::reload_bar(manifest)?;
        }
        return Ok(path);
    }

    let dest = plugins::add(manifest, &entry.id, kind)?;

    if is_raw_lua_url(source) {
        download_raw(source, &dest)?;
    } else if is_git_source(source) {
        let repo = normalize_git_url(source);
        let sub = if entry.path.is_empty() {
            None
        } else {
            Some(entry.path.as_str())
        };
        fetch_git_file(&repo, sub, &dest)?;
    } else {
        // treat as local path?
        let local = PathBuf::from(source);
        if local.is_file() {
            fs::copy(&local, &dest)
                .with_context(|| format!("copy {}", local.display()))?;
        } else {
            bail!("unsupported catalog source: {source}");
        }
    }

    if reload {
        plugins::reload_bar(manifest)?;
    }
    Ok(dest)
}

fn is_raw_lua_url(source: &str) -> bool {
    let lower = source.to_lowercase();
    (source.starts_with("http://") || source.starts_with("https://"))
        && (lower.ends_with(".lua") || lower.contains("raw.githubusercontent.com"))
}

fn is_git_source(source: &str) -> bool {
    let s = source.trim();
    s.starts_with("git:")
        || s.starts_with("git+")
        || s.ends_with(".git")
        || s.contains("github.com/")
        || s.contains("gitlab.com/")
        || s.contains("bitbucket.org/")
        || s.starts_with("ssh://")
        || s.starts_with("git@")
}

fn normalize_git_url(source: &str) -> String {
    let s = source
        .trim()
        .strip_prefix("git+")
        .or_else(|| source.trim().strip_prefix("git:"))
        .unwrap_or(source.trim())
        .to_string();
    // github.com/user/repo[/tree/branch/path] → https://github.com/user/repo.git
    if let Some(rest) = s
        .strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("http://github.com/"))
    {
        let parts: Vec<_> = rest.trim_matches('/').split('/').collect();
        if parts.len() >= 2 && !s.ends_with(".git") {
            return format!("https://github.com/{}/{}.git", parts[0], parts[1]);
        }
    }
    if s.contains("github.com/") && !s.ends_with(".git") && !s.contains("raw.githubusercontent") {
        return format!("{s}.git");
    }
    s
}

fn download_raw(url: &str, dest: &Path) -> Result<()> {
    let tmp = std::env::temp_dir().join(format!(
        "macmgr-dl-{}",
        Local::now().format("%Y%m%d%H%M%S")
    ));
    run_status("curl", &["-fsSL", url, "-o", tmp.to_str().unwrap_or("")])
        .with_context(|| format!("download {url}"))?;
    fs::copy(&tmp, dest)?;
    let _ = fs::remove_file(tmp);
    Ok(())
}

fn fetch_git_file(repo: &str, subpath: Option<&str>, dest: &Path) -> Result<()> {
    if which("git").is_none() {
        bail!("git not found — needed for catalog git installs");
    }
    let tmp = home_dir().join(format!(
        ".local/share/macmgr/catalog-clone-{}",
        Local::now().format("%Y%m%d%H%M%S")
    ));
    if tmp.exists() {
        let _ = fs::remove_dir_all(&tmp);
    }
    ensure_dir(tmp.parent().unwrap_or(Path::new("/")))?;

    let status = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "--quiet",
            repo,
            tmp.to_str().unwrap_or(""),
        ])
        .status()
        .context("git clone")?;
    if !status.success() {
        let _ = fs::remove_dir_all(&tmp);
        bail!("git clone failed: {repo}");
    }

    let result = (|| {
        let src = resolve_lua_in_clone(&tmp, subpath)?;
        fs::copy(&src, dest)
            .with_context(|| format!("copy {} → {}", src.display(), dest.display()))?;
        Ok(())
    })();

    let _ = fs::remove_dir_all(&tmp);
    result
}

fn resolve_lua_in_clone(root: &Path, subpath: Option<&str>) -> Result<PathBuf> {
    if let Some(p) = subpath.map(str::trim).filter(|s| !s.is_empty()) {
        let candidate = root.join(p);
        if candidate.is_file() {
            return Ok(candidate);
        }
        bail!("path `{p}` not found in repo");
    }

    // Prefer common sketchybar layouts
    let guesses = [
        "init.lua",
        "item.lua",
        "widget.lua",
        "plugin.lua",
    ];
    for g in guesses {
        let c = root.join(g);
        if c.is_file() {
            return Ok(c);
        }
    }

    let mut luas: Vec<_> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("lua"))
        .filter(|p| !p.to_string_lossy().contains("/.git/"))
        .collect();
    luas.sort();
    luas.into_iter()
        .next()
        .ok_or_else(|| anyhow!("no .lua file found in repo — pass --path"))
}

fn builtin_source(name: &str, id: &str) -> String {
    match name {
        "spacer" => format!(
            r#"-- catalog: spacer
local settings = require("settings")
sbar.add("item", "{id}", {{
  position = "right",
  width = settings.group_paddings,
  icon = {{ drawing = false }},
  label = {{ drawing = false }},
}})
"#
        ),
        "uptime" => format!(
            r#"-- catalog: uptime
local settings = require("settings")
local item = sbar.add("item", "widgets.{id}", {{
  position = "right",
  icon = {{ string = "􀐫", padding_left = 6 }},
  label = {{ string = "…", font = {{ family = settings.font.numbers }} }},
  update_freq = 120,
}})
item:subscribe("routine", function()
  sbar.exec("uptime | sed 's/.*up *//;s/,.*//'", function(out)
    item:set({{ label = {{ string = out }} }})
  end)
end)
"#
        ),
        _ => format!(
            r#"-- catalog: clock
local settings = require("settings")
local clock = sbar.add("item", "widgets.{id}", {{
  position = "right",
  icon = {{ string = "􀐫", padding_left = 8 }},
  label = {{ font = {{ family = settings.font.numbers }} }},
  update_freq = 30,
}})
clock:subscribe({{"routine", "system_woke"}}, function()
  clock:set({{ label = {{ string = os.date("%H:%M") }} }})
end)
"#
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_sources() {
        assert!(is_raw_lua_url(
            "https://raw.githubusercontent.com/u/r/main/x.lua"
        ));
        assert!(is_git_source("https://github.com/u/r.git"));
        assert!(is_git_source("git:https://github.com/u/r"));
        assert!(!is_git_source("builtin:clock"));
    }

    #[test]
    fn normalize_github() {
        assert_eq!(
            normalize_git_url("https://github.com/user/repo"),
            "https://github.com/user/repo.git"
        );
        assert_eq!(
            normalize_git_url("git:https://github.com/user/repo.git"),
            "https://github.com/user/repo.git"
        );
    }

    #[test]
    fn builtin_feed_parses() {
        let cat: CatalogFile = toml::from_str(BUILTIN_COMMUNITY).expect("community.toml");
        assert!(cat.plugins.len() >= 3);
        assert!(cat.plugins.iter().any(|p| p.id == "clock"));
    }
}
