use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};

use crate::core::manifest::Manifest;
use crate::core::paths::expand_tilde;
use crate::core::service;
use crate::util::editor::edit_path;

const DISABLED_MARK: &str = "-- macmgr:off ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginKind {
    Item,
    Widget,
}

impl PluginKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PluginKind::Item => "item",
            PluginKind::Widget => "widget",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Plugin {
    pub id: String,
    pub kind: PluginKind,
    pub path: PathBuf,
    pub enabled: bool,
    pub require_mod: String,
}

fn sketchybar_root(manifest: &Manifest) -> Result<PathBuf> {
    let cfg = manifest
        .find_config("sketchybar")
        .ok_or_else(|| anyhow!("sketchybar config missing from manifest"))?;
    let path = expand_tilde(&cfg.path);
    if !path.is_dir() {
        return Err(anyhow!(
            "LIVE SketchyBar dir missing: {} (your real config lives here)",
            path.display()
        ));
    }
    Ok(path)
}

fn items_init(root: &Path) -> PathBuf {
    root.join("items/init.lua")
}

fn widgets_init(root: &Path) -> PathBuf {
    root.join("items/widgets/init.lua")
}

fn parse_requires(init_path: &Path) -> Result<Vec<(bool, String)>> {
    if !init_path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(init_path)?;
    let mut out = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        let (enabled, rest) = if let Some(rest) = trimmed.strip_prefix(DISABLED_MARK) {
            (false, rest.trim())
        } else if trimmed.starts_with("--") {
            continue;
        } else {
            (true, trimmed)
        };
        if let Some(modname) = extract_require(rest) {
            out.push((enabled, modname));
        }
    }
    Ok(out)
}

