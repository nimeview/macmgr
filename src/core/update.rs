//! Best-effort self-update hints (GitHub Releases / brew).

use anyhow::Result;

use crate::util::shell::run_capture_lossy;

const RELEASES: &str = "https://github.com/arturpanan/MacOS_Manager/releases/latest";
const REPO: &str = "https://github.com/arturpanan/MacOS_Manager";

pub fn print_update_help() {
    println!("macmgr self-update");
    println!("{}", "─".repeat(48));
    println!("Current: {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Options:");
    println!("  1. From source (dev / preview):");
    println!("       cd <clone> && cargo install --path . --force");
    println!("  2. GitHub Release binary:");
    println!("       {RELEASES}");
    println!("  3. Homebrew tap (when published):");
    println!("       brew upgrade macmgr");
    println!();
    println!("Repo: {REPO}");
}

/// Probe latest release tag via GitHub API (best-effort, no auth).
pub fn check_latest() -> Result<()> {
    print_update_help();
    let body = run_capture_lossy(
        "curl",
        &[
            "-fsSL",
            "-H",
            "Accept: application/vnd.github+json",
            "https://api.github.com/repos/arturpanan/MacOS_Manager/releases/latest",
        ],
    );
    if body.is_empty() {
        println!("\n(Could not query GitHub API — offline or repo not public yet.)");
        return Ok(());
    }
    if let Some(tag) = body
        .lines()
        .find(|l| l.contains("\"tag_name\""))
        .and_then(|l| l.split('"').nth(3))
    {
        println!("\nLatest release tag: {tag}");
        if tag.trim_start_matches('v') != env!("CARGO_PKG_VERSION") {
            println!(
                "You are on {} — consider updating.",
                env!("CARGO_PKG_VERSION")
            );
        } else {
            println!("You appear up to date.");
        }
    }
    Ok(())
}
