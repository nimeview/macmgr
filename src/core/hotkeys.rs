use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};

use crate::core::manifest::Manifest;
use crate::core::paths::expand_tilde;

const OFF: &str = "# macmgr:off ";

#[derive(Debug, Clone)]
pub struct Hotkey {
    /// 0-based start line index in skhdrc
    pub index: usize,
    /// Inclusive end line (same as index for single-line)
    pub end_index: usize,
    pub enabled: bool,
    pub keys: String,
    pub action: String,
}

impl Hotkey {
    pub fn is_multiline(&self) -> bool {
        self.end_index > self.index || self.action.contains('\n')
    }
}

fn skhdrc(manifest: &Manifest) -> Result<PathBuf> {
    let cfg = manifest
        .find_config("skhd")
        .ok_or_else(|| anyhow!("skhd config not in manifest"))?;
    let path = expand_tilde(&cfg.path);
    if !path.exists() {
        return Err(anyhow!("skhdrc missing: {}", path.display()));
    }
    Ok(path)
}

fn backup(path: &PathBuf) {
    let bak = path.with_extension("skhdrc.macmgr.bak");
    let _ = fs::copy(path, bak);
}

fn write_lines(path: &PathBuf, lines: &[String]) -> Result<()> {
    backup(path);
    let mut body = lines.join("\n");
    if !body.ends_with('\n') {
        body.push('\n');
    }
    fs::write(path, body)?;
    Ok(())
}

fn parse_binding_line(trimmed: &str) -> Option<(String, String)> {
    if let Some((keys, rest)) = trimmed.split_once(':') {
        let keys = keys.trim().to_string();
        let action = rest.trim().to_string();
        if !keys.is_empty() && !action.is_empty() {
            return Some((keys, action));
        }
    }
    if let Some((keys, rest)) = trimmed.split_once(';') {
        let keys = keys.trim().to_string();
        if !keys.is_empty() {
            return Some((keys, format!("; {}", rest.trim())));
        }
    }
    None
}

/// Normalize action: strip trailing `\` join markers used in file form.
fn join_continued_action(parts: &[String]) -> String {
    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        let mut p = part.trim_end().to_string();
        if p.ends_with('\\') {
            p.pop();
            p = p.trim_end().to_string();
        }
        if i > 0 {
            out.push('\n');
            out.push_str(p.trim_start());
        } else {
            out.push_str(&p);
        }
    }
    out
}

/// Write action as skhd backslash-continued lines.
pub fn format_binding_lines(keys: &str, action: &str, enabled: bool) -> Vec<String> {
    let keys = keys.trim();
    let action = action.trim();
    let parts: Vec<&str> = if action.contains('\n') {
        action.lines().map(str::trim).filter(|l| !l.is_empty()).collect()
    } else {
        vec![action]
    };
    if parts.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        let body = if i == 0 {
            if last {
                format!("{keys} : {part}")
            } else {
                format!("{keys} : {part} \\")
            }
        } else if last {
            format!("    {part}")
        } else {
            format!("    {part} \\")
        };
        lines.push(if enabled {
            body
        } else {
            format!("{OFF}{body}")
        });
    }
    lines
}

pub fn list(manifest: &Manifest) -> Result<Vec<Hotkey>> {
    let path = skhdrc(manifest)?;
    let raw = fs::read_to_string(&path)?;
    Ok(parse_lines(&raw))
}

pub fn parse_lines(raw: &str) -> Vec<Hotkey> {
    let lines: Vec<&str> = raw.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        let (enabled, body) = if let Some(rest) = trimmed.strip_prefix(OFF) {
            (false, rest.trim())
        } else if trimmed.starts_with('#') {
            i += 1;
            continue;
        } else {
            (true, trimmed)
        };

        if let Some((keys, first_action)) = parse_binding_line(body) {
            let start = i;
            let mut action_parts = vec![first_action];
            let mut end = i;
            // Consume backslash continuations
            while end < lines.len() {
                let cur = action_parts.last().map(|s| s.trim_end()).unwrap_or("");
                if !cur.ends_with('\\') {
                    break;
                }
                if end + 1 >= lines.len() {
                    break;
                }
                end += 1;
                let next = lines[end];
                let next_trim = next.trim();
                // Allow OFF-prefixed continuation when whole binding disabled
                let cont = next_trim
                    .strip_prefix(OFF)
                    .map(str::trim)
                    .unwrap_or(next_trim);
                if cont.starts_with('#') {
                    break;
                }
                action_parts.push(cont.to_string());
            }
            let action = join_continued_action(&action_parts);
            out.push(Hotkey {
                index: start,
                end_index: end,
                enabled,
                keys,
                action,
            });
            i = end + 1;
        } else {
            i += 1;
        }
    }
    out
}

/// Compatibility: old callers expecting keys/action only.
#[allow(dead_code)]
pub fn parse_skhd(manifest: &Manifest) -> Result<Vec<Hotkey>> {
    list(manifest)
}

#[allow(dead_code)]
pub fn parse_skhd_text(raw: &str) -> Vec<Hotkey> {
    parse_lines(raw)
}

