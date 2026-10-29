mod cli;
mod core;
mod tui;
mod util;

use anyhow::Result;
use clap::Parser;

use cli::{
    CatalogCmd, Cli, Commands, ConfigCmd, HotkeysCmd, InstallScope, PkgCmd, PluginCmd,
    PluginKindArg, PresetCmd, ProfileCmd, RiceCmd, RulesCmd, ServiceCmd, SnapshotCmd, ThemeCmd,
    WallpaperCmd,
};
use core::brew;
use core::catalog;
use core::config;
use core::doctor;
use core::extras;
use core::hotkeys;
use core::install::{self, InstallMode};
use core::manifest::Manifest;
use core::pack;
use core::plugins::{self, PluginKind};
use core::presets;
use core::profiles;
use core::rice;
use core::rules;
use core::service;
use core::setup;
use core::snapshot::{self, SnapshotKind};
use core::themes;
use core::user_manifest;
use core::wallpaper;
use core::watch;
use util::confirm::confirm;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let command = cli.command.unwrap_or(Commands::Tui);

    // Self-install does not need manifest from disk in weird places
    if matches!(command, Commands::SelfInstall) {
        install::self_install()?;
        return Ok(());
    }
    if matches!(command, Commands::SelfUpdate) {
        core::update::check_latest()?;
        return Ok(());
    }

    let manifest = Manifest::load()?;

    match command {
        Commands::Tui => tui::run(manifest)?,
        Commands::Status { json } => cmd_status(&manifest, json)?,
        Commands::Doctor { json } => {
            let checks = doctor::run_checks(&manifest);
            if json {
                println!("{}", doctor::format_json(&checks)?);
            } else {
                print!("{}", doctor::format_report(&checks));
            }
        }
        Commands::Setup { yes, force } => {
            if !yes
                && !confirm(
                    "Run first-time setup (seed shell/, brew core, plant configs)?",
                    false,
                )?
            {
                println!("aborted");
                return Ok(());
            }
            setup::run(&manifest, yes, force)?;
        }
        Commands::Install { scope, yes } => {
            let mode = match scope {
                InstallScope::Core => InstallMode::Core,
                InstallScope::Full => InstallMode::Full,
            };
            let label = match mode {
                InstallMode::Core => "CORE",
                InstallMode::Full => "FULL",
            };
            if !confirm(&format!("Install {label} rice stack now?"), yes)? {
                println!("aborted");
                return Ok(());
            }
            install::install(&manifest, mode, yes)?;
        }
        Commands::Uninstall { keep_configs, yes } => {
            if !confirm(
                "Uninstall core rice packages (yabai/skhd/sketchybar/borders)?",
                yes,
            )? {
                println!("aborted");
                return Ok(());
            }
            install::uninstall(&manifest, keep_configs, yes)?;
        }
        Commands::Purge {
            yes,
            remove_cli,
            keep_packages,
            keep_live,
            keep_rice,
            keep_data,
        } => {
            println!(
                "WARNING: This permanently removes the tiling rice environment:\n\
                 - stop yabai/skhd/sketchybar/borders\n\
                 - brew uninstall packages from the manifest\n\
                 - delete LIVE configs, My rice, snapshots, macmgr data\n"
            );
            if yes {
                // still require an explicit --yes; no second prompt
            } else {
                print!("Type PURGE to confirm: ");
                use std::io::{stdin, stdout, Write};
                let _ = stdout().flush();
                let mut buf = String::new();
                stdin().read_line(&mut buf)?;
                if buf.trim() != "PURGE" {
                    println!("aborted");
                    return Ok(());
                }
            }
            let opts = install::PurgeOptions {
                remove_packages: !keep_packages,
                remove_live_configs: !keep_live,
                remove_rice: !keep_rice,
                remove_macmgr_data: !keep_data,
                remove_cli,
            };
            install::purge(&manifest, &opts)?;
        }
        Commands::Service { action } => cmd_service(&manifest, action)?,
        Commands::Pkg { action } => cmd_pkg(&manifest, action)?,
        Commands::Config { action } => cmd_config(&manifest, action)?,
        Commands::Rice { action } => cmd_rice(&manifest, action)?,
        Commands::Preset { action } => cmd_preset(&manifest, action)?,
        Commands::Wallpaper { action } => cmd_wallpaper(&manifest, action)?,
        Commands::Hotkeys { action } => {
            let action = action.unwrap_or(HotkeysCmd::List);
            match action {
                HotkeysCmd::List => {
                    let keys = hotkeys::list(&manifest)?;
                    print!("{}", hotkeys::format_table(&keys));
                }
                HotkeysCmd::Toggle { n } => {
                    let on = hotkeys::toggle(&manifest, n)?;
                    println!("hotkey #{n} {}", if on { "enabled" } else { "disabled" });
                }
                HotkeysCmd::Add { keys, action } => {
                    let n = hotkeys::add(&manifest, &keys, &action)?;
                    println!("added · now {n} bindings");
                }
                HotkeysCmd::Delete { n, yes } => {
                    if !confirm(&format!("Delete hotkey #{n}?"), yes)? {
                        println!("aborted");
                        return Ok(());
                    }
                    hotkeys::delete(&manifest, n)?;
                    println!("deleted #{n}");
                }
                HotkeysCmd::Set { n, keys, action } => {
                    hotkeys::set(&manifest, n, keys.as_deref(), action.as_deref())?;
                    println!("updated #{n}");
                }
            }
        }
        Commands::Logs { id, lines } => {
            let sources = core::logs::sources();
            if id == "list" {
                for s in &sources {
                    println!(
                        "{:<12} {} {}",
                        s.id,
                        if s.path.exists() { "ok" } else { "--" },
                        s.path.display()
                    );
                }
            } else if let Some(src) = sources.iter().find(|s| s.id == id || s.label.contains(&id)) {
                println!("# {} ({})", src.label, src.path.display());
                println!("{}", core::logs::read_tail(&src.path, lines));
            } else {
                anyhow::bail!("unknown log `{id}` (try: macmgr logs list)");
            }
        }
        Commands::Plugin { action } => cmd_plugin(&manifest, action)?,
        Commands::Theme { action } => match action {
            ThemeCmd::List => {
                for t in themes::list()? {
                    println!("{:<12} {}", t.id, t.label);
                }
            }
            ThemeCmd::Apply { id, reload } => {
                themes::apply(&manifest, &id, reload)?;
                println!("theme {id} applied");
            }
            ThemeCmd::Backup => {
                let p = themes::backup_current(&manifest)?;
                println!("backup → {}", p.display());
            }
        },
        Commands::Profile { action } => match action {
            ProfileCmd::List => {
                for p in profiles::list()? {
                    println!("{:<12} {}", p.id, p.label);
                }
            }
            ProfileCmd::Apply { id } => {
                profiles::apply(&manifest, &id)?;
                println!("profile {id} applied");
            }
            ProfileCmd::Save { id, label } => {
                let label = if label.is_empty() { id.clone() } else { label };
                let p = profiles::save_from_current(&manifest, &id, &label)?;
                println!("saved {}", p.display());
            }
        },
        Commands::Catalog { action } => match action {
            CatalogCmd::List => {
                for item in catalog::list_all()? {
                    let e = &item.entry;
                    let tag = match item.origin {
                        catalog::CatalogOrigin::Local => "local",
                        catalog::CatalogOrigin::Community => "feed",
                    };
                    let src = if e.path.is_empty() {
                        e.source.clone()
                    } else {
                        format!("{} · {}", e.source, e.path)
                    };
                    println!(
                        "{:<12} {:<6} {:<8} {:<18} {}",
                        e.id, tag, e.kind, e.label, src
                    );
                }
            }
            CatalogCmd::Install { id, reload } => {
                let path = catalog::install(&manifest, &id, reload)?;
                println!("installed → {}", path.display());
            }
            CatalogCmd::FromUrl {
                url,
                id,
                kind,
                path,
                reload,
            } => {
                let dest =
                    catalog::install_url(&manifest, &url, &id, &kind, path.as_deref(), reload)?;
                println!("installed → {}", dest.display());
            }
            CatalogCmd::Add {
                id,
                label,
                source,
                kind,
                path,
                description,
            } => {
                catalog::add_entry(catalog::CatalogEntry {
                    id: id.clone(),
                    label: label.unwrap_or_else(|| id.clone()),
                    kind,
                    description,
                    source,
                    path: path.unwrap_or_default(),
                })?;
                println!("catalog += {id}");
            }
            CatalogCmd::Feeds => {
                for f in catalog::list_feeds()? {
                    println!("{:<16} {}", f.id, f.url);
                }
            }
            CatalogCmd::FeedAdd { url, id } => {
                let id = id.unwrap_or_else(|| {
                    let slug = url
                        .rsplit('/')
                        .next()
                        .unwrap_or("feed")
                        .trim_end_matches(".toml");
                    slug.chars()
                        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                        .collect()
                });
                catalog::add_feed(&id, &url)?;
                println!("feed `{id}` → {url}");
            }
            CatalogCmd::Pull => {
                let r = catalog::pull_feeds()?;
                println!(
                    "pulled {} feed(s) · {} plugin(s) → {}",
                    r.feeds,
                    r.plugins,
                    r.path.display()
                );
            }
        },
        Commands::Rules { action } => match action {
            RulesCmd::List => {
                let list = rules::list(&manifest)?;
                print!("{}", rules::format_table(&list));
            }
            RulesCmd::Toggle { line } => {
                let idx = line.saturating_sub(1);
                // find rule whose display line matches — list uses raw index
                let list = rules::list(&manifest)?;
                let rule = list
                    .iter()
                    .find(|r| r.index + 1 == line)
                    .or_else(|| list.get(idx))
                    .ok_or_else(|| anyhow::anyhow!("no rule for line {line}"))?;
                let on = rules::toggle(&manifest, rule.index)?;
                println!("rule L{} {}", rule.index + 1, if on { "ON" } else { "off" });
            }
            RulesCmd::Add { app, manage_off } => {
                rules::add(&manifest, &app, manage_off)?;
                println!("added rule for {app}");
            }
        },
        Commands::Watch { interval } => {
            println!("watching LIVE sketchybar (interval {interval}ms)");
            println!("Press Enter to stop…");
            let handle = watch::start(&manifest, interval)?;
            let mut buf = String::new();
            let _ = std::io::stdin().read_line(&mut buf);
            handle.stop();
            println!("watch stopped");
        }
        Commands::UpdateCore { yes } => {
            if !confirm("Upgrade core brew packages + reload rice?", yes)? {
                println!("aborted");
                return Ok(());
            }
            for pkg in manifest.core_packages() {
                match brew::upgrade_package(&pkg.name, &pkg.kind) {
                    Ok(()) => println!("upgraded {}", pkg.name),
                    Err(e) => println!("{}: {e}", pkg.name),
                }
            }
            service::reload_rice(&manifest)?;
            println!("done");
        }
        Commands::Snapshot { action } => {
            let action = action.unwrap_or(SnapshotCmd::Create { name: None });
            match action {
                SnapshotCmd::Create { name } => {
                    let info = snapshot::create(&manifest, name.as_deref(), SnapshotKind::Manual)?;
                    println!("snapshot → {} ({})", info.path.display(), info.meta.label);
                }
                SnapshotCmd::List => {
                    for s in snapshot::list()? {
                        println!("{}", snapshot::format_line(&s));
                    }
                }
                SnapshotCmd::Restore { id, yes } => {
                    if !confirm(&format!("Restore snapshot `{id}` (undo point first)?"), yes)? {
                        println!("aborted");
                        return Ok(());
                    }
                    let info = snapshot::restore(&manifest, &id)?;
                    println!("restored {}", info.meta.id);
                    let _ = plugins::reload_bar(&manifest);
                }
                SnapshotCmd::Undo { yes } => {
                    let tip = snapshot::undo_available().unwrap_or_else(|| "(empty)".into());
                    if !confirm(&format!("Undo → restore `{tip}`?"), yes)? {
                        println!("aborted");
                        return Ok(());
                    }
                    let info = snapshot::undo(&manifest)?;
                    println!("undone → {}", info.meta.id);
                    let _ = plugins::reload_bar(&manifest);
                }
                SnapshotCmd::Delete { id, yes } => {
                    if !confirm(&format!("Delete snapshot `{id}`?"), yes)? {
                        println!("aborted");
                        return Ok(());
                    }
                    snapshot::delete(&id)?;
                    println!("deleted {id}");
                }
            }
        }
        Commands::Export { path } => {
            let dest = pack::export(&manifest, path.as_deref().map(std::path::Path::new))?;
            println!("export → {}", dest.display());
        }
        Commands::Import {
            path,
            yes,
            install_packages,
            reload,
        } => {
            if !confirm(
                &format!("Import rice pack `{path}` (creates undo snapshot first)?"),
                yes,
            )? {
                println!("aborted");
                return Ok(());
            }
            let msg = pack::import(
                &manifest,
                std::path::Path::new(&path),
                pack::ImportOpts {
                    install_packages,
                    reload,
                },
            )?;
            println!("{msg}");
        }
        Commands::Spaces => {
            extras::ensure_spaces(&manifest)?;
            println!("spaces ok");
        }
        Commands::Tips => print!("{}", extras::tips()),
        Commands::SelfInstall => unreachable!(),
        Commands::SelfUpdate => unreachable!(),
    }

    Ok(())
}