fn extract_require(line: &str) -> Option<String> {
    let line = line.trim().trim_end_matches(';');
    let start = line.find("require(")?;
    let after = &line[start + "require(".len()..];
    let quote = after.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &after[1..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

pub fn list(manifest: &Manifest) -> Result<Vec<Plugin>> {
    let root = sketchybar_root(manifest)?;
    let mut plugins = Vec::new();

    let item_reqs = parse_requires(&items_init(&root))?;
    let items_dir = root.join("items");
    if items_dir.is_dir() {
        for entry in fs::read_dir(&items_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.file_name().and_then(|n| n.to_str()) == Some("init.lua") {
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("lua") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let require_mod = format!("items.{stem}");
            let enabled = item_reqs
                .iter()
                .find(|(_, m)| m == &require_mod)
                .map(|(e, _)| *e)
                .unwrap_or(false);
            plugins.push(Plugin {
                id: stem,
                kind: PluginKind::Item,
                path,
                enabled,
                require_mod,
            });
        }
    }

    let widget_reqs = parse_requires(&widgets_init(&root))?;
    let widgets_dir = root.join("items/widgets");
    if widgets_dir.is_dir() {
        for entry in fs::read_dir(&widgets_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.file_name().and_then(|n| n.to_str()) == Some("init.lua") {
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("lua") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let require_mod = format!("items.widgets.{stem}");
            let enabled = widget_reqs
                .iter()
                .find(|(_, m)| m == &require_mod)
                .map(|(e, _)| *e)
                .unwrap_or(false);
            // parent items.widgets must be enabled for widgets to load
            plugins.push(Plugin {
                id: format!("widgets.{stem}"),
                kind: PluginKind::Widget,
                path,
                enabled,
                require_mod,
            });
        }
    }

    plugins.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(plugins)
}

fn set_require_enabled(init_path: &Path, module: &str, enabled: bool) -> Result<()> {
    let mut lines: Vec<String> = if init_path.exists() {
        fs::read_to_string(init_path)?
            .lines()
            .map(|l| l.to_string())
            .collect()
    } else {
        Vec::new()
    };

    let require_line = format!("require(\"{module}\")");
    let mut found = false;
    for line in &mut lines {
        let trimmed = line.trim();
        let candidate = if let Some(rest) = trimmed.strip_prefix(DISABLED_MARK) {
            rest.trim()
        } else {
            trimmed
        };
        if extract_require(candidate).as_deref() == Some(module) {
            *line = if enabled {
                require_line.clone()
            } else {
                format!("{DISABLED_MARK}{require_line}")
            };
            found = true;
            break;
        }
    }
    if !found {
        if enabled {
            lines.push(require_line);
        } else {
            lines.push(format!("{DISABLED_MARK}{require_line}"));
        }
    }

    if let Some(parent) = init_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut body = lines.join("\n");
    if !body.ends_with('\n') {
        body.push('\n');
    }
    // backup
    if init_path.exists() {
        let bak = init_path.with_extension("lua.macmgr.bak");
        let _ = fs::copy(init_path, bak);
    }
    fs::write(init_path, body).with_context(|| format!("writing {}", init_path.display()))?;
    Ok(())
}

pub fn enable(manifest: &Manifest, id: &str) -> Result<()> {
    let root = sketchybar_root(manifest)?;
    let plugin = find_plugin(manifest, id)?;
    match plugin.kind {
        PluginKind::Item => set_require_enabled(&items_init(&root), &plugin.require_mod, true)?,
        PluginKind::Widget => {
            // ensure widgets aggregator is on
            set_require_enabled(&items_init(&root), "items.widgets", true)?;
            set_require_enabled(&widgets_init(&root), &plugin.require_mod, true)?;
        }
    }
    Ok(())
}

pub fn disable(manifest: &Manifest, id: &str) -> Result<()> {
    let root = sketchybar_root(manifest)?;
    let plugin = find_plugin(manifest, id)?;
    match plugin.kind {
        PluginKind::Item => set_require_enabled(&items_init(&root), &plugin.require_mod, false)?,
        PluginKind::Widget => {
            set_require_enabled(&widgets_init(&root), &plugin.require_mod, false)?
        }
    }
    Ok(())
}

pub fn toggle(manifest: &Manifest, id: &str) -> Result<bool> {
    let plugin = find_plugin(manifest, id)?;
    if plugin.enabled {
        disable(manifest, id)?;
        Ok(false)
    } else {
        enable(manifest, id)?;
        Ok(true)
    }
}

fn find_plugin(manifest: &Manifest, id: &str) -> Result<Plugin> {
    list(manifest)?
        .into_iter()
        .find(|p| p.id == id || p.id.strip_prefix("widgets.") == Some(id))
        .ok_or_else(|| anyhow!("plugin `{id}` not found (macmgr plugin list)"))
}

pub fn add(manifest: &Manifest, name: &str, kind: PluginKind) -> Result<PathBuf> {
    let name = sanitize_name(name)?;
    let root = sketchybar_root(manifest)?;

    let (path, require_mod, init) = match kind {
        PluginKind::Item => {
            let path = root.join("items").join(format!("{name}.lua"));
            (path, format!("items.{name}"), items_init(&root))
        }
        PluginKind::Widget => {
            let path = root.join("items/widgets").join(format!("{name}.lua"));
            (path, format!("items.widgets.{name}"), widgets_init(&root))
        }
    };

    if path.exists() {
        return Err(anyhow!("already exists: {}", path.display()));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, stub_source(kind, &name))?;
    set_require_enabled(&init, &require_mod, true)?;
    if kind == PluginKind::Widget {
        set_require_enabled(&items_init(&root), "items.widgets", true)?;
    }
    Ok(path)
}

pub fn remove(manifest: &Manifest, id: &str, delete_file: bool) -> Result<()> {
    let root = sketchybar_root(manifest)?;
    let plugin = find_plugin(manifest, id)?;
    match plugin.kind {
        PluginKind::Item => {
            if delete_file {
                remove_require_line(&items_init(&root), &plugin.require_mod)?;
            } else {
                set_require_enabled(&items_init(&root), &plugin.require_mod, false)?;
            }
        }
        PluginKind::Widget => {
            if delete_file {
                remove_require_line(&widgets_init(&root), &plugin.require_mod)?;
            } else {
                set_require_enabled(&widgets_init(&root), &plugin.require_mod, false)?;
            }
        }
    }
    if delete_file && plugin.path.exists() {
        fs::remove_file(&plugin.path)?;
    }
    Ok(())
}

fn remove_require_line(init_path: &Path, module: &str) -> Result<()> {
    if !init_path.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(init_path)?;
    let lines: Vec<String> = raw
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            let candidate = trimmed
                .strip_prefix(DISABLED_MARK)
                .map(|s| s.trim())
                .unwrap_or(trimmed);
            extract_require(candidate).as_deref() != Some(module)
        })
        .map(|l| l.to_string())
        .collect();
    let mut body = lines.join("\n");
    if !body.ends_with('\n') {
        body.push('\n');
    }
    fs::write(init_path, body)?;
    Ok(())
}

pub fn edit(manifest: &Manifest, id: &str) -> Result<()> {
    let plugin = find_plugin(manifest, id)?;
    edit_path(&plugin.path)
}

pub fn reload_bar(manifest: &Manifest) -> Result<()> {
    reload_bar_inner(manifest, None, true)
}

/// Reload after editing `changed` — validates that file first, then whole tree.
pub fn reload_bar_after(manifest: &Manifest, changed: &Path) -> Result<()> {
    reload_bar_inner(manifest, Some(changed), true)
}

/// Fast path for watch-mode (no undo snapshot).
pub fn reload_bar_fast(manifest: &Manifest) -> Result<()> {
    reload_bar_inner(manifest, None, false)
}

fn reload_bar_inner(manifest: &Manifest, changed: Option<&Path>, safe: bool) -> Result<()> {
    crate::core::validate::before_reload(manifest, changed)?;

    if safe {
        let _ = crate::core::snapshot::push_undo(manifest, "before-bar-reload");
    }

    do_restart_bar(manifest)?;

    if safe && !wait_for_sketchybar(2500) {
        // Restore previous LIVE configs and try again
        match crate::core::snapshot::undo(manifest) {
            Ok(info) => {
                let _ = do_restart_bar(manifest);
                let _ = wait_for_sketchybar(2000);
                bail!(
                    "safe reload: sketchybar did not stay up — restored undo `{}`",
                    info.meta.id
                );
            }
            Err(e) => {
                bail!("safe reload: sketchybar down and undo failed: {e}");
            }
        }
    }
    Ok(())
}

fn do_restart_bar(manifest: &Manifest) -> Result<()> {
    if let Some(svc) = manifest.find_service("sketchybar") {
        service::restart(svc)?;
    } else {
        let _ = crate::util::shell::run_status("sketchybar", &["--reload"]);
    }
    Ok(())
}

fn wait_for_sketchybar(timeout_ms: u64) -> bool {
    use std::thread;
    use std::time::Duration;
    let steps = (timeout_ms / 100).max(1);
    for _ in 0..steps {
        if service::is_process_running("sketchybar") {
            return true;
        }
        thread::sleep(Duration::from_millis(100));
    }
    service::is_process_running("sketchybar")
}

fn sanitize_name(name: &str) -> Result<String> {
    let name = name.trim().to_lowercase().replace(' ', "_");
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(anyhow!(
            "invalid plugin name `{name}` (use letters, digits, _)"
        ));
    }
    if name == "init" || name == "widgets" {
        return Err(anyhow!("reserved name `{name}`"));
    }
    Ok(name)
}

fn stub_source(kind: PluginKind, name: &str) -> String {
    match kind {
        PluginKind::Item => format!(
            r#"-- macmgr plugin: {name} (item)
-- LIVE file under ~/.config/sketchybar/items/{name}.lua
local colors = require("colors")
local settings = require("settings")

local item = sbar.add("item", "{name}", {{
  position = "right",
  icon = {{ string = "􀆊", padding_left = 8, padding_right = 4 }},
  label = {{ string = "{name}", padding_right = 8 }},
}})

item:subscribe("mouse.clicked", function()
  sbar.exec("open -a Finder")
end)
"#
        ),
        PluginKind::Widget => format!(
            r#"-- macmgr plugin: {name} (widget)
-- LIVE file under ~/.config/sketchybar/items/widgets/{name}.lua
local colors = require("colors")
local settings = require("settings")

local widget = sbar.add("item", "widgets.{name}", {{
  position = "right",
  icon = {{ string = "􀄫", font = {{ size = 14.0 }} }},
  label = {{
    string = "{name}",
    font = {{ family = settings.font.numbers }},
  }},
  update_freq = 60,
}})

widget:subscribe("routine", function()
  widget:set({{ label = {{ string = os.date("%H:%M") }} }})
end)
"#
        ),
    }
}

pub fn format_table(plugins: &[Plugin]) -> String {
    let mut out = String::from("SketchyBar plugins (LIVE ~/.config/sketchybar)\n");
    out.push_str(&"─".repeat(72));
    out.push('\n');
    for p in plugins {
        let mark = if p.enabled { "ON " } else { "off" };
        out.push_str(&format!(
            "  {mark}  {:<8} {:<22} {}\n",
            p.kind.as_str(),
            p.id,
            p.path.display()
        ));
    }
    out.push_str(&format!(
        "\n{} plugins · enable/disable edits items/init.lua (and widgets/init.lua)\n",
        plugins.len()
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_require_double_and_single_quotes() {
        assert_eq!(
            extract_require(r#"require("items.widgets.battery")"#).as_deref(),
            Some("items.widgets.battery")
        );
        assert_eq!(
            extract_require("require('items.foo');").as_deref(),
            Some("items.foo")
        );
        assert_eq!(extract_require("-- comment"), None);
    }

    #[test]
    fn sanitize_plugin_name() {
        assert_eq!(sanitize_name("My Stats").unwrap(), "my_stats");
        assert!(sanitize_name("init").is_err());
        assert!(sanitize_name("bad-name!").is_err());
    }
}
