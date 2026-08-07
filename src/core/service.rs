use anyhow::{anyhow, Result};

use crate::core::manifest::{Manifest, ServiceDef};
use crate::util::shell::{run_capture_lossy, run_status};

#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub id: String,
    pub running: bool,
    pub installed: bool,
    pub core: bool,
}

pub fn list_statuses(manifest: &Manifest) -> Vec<ServiceStatus> {
    list_statuses_fast(manifest)
}

/// Fast path: pgrep + which only (no brew/launchctl subprocess storms).
pub fn list_statuses_fast(manifest: &Manifest) -> Vec<ServiceStatus> {
    manifest
        .services
        .iter()
        .map(|s| {
            let installed = crate::core::paths::which(&s.brew).is_some();
            let running = installed && is_process_running(&s.brew);
            ServiceStatus {
                id: s.id.clone(),
                running,
                installed,
                core: s.core,
            }
        })
        .collect()
}

/// Slower but more accurate (brew services + launchctl).
pub fn list_statuses_full(manifest: &Manifest) -> Vec<ServiceStatus> {
    let brew_services = run_capture_lossy("brew", &["services", "list"]);
    let uid = users_uid();
    manifest
        .services
        .iter()
        .map(|s| {
            let installed = crate::core::paths::which(&s.brew).is_some();
            let running = if !s.label.is_empty() {
                launchctl_running(uid, &s.label)
            } else {
                false
            } || brew_service_started(&brew_services, &s.brew)
                || is_process_running(&s.brew);
            ServiceStatus {
                id: s.id.clone(),
                running,
                installed,
                core: s.core,
            }
        })
        .collect()
}

fn users_uid() -> u32 {
    run_capture_lossy("id", &["-u"])
        .trim()
        .parse()
        .unwrap_or(501)
}

fn launchctl_running(uid: u32, label: &str) -> bool {
    let out = run_capture_lossy("launchctl", &["print", &format!("gui/{uid}/{label}")]);
    out.contains("state = running")
}

fn brew_service_started(brew_services: &str, name: &str) -> bool {
    brew_services.lines().any(|line| {
        let parts: Vec<_> = line.split_whitespace().collect();
        parts.first().copied() == Some(name)
            && parts
                .get(1)
                .is_some_and(|st| *st == "started" || *st == "running")
    })
}

pub fn is_process_running(name: &str) -> bool {
    let out = run_capture_lossy("pgrep", &["-x", name]);
    !out.trim().is_empty()
}

pub fn start(svc: &ServiceDef) -> Result<()> {
    if crate::core::paths::which(&svc.brew).is_none() {
        return Err(anyhow!("{} is not installed", svc.brew));
    }
    match svc.control.as_str() {
        "native" => run_status(&svc.brew, &["--start-service"]),
        _ => run_status("brew", &["services", "start", &svc.brew]),
    }
}

pub fn stop(svc: &ServiceDef) -> Result<()> {
    if crate::core::paths::which(&svc.brew).is_none() {
        return Ok(());
    }
    match svc.control.as_str() {
        "native" => run_status(&svc.brew, &["--stop-service"]),
        _ => run_status("brew", &["services", "stop", &svc.brew]),
    }
}

pub fn restart(svc: &ServiceDef) -> Result<()> {
    if crate::core::paths::which(&svc.brew).is_none() {
        return Err(anyhow!("{} is not installed", svc.brew));
    }
    match svc.control.as_str() {
        "native" => run_status(&svc.brew, &["--restart-service"]),
        _ => run_status("brew", &["services", "restart", &svc.brew]),
    }
}

pub fn start_by_id(manifest: &Manifest, id: &str) -> Result<()> {
    start(resolve(manifest, id)?)
}

pub fn stop_by_id(manifest: &Manifest, id: &str) -> Result<()> {
    stop(resolve(manifest, id)?)
}

pub fn restart_by_id(manifest: &Manifest, id: &str) -> Result<()> {
    restart(resolve(manifest, id)?)
}

pub fn act_all(manifest: &Manifest, action: &str, core_only: bool) -> Result<()> {
    for svc in manifest.services.iter().filter(|s| !core_only || s.core) {
        let res = match action {
            "start" => start(svc),
            "stop" => stop(svc),
            "restart" => restart(svc),
            _ => return Err(anyhow!("unknown action {action}")),
        };
        if let Err(e) = res {
            eprintln!("warn: {} {}: {e}", action, svc.id);
        }
    }
    Ok(())
}

pub fn reload_rice(manifest: &Manifest) -> Result<()> {
    crate::core::validate::before_reload(manifest, None)?;
    for svc in manifest.core_services() {
        if crate::core::paths::which(&svc.brew).is_none() {
            eprintln!("skip: {} not installed", svc.brew);
            continue;
        }
        if let Err(e) = restart(svc) {
            eprintln!("warn: restart {}: {e}", svc.id);
        }
    }
    Ok(())
}

fn resolve<'a>(manifest: &'a Manifest, id: &str) -> Result<&'a ServiceDef> {
    manifest
        .find_service(id)
        .ok_or_else(|| anyhow!("unknown service `{id}`"))
}