fn cmd_status(manifest: &Manifest, json: bool) -> Result<()> {
    if json {
        let services: Vec<_> = service::list_statuses(manifest)
            .into_iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "running": s.running,
                    "installed": s.installed,
                    "core": s.core,
                })
            })
            .collect();
        let cfgs = config::list_configs(manifest);
        let body = serde_json::json!({
            "name": manifest.name,
            "version": manifest.version,
            "manifest_version": manifest.manifest_version,
            "services": services,
            "brew_outdated": brew::outdated_list(),
            "configs_present": cfgs.iter().filter(|c| c.exists).count(),
            "configs_total": cfgs.len(),
            "rice": manifest.rice_path().display().to_string(),
        });
        println!("{}", serde_json::to_string_pretty(&body)?);
        return Ok(());
    }
    println!("{} v{}", manifest.name, manifest.version);
    println!("{}", manifest.description);
    println!();
    println!("Services:");
    for s in service::list_statuses(manifest) {
        let mark = if s.running { "●" } else { "○" };
        let core = if s.core { "core" } else { "extra" };
        println!(
            "  {mark} {:<12} {:<8} ({})",
            s.id,
            if s.running { "running" } else { "stopped" },
            core
        );
    }
    println!();
    let outdated = brew::outdated_list();
    println!(
        "Brew outdated: {}",
        if outdated.is_empty() {
            "none".into()
        } else {
            outdated.join(", ")
        }
    );
    let cfgs = config::list_configs(manifest);
    let ok = cfgs.iter().filter(|c| c.exists).count();
    println!("Configs present: {}/{}", ok, cfgs.len());
    Ok(())
}