pub fn toggle(manifest: &Manifest, line_1based: usize) -> Result<bool> {
    let path = skhdrc(manifest)?;
    let raw = fs::read_to_string(&path)?;
    let mut lines: Vec<String> = raw.lines().map(|l| l.to_string()).collect();
    let idx = line_1based
        .checked_sub(1)
        .ok_or_else(|| anyhow!("line must be >= 1"))?;
    let items = parse_lines(&raw);
    let hk = items
        .get(idx)
        .ok_or_else(|| anyhow!("hotkey #{line_1based} not found"))?;
    let was_enabled = hk.enabled;
    for line_idx in hk.index..=hk.end_index {
        let line = lines
            .get_mut(line_idx)
            .ok_or_else(|| anyhow!("line {} missing", line_idx + 1))?;
        let trimmed = line.trim().to_string();
        if was_enabled {
            if !trimmed.starts_with(OFF) && !trimmed.starts_with('#') {
                *line = format!("{OFF}{trimmed}");
            }
        } else if let Some(rest) = trimmed.strip_prefix(OFF) {
            *line = rest.trim().to_string();
        }
    }
    write_lines(&path, &lines)?;
    Ok(!was_enabled)
}

pub fn add(manifest: &Manifest, keys: &str, action: &str) -> Result<usize> {
    let keys = keys.trim();
    let action = normalize_action_input(action);
    if keys.is_empty() || action.is_empty() {
        bail!("keys and action required");
    }
    let path = skhdrc(manifest)?;
    let raw = fs::read_to_string(&path)?;
    let mut lines: Vec<String> = raw.lines().map(|l| l.to_string()).collect();
    lines.extend(format_binding_lines(keys, &action, true));
    write_lines(&path, &lines)?;
    Ok(list(manifest)?.len())
}

/// Accept `;;` or literal `\n` escapes as multi-line separators in prompts/CLI.
pub fn normalize_action_input(action: &str) -> String {
    action
        .trim()
        .replace("\\n", "\n")
        .split(";;")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn delete(manifest: &Manifest, line_1based: usize) -> Result<()> {
    let path = skhdrc(manifest)?;
    let raw = fs::read_to_string(&path)?;
    let items = parse_lines(&raw);
    let idx = line_1based
        .checked_sub(1)
        .ok_or_else(|| anyhow!("line must be >= 1"))?;
    let hk = items
        .get(idx)
        .ok_or_else(|| anyhow!("hotkey #{line_1based} not found"))?;
    let mut lines: Vec<String> = raw.lines().map(|l| l.to_string()).collect();
    if hk.end_index >= lines.len() {
        bail!("line out of range");
    }
    lines.drain(hk.index..=hk.end_index);
    write_lines(&path, &lines)?;
    Ok(())
}

pub fn set(
    manifest: &Manifest,
    line_1based: usize,
    keys: Option<&str>,
    action: Option<&str>,
) -> Result<()> {
    let path = skhdrc(manifest)?;
    let raw = fs::read_to_string(&path)?;
    let items = parse_lines(&raw);
    let idx = line_1based
        .checked_sub(1)
        .ok_or_else(|| anyhow!("line must be >= 1"))?;
    let hk = items
        .get(idx)
        .ok_or_else(|| anyhow!("hotkey #{line_1based} not found"))?;
    let new_keys = keys
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&hk.keys);
    let new_action = action
        .map(normalize_action_input)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| hk.action.clone());
    let mut lines: Vec<String> = raw.lines().map(|l| l.to_string()).collect();
    let replacement = format_binding_lines(new_keys, &new_action, hk.enabled);
    lines.splice(hk.index..=hk.end_index, replacement);
    write_lines(&path, &lines)?;
    Ok(())
}

pub fn format_table(keys: &[Hotkey]) -> String {
    let mut out = String::from("Hotkeys (skhd)\n");
    out.push_str(&"─".repeat(72));
    out.push('\n');
    for (n, hk) in keys.iter().enumerate() {
        let mark = if hk.enabled { "ON " } else { "off" };
        let multi = if hk.is_multiline() { " …" } else { "" };
        let action = crate::util::text::truncate_chars(
            &hk.action.replace('\n', " · "),
            44,
        );
        out.push_str(&format!(
            "  {:>3} {mark} {:<24} {}{multi}\n",
            n + 1,
            crate::util::text::truncate_chars(&hk.keys, 24),
            action
        ));
    }
    out.push_str(&format!(
        "\n{} bindings · multi-line via `\\` or add with ;; · # macmgr:off\n",
        keys.len()
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_and_disabled() {
        let raw = r#"
# comment
alt - return : open -na Kitty
# macmgr:off alt - x : echo hi
alt + shift - h : yabai -m window --warp west
"#;
        let keys = parse_lines(raw);
        assert_eq!(keys.len(), 3);
        assert!(keys[0].enabled);
        assert_eq!(keys[0].keys, "alt - return");
        assert!(!keys[1].enabled);
        assert_eq!(keys[1].keys, "alt - x");
        assert!(keys[2].enabled);
    }

    #[test]
    fn parse_skips_plain_comments() {
        let raw = "# just a comment\n\nalt - q : yabai -m window --close\n";
        let keys = parse_lines(raw);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].action, "yabai -m window --close");
    }

    #[test]
    fn parse_multiline_backslash() {
        let raw = r#"
alt - r : yabai -m space --rotate 90; \
    yabai -m space --balance
alt - q : yabai -m window --close
"#;
        let keys = parse_lines(raw);
        assert_eq!(keys.len(), 2);
        assert!(keys[0].is_multiline());
        assert_eq!(keys[0].end_index, keys[0].index + 1);
        assert!(keys[0].action.contains("rotate"));
        assert!(keys[0].action.contains("balance"));
        assert!(!keys[1].is_multiline());
    }

    #[test]
    fn normalize_double_semicolon() {
        let a = normalize_action_input("echo a ;; echo b");
        assert_eq!(a, "echo a\necho b");
    }

    #[test]
    fn format_multiline_binding() {
        let lines = format_binding_lines("alt - r", "foo\nbar", true);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with('\\'));
        assert!(lines[1].contains("bar"));
    }
}
