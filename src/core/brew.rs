use anyhow::{anyhow, Result};

use crate::core::manifest::{Manifest, PackageDef};
use crate::util::shell::{run_capture, run_capture_lossy, run_status, run_status_inherit};

#[derive(Debug, Clone)]
pub struct BrewPkgInfo {
    pub name: String,
    pub kind: String,
    pub installed: bool,
    pub outdated: bool,
}

pub fn brew_available() -> bool {
    crate::core::paths::which("brew").is_some()
}

pub fn ensure_brew() -> Result<()> {
    if brew_available() {
        Ok(())
    } else {
        Err(anyhow!("Homebrew (brew) not found in PATH"))
    }
}

pub fn tap(name: &str) -> Result<()> {
    ensure_brew()?;
    let taps = run_capture_lossy("brew", &["tap"]);
    if taps.lines().any(|l| l.trim() == name) {
        return Ok(());
    }
    run_status("brew", &["tap", name])
}

fn install_args(pkg: &PackageDef) -> Vec<String> {
    let mut args = Vec::new();
    if pkg.kind == "cask" {
        args.push("install".into());
        args.push("--cask".into());
        args.push(pkg.name.clone());
    } else if pkg.head {
        args.push("install".into());
        args.push(pkg.name.clone());
        args.push("--head".into());
    } else {
        args.push("install".into());
        args.push(pkg.name.clone());
    }
    args
}

pub fn install_package(pkg: &PackageDef) -> Result<()> {
    ensure_brew()?;
    if is_installed(&pkg.name, &pkg.kind)? {
        return Ok(());
    }
    let args = install_args(pkg);
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_status("brew", &refs)
}

/// Prints brew output — only with TUI suspended / CLI.
pub fn install_package_visible(pkg: &PackageDef) -> Result<()> {
    ensure_brew()?;
    if is_installed(&pkg.name, &pkg.kind)? {
        println!("{} already installed", pkg.name);
        return Ok(());
    }
    let args = install_args(pkg);
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_status_inherit("brew", &refs)
}

pub fn uninstall_package(name: &str, kind: &str) -> Result<()> {
    ensure_brew()?;
    if kind == "cask" {
        run_status("brew", &["uninstall", "--cask", name])
    } else {
        run_status("brew", &["uninstall", name])
    }
}

pub fn uninstall_package_visible(name: &str, kind: &str) -> Result<()> {
    ensure_brew()?;
    if kind == "cask" {
        run_status_inherit("brew", &["uninstall", "--cask", name])
    } else {
        run_status_inherit("brew", &["uninstall", name])
    }
}

pub fn upgrade_package(name: &str, kind: &str) -> Result<()> {
    ensure_brew()?;
    if kind == "cask" {
        run_status("brew", &["upgrade", "--cask", name])
    } else {
        run_status("brew", &["upgrade", name])
    }
}

pub fn upgrade_all() -> Result<()> {
    ensure_brew()?;
    run_status("brew", &["update"])?;
    run_status("brew", &["upgrade"])
}

pub fn is_installed(name: &str, kind: &str) -> Result<bool> {
    ensure_brew()?;
    let list = if kind == "cask" {
        run_capture_lossy("brew", &["list", "--cask"])
    } else {
        run_capture_lossy("brew", &["list", "--formula"])
    };
    Ok(list.lines().any(|l| l.trim() == name))
}

pub fn outdated_list() -> Vec<String> {
    if !brew_available() {
        return Vec::new();
    }
    run_capture_lossy("brew", &["outdated", "--quiet"])
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

pub fn search(query: &str) -> Result<String> {
    ensure_brew()?;
    run_capture("brew", &["search", query])
}

#[derive(Debug, Clone)]
pub struct BrewSearchHit {
    pub kind: String,
    pub name: String,
}

/// Parse `brew search` into formula/cask hits (best-effort).
pub fn search_hits(query: &str) -> Result<Vec<BrewSearchHit>> {
    let raw = search(query)?;
    Ok(parse_search_output(&raw))
}

pub fn parse_search_output(raw: &str) -> Vec<BrewSearchHit> {
    let mut hits = Vec::new();
    let mut kind = "formula";
    for line in raw.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let lower = t.to_lowercase();
        if lower.contains("formulae") || lower == "==> formulae" {
            kind = "formula";
            continue;
        }
        if lower.contains("casks") || lower == "==> casks" {
            kind = "cask";
            continue;
        }
        if t.starts_with("==>") {
            continue;
        }
        // brew may print "name: description" or just name
        let name = t.split_whitespace().next().unwrap_or(t);
        let name = name.trim_end_matches(':');
        if name.is_empty() || name.contains('/') && !name.contains('@') {
            // skip paths-looking noise; allow foo@1.2
            if name.contains('/') {
                continue;
            }
        }
        if name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '@' | '+' | '.'))
        {
            hits.push(BrewSearchHit {
                kind: kind.to_string(),
                name: name.to_string(),
            });
        }
    }
    hits
}