fn cmd_service(manifest: &Manifest, action: ServiceCmd) -> Result<()> {
    match action {
        ServiceCmd::List => {
            for s in service::list_statuses(manifest) {
                println!(
                    "{:<12} {:<8} installed={}",
                    s.id,
                    if s.running { "running" } else { "stopped" },
                    s.installed
                );
            }
        }
        ServiceCmd::Start { name } => {
            if name == "all" {
                service::act_all(manifest, "start", false)?;
            } else {
                service::start_by_id(manifest, &name)?;
            }
            println!("started {name}");
        }
        ServiceCmd::Stop { name } => {
            if name == "all" {
                service::act_all(manifest, "stop", false)?;
            } else {
                service::stop_by_id(manifest, &name)?;
            }
            println!("stopped {name}");
        }
        ServiceCmd::Restart { name } => {
            if name == "all" {
                service::act_all(manifest, "restart", false)?;
            } else {
                service::restart_by_id(manifest, &name)?;
            }
            println!("restarted {name}");
        }
    }
    Ok(())
}

fn cmd_pkg(manifest: &Manifest, action: PkgCmd) -> Result<()> {
    match action {
        PkgCmd::List => {
            for p in brew::package_infos(manifest) {
                let st = if p.outdated {
                    "outdated"
                } else if p.installed {
                    "installed"
                } else {
                    "missing"
                };
                println!("{:<22} {:<8} {st}", p.name, p.kind);
            }
        }
        PkgCmd::Search { query } => {
            print!("{}", brew::search(&query)?);
        }
        PkgCmd::Install { name } => {
            if let Some(pkg) = manifest.find_package(&name) {
                brew::install_package(pkg)?;
                println!("installed {}", pkg.name);
            } else {
                let (kind, n) = brew::parse_pkg_spec(&name);
                brew::install_any(&name)?;
                println!("installed {kind} {n}");
            }
        }
        PkgCmd::Add { name, install } => {
            let pkg = user_manifest::track_package(&name)?;
            println!(
                "tracked {} {} → {}",
                pkg.kind,
                pkg.name,
                user_manifest::user_overlay_path().display()
            );
            if install {
                brew::install_package(&pkg)?;
                println!("installed {}", pkg.name);
            }
        }
        PkgCmd::Remove { name, force } => {
            user_manifest::untrack_package(&name, force)?;
            println!(
                "untracked {name} → {}",
                user_manifest::user_overlay_path().display()
            );
        }
        PkgCmd::Upgrade { name } => {
            if let Some(name) = name {
                let kind = manifest
                    .find_package(&name)
                    .map(|p| p.kind.as_str())
                    .unwrap_or("formula");
                brew::upgrade_package(&name, kind)?;
                println!("upgraded {name}");
            } else {
                brew::upgrade_all()?;
                println!("upgraded all");
            }
        }
        PkgCmd::Uninstall { name, yes } => {
            if !confirm(&format!("Uninstall {name}?"), yes)? {
                println!("aborted");
                return Ok(());
            }
            let kind = manifest
                .find_package(&name)
                .map(|p| p.kind.as_str())
                .unwrap_or("formula");
            brew::uninstall_package(&name, kind)?;
            println!("uninstalled {name}");
        }
        PkgCmd::UpdateAll { yes } => {
            if !confirm("brew update && brew upgrade?", yes)? {
                println!("aborted");
                return Ok(());
            }
            brew::upgrade_all()?;
            println!("done");
        }
    }
    Ok(())
}

