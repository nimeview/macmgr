use std::fs;
use std::process::Command;

use serde::Serialize;

use crate::core::brew::{self, brew_available};
use crate::core::manifest::Manifest;
use crate::core::paths::{expand_tilde, launch_agents_dir, which};
use crate::core::service;
use crate::core::setup;
use crate::util::shell::run_capture_lossy;

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub name: String,
    pub ok: bool,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

pub fn run_checks(manifest: &Manifest) -> Vec<Check> {
    let mut checks = Vec::new();

    checks.push(Check {
        name: "Homebrew".into(),
        ok: brew_available(),
        detail: which("brew")
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "not found".into()),
        fix: (!brew_available()).then(|| {
            "/bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"".into()
        }),
    });

    for svc in &manifest.services {
        if !svc.core {
            continue;
        }
        let bin = which(&svc.brew);
        let ok = bin.is_some();
        checks.push(Check {
            name: format!("bin:{}", svc.brew),
            ok,
            detail: bin
                .map(|p| brew::version_of(&svc.brew).unwrap_or_else(|| p.display().to_string()))
                .unwrap_or_else(|| "missing".into()),
            fix: (!ok).then(|| format!("macmgr pkg install {} · or: macmgr setup --yes", svc.brew)),
        });
    }

    let statuses = service::list_statuses(manifest);
    for st in statuses.iter().filter(|s| s.core) {
        let ok = st.running;
        checks.push(Check {
            name: format!("svc:{}", st.id),
            ok,
            detail: if st.running {
                "running".into()
            } else if st.installed {
                "installed but not running".into()
            } else {
                "not installed".into()
            },
            fix: (!ok).then(|| format!("macmgr service start {}", st.id)),
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
                " (yabai scripting addition may need SIP partially disabled)"
            } else {
                ""
            }
        ),
        fix: sip_enabled.then(|| "See docs/INSTALL.md · yabai wiki: scripting-addition".into()),
    });

    let agents = launch_agents_dir();
    for svc in manifest.core_services() {
        if svc.label.is_empty() {
            continue;
        }
        let plist = agents.join(format!("{}.plist", svc.label));
        let ok = plist.exists();
        checks.push(Check {
            name: format!("plist:{}", svc.id),
            ok,
            detail: if ok {
                plist.display().to_string()
            } else {
                "LaunchAgent missing".into()
            },
            fix: (!ok).then(|| format!("macmgr service start {}  # registers agent", svc.id)),
        });
    }

    let rice = manifest.rice_path();
    let rice_ok = rice.is_dir() && rice.join(".config").is_dir();
    checks.push(Check {
        name: "rice".into(),
        ok: rice_ok,
        detail: if rice_ok {
            rice.display().to_string()
        } else {
            format!("missing {} — seed starter or rice init", rice.display())
        },
        fix: (!rice_ok).then(|| "macmgr setup --yes · or: macmgr rice init --git".into()),
    });

    checks.push(Check {
        name: "shell_pack".into(),
        ok: setup::find_shell_pack().is_some(),
        detail: setup::describe_shell_pack(),
        fix: setup::find_shell_pack()
            .is_none()
            .then(|| "Clone MacOS_Manager repo (shell/ required for first-time setup)".into()),
    });

    for cfg in &manifest.configs {
        let path = expand_tilde(&cfg.path);
        let ok = path.exists();
        checks.push(Check {
            name: format!("cfg:{}", cfg.id),
            ok,
            detail: if ok {
                let meta = if path.is_symlink() || is_symlink_component(&path) {
                    "symlink/ok"
                } else {
                    "present"
                };
                format!("{} ({meta})", path.display())
            } else {
                format!("missing: {}", path.display())
            },
            fix: (!ok).then(|| "macmgr setup --yes · or: macmgr rice pull --yes".into()),
        });
    }

    // kitty
    let kitty_conf = expand_tilde("~/.config/kitty/kitty.conf");
    let kitty_ok = kitty_conf.exists();
    checks.push(Check {
        name: "kitty.conf".into(),
        ok: kitty_ok,
        detail: if kitty_ok {
            kitty_conf.display().to_string()
        } else {
            "missing — theme apply / setup will create it".into()
        },
        fix: (!kitty_ok).then(|| "macmgr setup --yes · or copy shell/.config/kitty".into()),
    });

    // skhdrc readable
    let skhd = expand_tilde("~/.config/skhd/skhdrc");
    let skhd_ok = skhd.exists();
    let skhd_detail = if skhd_ok {
        let size = fs::metadata(&skhd).map(|m| m.len()).unwrap_or(0);
        format!("{} ({size} bytes)", skhd.display())
    } else {
        "missing".into()
    };
    checks.push(Check {
        name: "skhdrc".into(),
        ok: skhd_ok,
        detail: skhd_detail,
        fix: (!skhd_ok).then(|| "macmgr setup --yes".into()),
    });

    // yabai spaces count
    let spaces_json = run_capture_lossy("yabai", &["-m", "query", "--spaces"]);
    if !spaces_json.is_empty() && spaces_json.trim_start().starts_with('[') {
        let count = spaces_json.matches("\"index\"").count().max(
            spaces_json
                .lines()
                .filter(|l| l.contains("\"index\""))
                .count(),
        );
        // crude: count "has-focus" or parse indices
        let n = spaces_json.matches("\"is-native-fullscreen\"").count();
        let n = if n == 0 {
            spaces_json.matches("\"index\":").count()
        } else {
            n
        };
        checks.push(Check {
            name: "spaces".into(),
            ok: n >= 1,
            detail: format!("{n} space(s) · want ~9 for starter bar"),
            fix: (n < 9).then(|| format!("macmgr spaces · or: {}", manifest.paths.ensure_spaces)),
        });
        let _ = count;
    } else {
        checks.push(Check {
            name: "spaces".into(),
            ok: false,
            detail: "yabai query failed (is yabai running?)".into(),
            fix: Some("macmgr service start yabai".into()),
        });
    }

    checks.push(Check {
        name: "Accessibility".into(),
        ok: true,
        detail: "grant Accessibility for yabai & skhd in System Settings → Privacy & Security"
            .into(),
        fix: Some(
            "open \"x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility\""
                .into(),
        ),
    });

    if !manifest.paths.ensure_spaces.is_empty() {
        let p = expand_tilde(&manifest.paths.ensure_spaces);
        let ok = p.exists();
        checks.push(Check {
            name: "ensure_spaces".into(),
            ok,
            detail: p.display().to_string(),
            fix: (!ok).then(|| "macmgr setup --yes  # installs helper to ~/bin".into()),
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
        fix: (!outdated.is_empty()).then(|| "macmgr pkg update-all --yes".into()),
    });

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
        if !c.ok {
            if let Some(fix) = &c.fix {
                out.push_str(&format!("      fix: {fix}\n"));
            }
        }
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

pub fn format_json(checks: &[Check]) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(checks)
}