/// Parse `name`, `cask:name`, `formula:name`, or `--cask name`.
pub fn parse_pkg_spec(spec: &str) -> (String, String) {
    let spec = spec.trim();
    if let Some(rest) = spec.strip_prefix("cask:") {
        return ("cask".into(), rest.trim().to_string());
    }
    if let Some(rest) = spec.strip_prefix("formula:") {
        return ("formula".into(), rest.trim().to_string());
    }
    if let Some(rest) = spec.strip_prefix("--cask ") {
        return ("cask".into(), rest.trim().to_string());
    }
    let parts: Vec<_> = spec.split_whitespace().collect();
    if parts.len() >= 2 && (parts[0] == "--cask" || parts[0] == "-c") {
        return ("cask".into(), parts[1].to_string());
    }
    ("formula".into(), spec.to_string())
}

pub fn adhoc_def(name: &str, kind: &str) -> PackageDef {
    PackageDef {
        name: name.to_string(),
        kind: kind.to_string(),
        group: "adhoc".into(),
        core: false,
        optional: false,
        head: false,
    }
}

/// Install any brew formula/cask by name (visible stdio).
pub fn install_any_visible(spec: &str) -> Result<()> {
    let (kind, name) = parse_pkg_spec(spec);
    if name.is_empty() {
        return Err(anyhow!("empty package name"));
    }
    install_package_visible(&adhoc_def(&name, &kind))
}

pub fn install_any(spec: &str) -> Result<()> {
    let (kind, name) = parse_pkg_spec(spec);
    if name.is_empty() {
        return Err(anyhow!("empty package name"));
    }
    install_package(&adhoc_def(&name, &kind))
}

pub fn package_infos(manifest: &Manifest) -> Vec<BrewPkgInfo> {
    let outdated = outdated_list();
    let formulae = if brew_available() {
        run_capture_lossy("brew", &["list", "--formula"])
    } else {
        String::new()
    };
    let casks = if brew_available() {
        run_capture_lossy("brew", &["list", "--cask"])
    } else {
        String::new()
    };

    manifest
        .packages
        .iter()
        .map(|p| {
            let installed = if p.kind == "cask" {
                casks.lines().any(|l| l.trim() == p.name)
            } else {
                formulae.lines().any(|l| l.trim() == p.name)
            };
            BrewPkgInfo {
                name: p.name.clone(),
                kind: p.kind.clone(),
                installed,
                outdated: outdated.iter().any(|o| o == &p.name),
            }
        })
        .collect()
}

pub fn version_of(bin: &str) -> Option<String> {
    let path = crate::core::paths::which(bin)?;
    let out = run_capture_lossy(path.to_str()?, &["--version"]);
    let line = out.lines().next()?.trim().to_string();
    if line.is_empty() {
        None
    } else {
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_spec_formula_and_cask() {
        assert_eq!(
            parse_pkg_spec("ripgrep"),
            ("formula".into(), "ripgrep".into())
        );
        assert_eq!(
            parse_pkg_spec("cask:firefox"),
            ("cask".into(), "firefox".into())
        );
        assert_eq!(
            parse_pkg_spec("formula:jq"),
            ("formula".into(), "jq".into())
        );
        assert_eq!(
            parse_pkg_spec("--cask kitty"),
            ("cask".into(), "kitty".into())
        );
    }

    #[test]
    fn parse_search_sections() {
        let raw = "\
==> Formulae
neovim
ripgrep
==> Casks
firefox
kitty
";
        let hits = parse_search_output(raw);
        assert_eq!(hits.len(), 4);
        assert_eq!(hits[0].kind, "formula");
        assert_eq!(hits[0].name, "neovim");
        assert_eq!(hits[2].kind, "cask");
        assert_eq!(hits[2].name, "firefox");
    }
}