fn cmd_config(manifest: &Manifest, action: ConfigCmd) -> Result<()> {
    match action {
        ConfigCmd::List => {
            for c in config::list_configs(manifest) {
                let sync = match c.in_sync {
                    Some(true) => "sync",
                    Some(false) => "DIFF",
                    None => "—",
                };
                println!(
                    "{:<14} exists={} dir={} sync={:<4} {}",
                    c.id,
                    c.exists,
                    c.is_dir,
                    sync,
                    c.path.display()
                );
            }
        }
        ConfigCmd::Files { id } => {
            for p in config::list_live_files(manifest, &id)? {
                println!("{}", p.display());
            }
        }
        ConfigCmd::Edit { id } => config::edit(manifest, &id)?,
        ConfigCmd::Open { id } => config::open(manifest, &id)?,
        ConfigCmd::Add { spec, kind } => {
            let joined = spec.join(" ");
            let (id, path) = user_manifest::parse_config_add_spec(&joined)?;
            let cfg = user_manifest::track_config(manifest, &id, &path, kind.as_deref())?;
            println!(
                "tracked config {} → {} (rice {})",
                cfg.id, cfg.path, cfg.repo_path
            );
            println!("overlay {}", user_manifest::user_overlay_path().display());
        }
        ConfigCmd::Remove { id, force } => {
            user_manifest::untrack_config(&id, force)?;
            println!(
                "untracked config {id} → {}",
                user_manifest::user_overlay_path().display()
            );
        }
        ConfigCmd::Backup => {
            let dest = config::backup_all(manifest)?;
            println!("backup written to {}", dest.display());
        }
        ConfigCmd::Restore { yes } => {
            if !confirm(
                "Restore latest config backup (overwrites live configs)?",
                yes,
            )? {
                println!("aborted");
                return Ok(());
            }
            let src = config::restore_latest(manifest)?;
            println!("restored from {}", src.display());
        }
        ConfigCmd::Diff => {
            print!("{}", config::diff_all(manifest)?);
        }
        ConfigCmd::Validate => {
            crate::core::validate::sketchybar_live(manifest)?;
            let n = crate::core::validate::sketchybar_lua_files(manifest)?.len();
            println!("ok · {n} Lua file(s) validated");
        }
    }
    Ok(())
}

