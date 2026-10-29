use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};

use crate::core::manifest::Manifest;
use crate::core::paths::expand_tilde;

const OFF: &str = "# macmgr:off ";

#[derive(Debug, Clone)]
pub struct YabaiRule {
    pub index: usize,
    pub enabled: bool,
    pub raw: String,
    pub summary: String,
}

fn yabairc(manifest: &Manifest) -> Result<PathBuf> {
    let cfg = manifest
        .find_config("yabai")
        .ok_or_else(|| anyhow!("yabai config missing"))?;
    let path = expand_tilde(&cfg.path);
    if !path.exists() {
        return Err(anyhow!("yabairc missing: {}", path.display()));
    }
    Ok(path)
}

pub fn list(manifest: &Manifest) -> Result<Vec<YabaiRule>> {
    let path = yabairc(manifest)?;
    let raw = fs::read_to_string(&path)?;
    let mut out = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        let (enabled, body) = if let Some(rest) = trimmed.strip_prefix(OFF) {
            (false, rest.trim())
        } else if trimmed.starts_with('#') {
            continue;
        } else {
            (true, trimmed)
        };
        if body.contains("yabai -m rule") || body.contains("yabai -m rule --add") {
            let summary = summarize(body);
            out.push(YabaiRule {
                index: i,
                enabled,
                raw: body.to_string(),
                summary,
            });
        }
    }
    Ok(out)
}

fn summarize(line: &str) -> String {
    // pull app="..." if present
    if let Some(start) = line.find("app=") {
        let rest = &line[start + 4..];
        let quote = rest.chars().next().unwrap_or('"');
        if let Some(end) = rest[1..].find(quote) {
            return format!("app={}", &rest[..=end + 1]);
        }
    }
    crate::util::text::truncate_chars(line, 60)
}

pub fn set_enabled(manifest: &Manifest, index: usize, enabled: bool) -> Result<()> {
    let path = yabairc(manifest)?;
    let raw = fs::read_to_string(&path)?;
    let mut lines: Vec<String> = raw.lines().map(|l| l.to_string()).collect();
    let line = lines
        .get_mut(index)
        .ok_or_else(|| anyhow!("line {index} out of range"))?;
    let trimmed = line.trim().to_string();
    let body = trimmed
        .strip_prefix(OFF)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| {
            if trimmed.starts_with('#') {
                trimmed.trim_start_matches('#').trim().to_string()
            } else {
                trimmed
            }
        });
    if !body.contains("yabai -m rule") {
        return Err(anyhow!("line {index} is not a yabai rule"));
    }
    *line = if enabled {
        body
    } else {
        format!("{OFF}{body}")
    };
    // backup
    let bak = path.with_extension("yabairc.macmgr.bak");
    let _ = fs::copy(&path, bak);
    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    fs::write(&path, out)?;
    Ok(())
}

pub fn toggle(manifest: &Manifest, index: usize) -> Result<bool> {
    let rules = list(manifest)?;
    let rule = rules
        .iter()
        .find(|r| r.index == index)
        .ok_or_else(|| anyhow!("no rule at line {index}"))?;
    let next = !rule.enabled;
    set_enabled(manifest, index, next)?;
    Ok(next)
}

pub fn add(manifest: &Manifest, app_pattern: &str, manage_off: bool) -> Result<()> {
    let path = yabairc(manifest)?;
    let mut raw = fs::read_to_string(&path)?;
    if !raw.ends_with('\n') {
        raw.push('\n');
    }
    let line = if manage_off {
        format!("yabai -m rule --add app=\"^{app_pattern}$\" manage=off\n")
    } else {
        format!("yabai -m rule --add app=\"^{app_pattern}$\"\n")
    };
    raw.push_str(&line);
    fs::write(&path, raw)?;
    Ok(())
}

pub fn format_table(rules: &[YabaiRule]) -> String {
    let mut out = String::from("yabai rules (LIVE yabairc)\n");
    out.push_str(&"─".repeat(72));
    out.push('\n');
    for r in rules {
        let mark = if r.enabled { "ON " } else { "off" };
        out.push_str(&format!("  {mark}  L{:<4} {}\n", r.index + 1, r.summary));
    }
    out.push_str(&format!("\n{} rules\n", rules.len()));
    out
}
