use std::fs;
use std::process::Command;

use crate::core::brew::{self, brew_available};
use crate::core::manifest::Manifest;
use crate::core::paths::{expand_tilde, launch_agents_dir, which};
use crate::core::service;
use crate::util::shell::run_capture_lossy;

#[derive(Debug, Clone)]
pub struct Check {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

pub fn run_checks(manifest: &Manifest) -> Vec<Check> {
    let mut checks = Vec::new();

    checks.push(Check {
        name: "Homebrew".into(),
        ok: brew_available(),
        detail: which("brew")
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "not found".into()),
    });

    for svc in &manifest.services {
        if !svc.core {
            continue;
        }
        let bin = which(&svc.brew);
        checks.push(Check {
            name: format!("bin:{}", svc.brew),
            ok: bin.is_some(),
            detail: bin
                .map(|p| {
                    brew::version_of(&svc.brew).unwrap_or_else(|| p.display().to_string())
                })
                .unwrap_or_else(|| "missing".into()),
        });
    }

    let statuses = service::list_statuses(manifest);
    for st in statuses.iter().filter(|s| s.core) {
        checks.push(Check {
            name: format!("svc:{}", st.id),
            ok: st.running,
            detail: if st.running {
                "running".into()
            } else if st.installed {
                "installed but not running".into()
            } else {
                "not installed".into()
            },
        });
    }

    let sip = run_capture_lossy("csrutil", &["status"]);
    let sip_enabled = sip.to_lowercase().contains("enabled");
    checks.push(Check {
        name: "SIP".into(),
        ok: true, // informational
        detail: format!(
            "{}{}",
            sip.lines().next().unwrap_or("unknown").trim(),
            if sip_enabled {
                " (advanced yabai features may need SIP disabled)"
            } else {
                ""
            }
        ),
    });

    let agents = launch_agents_dir();
    for svc in manifest.core_services() {
        if svc.label.is_empty() {
            continue;
        }
        let plist = agents.join(format!("{}.plist", svc.label));
        checks.push(Check {
            name: format!("plist:{}", svc.id),
            ok: plist.exists(),
            detail: if plist.exists() {
                plist.display().to_string()
            } else {
                "LaunchAgent missing".into()
            },
        });
    }

    let rice = manifest.rice_path();
    let rice_ok = rice.is_dir();
    let remote_hint = {
        let url = manifest.rice_config().git_url;
        if url.is_empty() {
            "no git_url in manifest (optional)".into()
        } else {
            format!("suggested remote: {url}")
        }
    };
    checks.push(Check {
        name: "rice".into(),
        ok: rice_ok,
        detail: if rice_ok {
            format!("{} · {remote_hint}", rice.display())
        } else {
            format!(
                "missing {} — run: macmgr rice init --git · {remote_hint}",
                rice.display()
            )
        },
    });

    for cfg in &manifest.configs {
        let path = expand_tilde(&cfg.path);
        checks.push(Check {
            name: format!("cfg:{}", cfg.id),
            ok: path.exists(),
            detail: if path.exists() {
                let meta = if path.is_symlink() || is_symlink_component(&path) {
                    "symlink/ok"
                } else {
                    "present"
                };
                format!("{} ({meta})", path.display())
            } else {
                format!("missing: {}", path.display())
            },
        });
    }

    // Accessibility hint — best effort via tccutil is not readable; check via sqlite if possible
    checks.push(Check {
        name: "Accessibility".into(),
        ok: true,
        detail: "grant Accessibility for yabai & skhd in System Settings → Privacy".into(),
    });

    if !manifest.paths.ensure_spaces.is_empty() {
        let p = expand_tilde(&manifest.paths.ensure_spaces);
        checks.push(Check {
            name: "ensure_spaces".into(),
            ok: p.exists(),
            detail: p.display().to_string(),
        });
    }

    let outdated = brew::outdated_list();
    checks.push(Check {
        name: "brew outdated".into(),
        ok: outdated.is_empty(),
        detail: if outdated.is_empty() {
            "up to date".into()
        } else {
            format!("{} packages: {}", outdated.len(), outdated.join(", "))
        },
    });

    // Screen Recording / scripting addition — informational
    let _ = Command::new("true").status();

    checks
}

fn is_symlink_component(path: &std::path::Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

pub fn format_report(checks: &[Check]) -> String {
    let mut out = String::from("macmgr doctor\n");
    out.push_str(&"─".repeat(60));
    out.push('\n');
    for c in checks {
        let mark = if c.ok { "OK " } else { "!! " };
        out.push_str(&format!("{mark} {:<22} {}\n", c.name, c.detail));
    }
    let failed = checks.iter().filter(|c| !c.ok).count();
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!(
        "{} checks, {} need attention\n",
        checks.len(),
        failed
    ));
    out
}