fn cmd_rice(manifest: &Manifest, action: RiceCmd) -> Result<()> {
    match action {
        RiceCmd::Reload => {
            service::reload_rice(manifest)?;
            println!("core services restarted");
        }
        RiceCmd::Init { git } => {
            let path = rice::init(manifest, git)?;
            println!("rice init → {}", path.display());
            if git {
                println!("git ready · add remote and push when you want to publish");
            }
        }
        RiceCmd::Status => {
            println!("rice: {}", rice::rice_dir(manifest).display());
            for s in rice::status(manifest) {
                println!("{}", rice::format_status_line(&s));
            }
        }
        RiceCmd::Diff { id, file } => {
            let file = file.as_deref().map(std::path::Path::new);
            print!("{}", rice::diff(manifest, id.as_deref(), file)?);
        }
        RiceCmd::Push { id, file } => {
            let file = file.as_deref().map(std::path::Path::new);
            let n = rice::push(manifest, id.as_deref(), file)?;
            println!("pushed {n} item(s) LIVE → rice (pinned locally)");
            println!("optional: macmgr rice commit && macmgr rice git-push");
        }
        RiceCmd::Pull { id, file, yes } => {
            let label = id.as_deref().unwrap_or("all managed");
            if !confirm(
                &format!("Pull rice → LIVE (`{label}`)? Creates undo snapshot first."),
                yes,
            )? {
                println!("aborted");
                return Ok(());
            }
            let file = file.as_deref().map(std::path::Path::new);
            let n = rice::pull(manifest, id.as_deref(), file)?;
            println!("pulled {n} item(s) rice → LIVE · undo available");
            let _ = plugins::reload_bar(manifest);
        }
        RiceCmd::GitStatus => {
            print!("{}", rice::git_status(manifest)?);
        }
        RiceCmd::Commit { message } => {
            println!("{}", rice::git_commit(manifest, message.as_deref())?);
        }
        RiceCmd::GitPush => {
            println!("{}", rice::git_push(manifest)?);
        }
        RiceCmd::Sync { direction, yes } => match direction.as_str() {
            "to-repo" | "to" | "push" => {
                let n = rice::push(manifest, None, None)?;
                println!("synced LIVE → rice ({n})");
            }
            "from-repo" | "from" | "pull" => {
                if !confirm("Pull ALL rice → LIVE (undo first)?", yes)? {
                    println!("aborted");
                    return Ok(());
                }
                let n = rice::pull(manifest, None, None)?;
                println!("synced rice → LIVE ({n}) · undo available");
                let _ = plugins::reload_bar(manifest);
            }
            other => anyhow::bail!("unknown sync direction `{other}` (to-repo|from-repo)"),
        },
    }
    Ok(())
}

fn cmd_preset(manifest: &Manifest, action: PresetCmd) -> Result<()> {
    match action {
        PresetCmd::List => {
            for (id, desc) in presets::list(manifest) {
                println!("{:<14} {desc}", id);
            }
        }
        PresetCmd::Apply { id } => {
            presets::apply(manifest, &id)?;
            println!("applied preset {id}");
        }
    }
    Ok(())
}

fn cmd_wallpaper(manifest: &Manifest, action: WallpaperCmd) -> Result<()> {
    match action {
        WallpaperCmd::List => {
            for (i, p) in wallpaper::list(manifest)?.iter().enumerate() {
                println!(
                    "{i:>3}  {}",
                    p.file_name().and_then(|n| n.to_str()).unwrap_or("?")
                );
            }
        }
        WallpaperCmd::Set { target } => {
            if let Ok(idx) = target.parse::<usize>() {
                wallpaper::set_by_index(manifest, idx)?;
            } else {
                wallpaper::set_by_name(manifest, &target)?;
            }
            println!("wallpaper set");
        }
    }
    Ok(())
}

fn maybe_reload_bar(manifest: &Manifest, reload: bool) -> Result<()> {
    if reload {
        plugins::reload_bar(manifest)?;
        println!("sketchybar reloaded");
    }
    Ok(())
}

fn cmd_plugin(manifest: &Manifest, action: PluginCmd) -> Result<()> {
    match action {
        PluginCmd::List => {
            let list = plugins::list(manifest)?;
            print!("{}", plugins::format_table(&list));
        }
        PluginCmd::Enable { id, reload } => {
            plugins::enable(manifest, &id)?;
            println!("enabled {id}");
            maybe_reload_bar(manifest, reload)?;
        }
        PluginCmd::Disable { id, reload } => {
            plugins::disable(manifest, &id)?;
            println!("disabled {id}");
            maybe_reload_bar(manifest, reload)?;
        }
        PluginCmd::Toggle { id, reload } => {
            let on = plugins::toggle(manifest, &id)?;
            println!("{} {id}", if on { "enabled" } else { "disabled" });
            maybe_reload_bar(manifest, reload)?;
        }
        PluginCmd::Add { name, kind, reload } => {
            let kind = match kind {
                PluginKindArg::Item => PluginKind::Item,
                PluginKindArg::Widget => PluginKind::Widget,
            };
            let path = plugins::add(manifest, &name, kind)?;
            println!("created {}", path.display());
            maybe_reload_bar(manifest, reload)?;
        }
        PluginCmd::Remove {
            id,
            delete,
            yes,
            reload,
        } => {
            if delete && !confirm(&format!("Delete plugin file for `{id}`?"), yes)? {
                println!("aborted");
                return Ok(());
            }
            plugins::remove(manifest, &id, delete)?;
            println!("removed {id} (delete_file={delete})");
            maybe_reload_bar(manifest, reload)?;
        }
        PluginCmd::Edit { id } => plugins::edit(manifest, &id)?,
        PluginCmd::Reload => {
            plugins::reload_bar(manifest)?;
            println!("sketchybar reloaded");
        }
    }
    Ok(())
}
