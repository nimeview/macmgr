use std::path::PathBuf;

use crate::core::brew;
use crate::core::catalog;
use crate::core::config;
use crate::core::extras;
use crate::core::hotkeys;
use crate::core::install::{self, InstallMode};
use crate::core::logs;
use crate::core::pack;
use crate::core::plugins::{self, PluginKind};
use crate::core::presets;
use crate::core::profiles;
use crate::core::rice;
use crate::core::rules;
use crate::core::service;
use crate::core::snapshot::{self, SnapshotKind};
use crate::core::themes;
use crate::core::wallpaper;
use crate::core::watch;

use super::state::{App, ConfirmAction, Overlay, PromptKind, Tab};

impl App {
    pub fn queue_suspend<F>(&mut self, f: F)
    where
        F: FnOnce() -> String + Send + 'static,
    {
        self.pending = Some(Box::new(f));
        self.dirty = true;
    }

    pub fn queue_editor<F>(&mut self, f: F)
    where
        F: FnOnce() -> String + Send + 'static,
    {
        self.pending_editor = Some(Box::new(f));
        self.dirty = true;
    }

    /// Open LIVE path in nano (or `$MACMGR_EDITOR`).
    pub fn open_external_path(&mut self, path: PathBuf) {
        let auto = self.auto_reload_plugins;
        let manifest = self.manifest.clone();
        self.queue_editor(move || {
            match config::edit_file_simple(&path) {
                Ok(()) => {
                    if auto && path.extension().and_then(|e| e.to_str()) == Some("lua") {
                        match plugins::reload_bar_after(&manifest, &path) {
                            Ok(()) => format!(
                                "edited {} · validated · bar reloaded",
                                path.display()
                            ),
                            Err(e) => format!(
                                "edited {} · reload blocked:\n{e}",
                                path.display()
                            ),
                        }
                    } else {
                        format!("edited {}", path.display())
                    }
                }
                Err(e) => format!("editor error: {e}"),
            }
        });
    }

    pub fn open_external_config(&mut self, id: &str) {
        let id = id.to_string();
        let manifest = self.manifest.clone();
        let auto = self.auto_reload_plugins;
        self.queue_editor(move || match config::edit_external_simple(&manifest, &id) {
            Ok(()) => {
                if auto && id == "sketchybar" {
                    match plugins::reload_bar(&manifest) {
                        Ok(()) => format!("edited {id} · validated · bar reloaded"),
                        Err(e) => format!("edited {id} · reload blocked:\n{e}"),
                    }
                } else {
                    format!("edited {id}")
                }
            }
            Err(e) => format!("editor error: {e}"),
        });
    }

    pub fn run_confirmed(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::PkgInstall(name) => {
                let def = self.manifest.find_package(&name).cloned();
                self.queue_suspend(move || {
                    let res = if let Some(def) = def {
                        brew::install_package_visible(&def)
                    } else {
                        brew::install_any_visible(&name)
                    };
                    match res {
                        Ok(()) => format!("Installed {name}\n\nPress Enter…"),
                        Err(e) => format!("error: {e}\n\nPress Enter…"),
                    }
                });
            }
            ConfirmAction::PkgUninstall(name) => {
                let kind = self
                    .manifest
                    .find_package(&name)
                    .map(|p| p.kind.clone())
                    .unwrap_or_else(|| "formula".into());
                self.queue_suspend(move || match brew::uninstall_package_visible(&name, &kind) {
                    Ok(()) => format!("Uninstalled {name}\n\nPress Enter…"),
                    Err(e) => format!("error: {e}\n\nPress Enter…"),
                });
            }
            ConfirmAction::PkgUpgrade(name) => {
                let kind = self
                    .manifest
                    .find_package(&name)
                    .map(|p| p.kind.clone())
                    .unwrap_or_else(|| "formula".into());
                self.queue_suspend(move || match brew::upgrade_package(&name, &kind) {
                    Ok(()) => format!("Upgraded {name}\n\nPress Enter…"),
                    Err(e) => format!("error: {e}\n\nPress Enter…"),
                });
            }
            ConfirmAction::InstallCore | ConfirmAction::InstallFull => {
                let full = matches!(action, ConfirmAction::InstallFull);
                let manifest = self.manifest.clone();
                self.queue_suspend(move || {
                    let mode = if full {
                        InstallMode::Full
                    } else {
                        InstallMode::Core
                    };
                    match install::install(&manifest, mode, true) {
                        Ok(()) => "Install finished.\n\nPress Enter…".into(),
                        Err(e) => format!("Install error: {e}\n\nPress Enter…"),
                    }
                });
            }
            ConfirmAction::ReloadRice => {
                let manifest = self.manifest.clone();
                self.queue_suspend(move || match service::reload_rice(&manifest) {
                    Ok(()) => "Rice reloaded.\n\nPress Enter…".into(),
                    Err(e) => format!("error: {e}\n\nPress Enter…"),
                });
            }
            ConfirmAction::RestoreConfigs => {
                let _ = snapshot::push_undo(&self.manifest, "before-backup-restore");
                match config::restore_latest(&self.manifest) {
                    Ok(p) => {
                        self.set_msg(format!("restored from {} · undo ready", p.display()));
                        self.refresh_tab(true);
                    }
                    Err(e) => self.set_msg(format!("error: {e}")),
                }
            }
            ConfirmAction::RestoreSnapshot(id) => {
                match snapshot::restore(&self.manifest, &id) {
                    Ok(info) => {
                        let _ = plugins::reload_bar(&self.manifest);
                        self.snapshots = snapshot::list().unwrap_or_default();
                        self.overlay = Overlay::Snapshots;
                        self.set_msg(format!(
                            "restored {} · undo ready",
                            info.meta.id
                        ));
                        self.refresh_tab(true);
                    }
                    Err(e) => self.set_msg(format!("error: {e}")),
                }
            }
            ConfirmAction::DeleteSnapshot(id) => match snapshot::delete(&id) {
                Ok(()) => {
                    self.snapshots = snapshot::list().unwrap_or_default();
                    self.clamp_selected();
                    self.set_msg(format!("deleted {id}"));
                }
                Err(e) => self.set_msg(format!("error: {e}")),
            },
            ConfirmAction::Undo => match snapshot::undo(&self.manifest) {
                Ok(info) => {
                    let _ = plugins::reload_bar(&self.manifest);
                    self.snapshots = snapshot::list().unwrap_or_default();
                    self.set_msg(format!("undone → {}", info.meta.id));
                    self.refresh_tab(true);
                }
                Err(e) => self.set_msg(format!("error: {e}")),
            },
            ConfirmAction::ImportPack(path) => {
                let manifest = self.manifest.clone();
                self.queue_suspend(move || {
                    match pack::import(
                        &manifest,
                        PathBuf::from(&path).as_path(),
                        pack::ImportOpts::default(),
                    ) {
                        Ok(msg) => format!("{msg}\n\nPress Enter…"),
                        Err(e) => format!("error: {e}\n\nPress Enter…"),
                    }
                });
            }
            ConfirmAction::RicePull { id } => {
                match rice::pull(&self.manifest, id.as_deref(), None) {
                    Ok(n) => {
                        let _ = plugins::reload_bar(&self.manifest);
                        self.rice_status = rice::status(&self.manifest);
                        self.set_msg(format!("pulled {n} · undo ready"));
                        self.refresh_tab(true);
                    }
                    Err(e) => self.set_msg(format!("error: {e}")),
                }
            }
            ConfirmAction::RicePullAll => match rice::pull(&self.manifest, None, None) {
                Ok(n) => {
                    let _ = plugins::reload_bar(&self.manifest);
                    self.rice_status = rice::status(&self.manifest);
                    self.set_msg(format!("pulled all ({n}) · undo ready"));
                    self.refresh_tab(true);
                }
                Err(e) => self.set_msg(format!("error: {e}")),
            },
            ConfirmAction::DeleteHotkey(n) => match hotkeys::delete(&self.manifest, n) {
                Ok(()) => {
                    self.hotkeys = hotkeys::list(&self.manifest).unwrap_or_default();
                    self.clamp_selected();
                    self.set_msg(format!("deleted hotkey #{n}"));
                }
                Err(e) => self.set_msg(format!("error: {e}")),
            },
            ConfirmAction::UpdateCore => {
                let manifest = self.manifest.clone();
                self.queue_suspend(move || {
                    let mut out = String::new();
                    for pkg in manifest.core_packages() {
                        match brew::upgrade_package(&pkg.name, &pkg.kind) {
                            Ok(()) => out.push_str(&format!("upgraded {}\n", pkg.name)),
                            Err(e) => out.push_str(&format!("{}: {e}\n", pkg.name)),
                        }
                    }
                    let _ = service::reload_rice(&manifest);
                    out.push_str("\nCore update done.\n\nPress Enter…");
                    out
                });
            }
            ConfirmAction::ClearLog(path) => {
                let _ = logs::clear(&path);
                self.set_msg("log cleared");
            }
            ConfirmAction::StopService(id) => match service::stop_by_id(&self.manifest, &id) {
                Ok(()) => {
                    self.set_msg(format!("stopped {id}"));
                    self.refresh_tab(false);
                }
                Err(e) => self.set_msg(format!("error: {e}")),
            },
        }
    }

    pub fn on_enter(&mut self) {
        match &self.overlay {
            Overlay::Confirm {
                action,
                yes,
                return_selected,
                ..
            } => {
                let action = action.clone();
                let yes = *yes;
                let ret = *return_selected;
                self.overlay = Overlay::None;
                self.set_selected(ret);
                if yes {
                    self.run_confirmed(action);
                } else {
                    self.set_msg("cancelled");
                }
                return;
            }
            Overlay::FileBrowser { files, .. } => {
                let idx = self.selected();
                if let Some(path) = files.get(idx).cloned() {
                    self.open_external_path(path);
                }
                return;
            }
            Overlay::LogView { .. }
            | Overlay::DiffView { .. }
            | Overlay::Help
            | Overlay::Filter
            | Overlay::Prompt { .. } => return,
            Overlay::Palette { query } => {
                let items = super::palette::filtered(query);
                if let Some(cmd) = items.get(self.selected()).copied() {
                    let id = cmd.id;
                    self.overlay = Overlay::None;
                    self.run_palette(id);
                }
                return;
            }
            Overlay::PkgSearch { results, .. } => {
                let indices = self.filtered_indices();
                if let Some(&real) = indices.get(self.selected()) {
                    if let Some(hit) = results.get(real).cloned() {
                        let spec = if hit.kind == "cask" {
                            format!("cask:{}", hit.name)
                        } else {
                            hit.name.clone()
                        };
                        self.open_confirm(
                            format!("Install {} `{}` via brew?", hit.kind, hit.name),
                            ConfirmAction::PkgInstall(spec),
                        );
                    }
                }
                return;
            }
            Overlay::Hotkeys => {
                let indices = self.filtered_indices();
                if let Some(&real) = indices.get(self.selected()) {
                    let n = real + 1;
                    match hotkeys::toggle(&self.manifest, n) {
                        Ok(on) => {
                            self.hotkeys = hotkeys::list(&self.manifest).unwrap_or_default();
                            self.set_msg(format!(
                                "hotkey #{n} {}",
                                if on { "enabled" } else { "disabled" }
                            ));
                        }
                        Err(e) => self.set_msg(format!("error: {e}")),
                    }
                }
                return;
            }
            Overlay::Snapshots => {
                let indices = self.filtered_indices();
                if let Some(&real) = indices.get(self.selected()) {
                    if let Some(s) = self.snapshots.get(real) {
                        let id = s.meta.id.clone();
                        self.open_confirm(
                            format!("Restore snapshot `{id}`? (undo point first)"),
                            ConfirmAction::RestoreSnapshot(id),
                        );
                    }
                }
                return;
            }
            Overlay::RiceSync => {
                // Enter = push selected LIVE → rice
                let indices = self.filtered_indices();
                if let Some(&real) = indices.get(self.selected()) {
                    if let Some(s) = self.rice_status.get(real).cloned() {
                        match rice::push(&self.manifest, Some(&s.id), None) {
                            Ok(n) => {
                                self.rice_status = rice::status(&self.manifest);
                                self.set_msg(format!("pushed {} ({n})", s.id));
                            }
                            Err(e) => self.set_msg(format!("error: {e}")),
                        }
                    }
                }
                return;
            }
            Overlay::WallpaperPicker { files } => {
                let indices = self.filtered_indices();
                let chosen = indices
                    .get(self.selected())
                    .and_then(|&real| files.get(real).cloned());
                if let Some(path) = chosen {
                    match wallpaper::set(&path) {
                        Ok(()) => {
                            self.overlay = Overlay::None;
                            self.set_msg(format!(
                                "wallpaper: {}",
                                path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("?")
                            ));
                        }
                        Err(e) => self.set_msg(format!("error: {e}")),
                    }
                }
                return;
            }
            Overlay::Catalog => {
                let indices = self.filtered_indices();
                if let Some(&real) = indices.get(self.selected()) {
                    if let Some(item) = self.catalog.get(real).cloned() {
                        match catalog::install(&self.manifest, &item.entry.id, true) {
                            Ok(p) => {
                                self.set_msg(format!(
                                    "installed {} → {}",
                                    item.entry.id,
                                    p.display()
                                ));
                                self.refresh_tab(true);
                            }
                            Err(e) => self.set_msg(format!("error: {e}")),
                        }
                    }
                }
                return;
            }
            Overlay::Themes => {
                let indices = self.filtered_indices();
                if let Some(&real) = indices.get(self.selected()) {
                    if let Some(t) = self.themes.get(real).cloned() {
                        match themes::apply(&self.manifest, &t.id, true) {
                            Ok(()) => self.set_msg(format!("theme {} applied", t.id)),
                            Err(e) => self.set_msg(format!("error: {e}")),
                        }
                    }
                }
                return;
            }
            Overlay::Profiles => {
                let indices = self.filtered_indices();
                if let Some(&real) = indices.get(self.selected()) {
                    if let Some(p) = self.profiles.get(real).cloned() {
                        match profiles::apply(&self.manifest, &p.id) {
                            Ok(()) => {
                                self.set_msg(format!("profile {} applied", p.id));
                                self.refresh_tab(true);
                            }
                            Err(e) => self.set_msg(format!("error: {e}")),
                        }
                    }
                }
                return;
            }
            Overlay::Rules => {
                let indices = self.filtered_indices();
                if let Some(&real) = indices.get(self.selected()) {
                    if let Some(r) = self.rules.get(real).cloned() {
                        match rules::toggle(&self.manifest, r.index) {
                            Ok(on) => {
                                self.set_msg(format!(
                                    "rule L{} {}",
                                    r.index + 1,
                                    if on { "enabled" } else { "disabled" }
                                ));
                                self.rules = rules::list(&self.manifest).unwrap_or_default();
                            }
                            Err(e) => self.set_msg(format!("error: {e}")),
                        }
                    }
                }
                return;
            }
            Overlay::None => {}
        }

        let indices = self.filtered_indices();
        let Some(&real) = indices.get(self.selected()) else {
            if self.tab == Tab::Dashboard {
                // jump to doctor if warnings
                if self.checks.iter().any(|c| !c.ok) {
                    self.tab = Tab::Doctor;
                    self.set_selected(0);
                }
            }
            return;
        };

        match self.tab {
            Tab::Services => {
                if let Some(svc) = self.services.get(real).cloned() {
                    if svc.running {
                        self.open_confirm(
                            format!("Stop {}?", svc.id),
                            ConfirmAction::StopService(svc.id),
                        );
                    } else {
                        match service::start_by_id(&self.manifest, &svc.id) {
                            Ok(()) => {
                                self.set_msg(format!("started {}", svc.id));
                                self.refresh_tab(false);
                            }
                            Err(e) => self.set_msg(format!("error: {e}")),
                        }
                    }
                }
            }
            Tab::Packages => {
                if let Some(pkg) = self.packages.get(real) {
                    if pkg.installed {
                        self.open_confirm(
                            format!("Uninstall {}?", pkg.name),
                            ConfirmAction::PkgUninstall(pkg.name.clone()),
                        );
                    } else {
                        self.open_confirm(
                            format!("Install {}?", pkg.name),
                            ConfirmAction::PkgInstall(pkg.name.clone()),
                        );
                    }
                }
            }
            Tab::Configs => self.open_live_config(real),
            Tab::Plugins => {
                if let Some(p) = self.plugins.get(real).cloned() {
                    match plugins::toggle(&self.manifest, &p.id) {
                        Ok(on) => {
                            if self.auto_reload_plugins {
                                let _ = plugins::reload_bar(&self.manifest);
                            }
                            self.set_msg(format!(
                                "{} {}{}",
                                if on { "enabled" } else { "disabled" },
                                p.id,
                                if self.auto_reload_plugins {
                                    " · bar reloaded"
                                } else {
                                    " · press R to reload"
                                }
                            ));
                            self.refresh_tab(false);
                        }
                        Err(e) => self.set_msg(format!("error: {e}")),
                    }
                }
            }
            Tab::Logs => {
                if let Some(src) = self.log_sources.get(real).cloned() {
                    let text = logs::read_tail(&src.path, 500);
                    self.overlay = Overlay::LogView {
                        title: src.label,
                        path: src.path,
                        text,
                        scroll: 0,
                    };
                }
            }
            Tab::Doctor => {
                if let Some(c) = self.checks.get(real) {
                    let detail = c.detail.clone();
                    let name = c.name.clone();
                    if name.starts_with("cfg:") {
                        self.tab = Tab::Configs;
                    } else if name.starts_with("svc:") || name.starts_with("bin:") {
                        self.tab = Tab::Services;
                    } else if name.contains("outdated") {
                        self.tab = Tab::Packages;
                    }
                    self.set_selected(0);
                    self.set_msg(detail);
                }
            }
            Tab::More => self.run_more_label(
                self.more_items
                    .get(real)
                    .copied()
                    .unwrap_or(""),
            ),
            Tab::Dashboard => {}
        }
    }

    fn run_more_label(&mut self, label: &str) {
        if label.starts_with("──") || label.is_empty() {
            return;
        }
        match label {
            "Install CORE rice" => {
                self.open_confirm("Install CORE rice?".into(), ConfirmAction::InstallCore)
            }
            "Install FULL rice" => {
                self.open_confirm("Install FULL rice?".into(), ConfirmAction::InstallFull)
            }
            "Update core stack (brew upgrade + reload)" => self.open_confirm(
                "Upgrade core brew packages + reload?".into(),
                ConfirmAction::UpdateCore,
            ),
            "Apply preset: compact (runtime)" => self.apply_preset("compact"),
            "Apply preset: comfortable (runtime)" => self.apply_preset("comfortable"),
            "Apply preset: spacious (runtime)" => self.apply_preset("spacious"),
            "Wallpaper picker…" => match wallpaper::list(&self.manifest) {
                Ok(files) => {
                    self.overlay = Overlay::WallpaperPicker { files };
                    self.set_selected(0);
                    self.set_msg("Wallpaper · Enter set · Esc back");
                }
                Err(e) => self.set_msg(format!("error: {e}")),
            },
            "Themes…" => {
                let _ = themes::ensure_defaults();
                self.themes = themes::list().unwrap_or_default();
                self.overlay = Overlay::Themes;
                self.set_selected(0);
            }
            "Profiles…" => {
                let _ = profiles::ensure_defaults();
                self.profiles = profiles::list().unwrap_or_default();
                self.overlay = Overlay::Profiles;
                self.set_selected(0);
            }
            "Yabai rules…" => {
                self.rules = rules::list(&self.manifest).unwrap_or_default();
                self.overlay = Overlay::Rules;
                self.set_selected(0);
            }
            "Plugin catalog…" => {
                let _ = catalog::ensure_defaults();
                self.catalog = catalog::list_all().unwrap_or_default();
                self.overlay = Overlay::Catalog;
                self.set_selected(0);
                self.set_msg("Catalog · Enter install · u from git/URL · f pull feeds");
            }
            "Hotkeys CRUD…" => {
                self.hotkeys = crate::core::hotkeys::list(&self.manifest).unwrap_or_default();
                self.overlay = Overlay::Hotkeys;
                self.set_selected(0);
                self.set_msg("Hotkeys · Enter toggle · a add (;; = newline) · d delete · e edit");
            }
            "Backup LIVE configs" => match config::backup_all(&self.manifest) {
                Ok(p) => self.set_msg(format!("backup → {}", p.display())),
                Err(e) => self.set_msg(format!("error: {e}")),
            },
            "Restore latest backup" => self.open_confirm(
                "Restore latest config backup?".into(),
                ConfirmAction::RestoreConfigs,
            ),
            "Snapshots… (named / restore)" => self.open_snapshots(),
            "Undo last restore/import" => {
                let tip = snapshot::undo_available().unwrap_or_else(|| "(empty)".into());
                self.open_confirm(format!("Undo last change → `{tip}`?"), ConfirmAction::Undo);
            }
            "Export rice pack…" => {
                let default = format!(
                    "~/Desktop/macmgr-rice-{}.tar.gz",
                    chrono::Local::now().format("%Y%m%d-%H%M%S")
                );
                self.overlay = Overlay::Prompt {
                    kind: PromptKind::ExportPath,
                    title: "Export path (.tar.gz)".into(),
                    buffer: default,
                };
                self.set_msg("type path · Enter export · Esc cancel");
            }
            "Import rice pack…" => {
                self.overlay = Overlay::Prompt {
                    kind: PromptKind::ImportPath,
                    title: "Import pack path".into(),
                    buffer: "~/Desktop/".into(),
                };
                self.set_msg("type .tar.gz path · Enter · Esc cancel");
            }
            "Rice sync… (LIVE ↔ My rice)" => self.open_rice_sync(),
            "Init rice from LIVE" => match rice::init(&self.manifest, true) {
                Ok(p) => {
                    self.rice_status = rice::status(&self.manifest);
                    self.set_msg(format!("rice init → {}", p.display()));
                }
                Err(e) => self.set_msg(format!("error: {e}")),
            },
            "Toggle watch-mode (auto reload bar)" => self.toggle_watch(),
            "Ensure yabai spaces (9)" => match extras::ensure_spaces(&self.manifest) {
                Ok(()) => self.set_msg("spaces ensured (9)"),
                Err(e) => self.set_msg(format!("error: {e}")),
            },
            "Rice reload" => {
                self.open_confirm("Reload core rice?".into(), ConfirmAction::ReloadRice)
            }
            _ => self.set_msg(format!("unknown More item: {label}")),
        }
    }

    fn apply_preset(&mut self, id: &str) {
        match presets::apply(&self.manifest, id) {
            Ok(()) => self.set_msg(format!("preset {id} applied (runtime until reboot)")),
            Err(e) => self.set_msg(format!("error: {e}")),
        }
    }

    pub fn open_rice_sync(&mut self) {
        if !rice::exists(&self.manifest) {
            self.set_msg("rice missing · More → Init rice from LIVE");
        }
        self.rice_status = rice::status(&self.manifest);
        self.overlay = Overlay::RiceSync;
        self.set_selected(0);
        self.set_msg("Rice sync · Enter/p push · P pull · d diff · i init · Esc");
    }

    pub fn open_snapshots(&mut self) {
        self.snapshots = snapshot::list().unwrap_or_default();
        self.overlay = Overlay::Snapshots;
        self.set_selected(0);
        let undo = snapshot::undo_available()
            .map(|u| format!(" · undo={u}"))
            .unwrap_or_default();
        self.set_msg(format!(
            "Snapshots · Enter restore · n name · u undo · d delete{undo}"
        ));
    }

    pub fn submit_prompt(&mut self) {
        let (kind, buffer) = match &self.overlay {
            Overlay::Prompt { kind, buffer, .. } => (kind.clone(), buffer.clone()),
            _ => return,
        };
        self.overlay = Overlay::None;
        let buffer = buffer.trim().to_string();
        match kind {
            PromptKind::SnapshotName => {
                let label = if buffer.is_empty() { None } else { Some(buffer.as_str()) };
                match snapshot::create(&self.manifest, label, SnapshotKind::Manual) {
                    Ok(info) => {
                        self.snapshots = snapshot::list().unwrap_or_default();
                        self.overlay = Overlay::Snapshots;
                        self.set_msg(format!("snapshot → {}", info.meta.id));
                    }
                    Err(e) => self.set_msg(format!("error: {e}")),
                }
            }
            PromptKind::ExportPath => {
                let path = if buffer.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(&buffer))
                };
                let manifest = self.manifest.clone();
                self.queue_suspend(move || {
                    match pack::export(&manifest, path.as_deref()) {
                        Ok(p) => format!("export → {}\n\nPress Enter…", p.display()),
                        Err(e) => format!("error: {e}\n\nPress Enter…"),
                    }
                });
            }
            PromptKind::ImportPath => {
                if buffer.is_empty() {
                    self.set_msg("import path empty");
                    return;
                }
                self.open_confirm(
                    format!("Import `{buffer}`? (undo first)"),
                    ConfirmAction::ImportPack(buffer),
                );
            }
            PromptKind::HotkeyBinding => {
                let Some((keys, action)) = buffer.split_once(':') else {
                    self.set_msg("format: keys : action");
                    return;
                };
                match hotkeys::add(&self.manifest, keys.trim(), action.trim()) {
                    Ok(n) => {
                        self.hotkeys = hotkeys::list(&self.manifest).unwrap_or_default();
                        self.overlay = Overlay::Hotkeys;
                        self.set_msg(format!("added · {n} bindings"));
                    }
                    Err(e) => self.set_msg(format!("error: {e}")),
                }
            }
            PromptKind::CatalogFromUrl => {
                // id url [path]
                let parts: Vec<_> = buffer.split_whitespace().collect();
                if parts.len() < 2 {
                    self.set_msg("format: id https://… [repo/path.lua]");
                    return;
                }
                let id = parts[0].to_string();
                let url = parts[1].to_string();
                let path = parts.get(2).map(|s| s.to_string());
                let manifest = self.manifest.clone();
                self.queue_suspend(move || {
                    match catalog::install_url(
                        &manifest,
                        &url,
                        &id,
                        "widget",
                        path.as_deref(),
                        true,
                    ) {
                        Ok(p) => format!("installed {id} → {}\n\nPress Enter…", p.display()),
                        Err(e) => format!("error: {e}\n\nPress Enter…"),
                    }
                });
            }
            PromptKind::PkgAny => {
                if buffer.is_empty() {
                    self.set_msg("empty package name");
                    return;
                }
                let (kind, name) = brew::parse_pkg_spec(&buffer);
                self.open_confirm(
                    format!("Install {kind} `{name}` via brew?"),
                    ConfirmAction::PkgInstall(if kind == "cask" {
                        format!("cask:{name}")
                    } else {
                        name
                    }),
                );
            }
            PromptKind::PkgSearchQuery => {
                if buffer.is_empty() {
                    self.set_msg("empty search query");
                    return;
                }
                self.run_pkg_search(&buffer);
            }
        }
    }

    pub fn open_pkg_search_prompt(&mut self) {
        self.overlay = Overlay::Prompt {
            kind: PromptKind::PkgSearchQuery,
            title: "brew search".into(),
            buffer: String::new(),
        };
        self.set_msg("type query · Enter search · Esc");
    }

    pub fn run_pkg_search(&mut self, query: &str) {
        self.set_msg(format!("searching brew for `{query}`…"));
        match brew::search_hits(query) {
            Ok(results) => {
                if results.is_empty() {
                    self.set_msg(format!("no brew hits for `{query}`"));
                    return;
                }
                let n = results.len();
                self.overlay = Overlay::PkgSearch {
                    query: query.to_string(),
                    results,
                };
                self.filter.clear();
                self.set_selected(0);
                self.set_msg(format!("brew search `{query}` · {n} · Enter install · Esc"));
            }
            Err(e) => self.set_msg(format!("error: {e}")),
        }
    }

    pub fn open_palette(&mut self) {
        self.overlay = Overlay::Palette {
            query: String::new(),
        };
        self.filter.clear();
        self.set_selected(0);
        self.set_msg("palette · type to filter · Enter run · Esc");
    }

    pub fn run_palette(&mut self, id: &str) {
        match id {
            "tab:dash" => self.switch_tab(Tab::Dashboard),
            "tab:svc" => self.switch_tab(Tab::Services),
            "tab:pkgs" => self.switch_tab(Tab::Packages),
            "tab:cfg" => self.switch_tab(Tab::Configs),
            "tab:plug" => self.switch_tab(Tab::Plugins),
            "tab:logs" => self.switch_tab(Tab::Logs),
            "tab:doc" => self.switch_tab(Tab::Doctor),
            "tab:more" => self.switch_tab(Tab::More),
            "pkg:search" => self.open_pkg_search_prompt(),
            "pkg:any" => {
                self.overlay = Overlay::Prompt {
                    kind: PromptKind::PkgAny,
                    title: "brew install (name or cask:name)".into(),
                    buffer: String::new(),
                };
                self.set_msg("package name · Enter install · Esc");
            }
            "rice:sync" => self.open_rice_sync(),
            "rice:init" => match rice::init(&self.manifest, true) {
                Ok(p) => self.set_msg(format!("rice init → {}", p.display())),
                Err(e) => self.set_msg(format!("error: {e}")),
            },
            "snap:list" => self.open_snapshots(),
            "snap:create" => {
                self.overlay = Overlay::Prompt {
                    kind: PromptKind::SnapshotName,
                    title: "Snapshot name".into(),
                    buffer: String::new(),
                };
                self.set_msg("name · Enter create · Esc cancel");
            }
            "snap:undo" => {
                let tip = snapshot::undo_available().unwrap_or_else(|| "(empty)".into());
                self.open_confirm(format!("Undo → `{tip}`?"), ConfirmAction::Undo);
            }
            "export" => {
                self.overlay = Overlay::Prompt {
                    kind: PromptKind::ExportPath,
                    title: "Export path (.tar.gz)".into(),
                    buffer: String::new(),
                };
                self.set_msg("type path · Enter export · Esc cancel");
            }
            "import" => {
                self.overlay = Overlay::Prompt {
                    kind: PromptKind::ImportPath,
                    title: "Import .tar.gz path".into(),
                    buffer: String::new(),
                };
                self.set_msg("type .tar.gz path · Enter · Esc cancel");
            }
            "hotkeys" => {
                self.hotkeys = hotkeys::list(&self.manifest).unwrap_or_default();
                self.overlay = Overlay::Hotkeys;
                self.set_selected(0);
                self.set_msg("Hotkeys · Enter toggle · a add (;; = newline) · d delete · e edit");
            }
            "catalog" => {
                let _ = catalog::ensure_defaults();
                self.catalog = catalog::list_all().unwrap_or_default();
                self.overlay = Overlay::Catalog;
                self.set_selected(0);
                self.set_msg("Catalog · Enter install · u URL · f pull feeds");
            }
            "catalog:pull" => match catalog::pull_feeds() {
                Ok(r) => {
                    self.catalog = catalog::list_all().unwrap_or_default();
                    self.set_msg(format!(
                        "feeds pulled · {} plugins from {} feed(s)",
                        r.plugins, r.feeds
                    ));
                }
                Err(e) => self.set_msg(format!("error: {e}")),
            },
            "themes" => {
                self.themes = themes::list().unwrap_or_default();
                self.overlay = Overlay::Themes;
                self.set_selected(0);
            }
            "profiles" => {
                self.profiles = profiles::list().unwrap_or_default();
                self.overlay = Overlay::Profiles;
                self.set_selected(0);
            }
            "rules" => {
                self.rules = rules::list(&self.manifest).unwrap_or_default();
                self.overlay = Overlay::Rules;
                self.set_selected(0);
            }
            "reload:bar" => match plugins::reload_bar(&self.manifest) {
                Ok(()) => self.set_msg("sketchybar reloaded"),
                Err(e) => self.set_msg(format!("error: {e}")),
            },
            "reload:rice" => {
                self.open_confirm("Reload core rice services?".into(), ConfirmAction::ReloadRice);
            }
            "watch" => self.toggle_watch(),
            "doctor" => {
                self.switch_tab(Tab::Doctor);
                self.refresh_tab(true);
            }
            "help" => {
                self.overlay = Overlay::Help;
                self.dirty = true;
            }
            _ => self.set_msg(format!("unknown palette cmd: {id}")),
        }
    }

    fn toggle_watch(&mut self) {
        if self.watch.is_some() {
            if let Some(w) = self.watch.take() {
                w.stop();
            }
            self.set_msg("watch-mode OFF");
        } else {
            match watch::start(&self.manifest, 800) {
                Ok(h) => {
                    self.watch = Some(h);
                    self.set_msg("watch-mode ON · auto reload sketchybar on .lua changes");
                }
                Err(e) => self.set_msg(format!("error: {e}")),
            }
        }
    }

    pub fn open_live_config(&mut self, real_idx: usize) {
        let Some(cfg) = self.configs.get(real_idx).cloned() else {
            return;
        };
        if !cfg.exists {
            self.set_msg(format!("missing: {}", cfg.path.display()));
            return;
        }
        if cfg.is_dir {
            match config::list_live_files(&self.manifest, &cfg.id) {
                Ok(files) => {
                    self.overlay = Overlay::FileBrowser {
                        config_id: cfg.id,
                        root: cfg.path,
                        files,
                    };
                    self.set_selected(0);
                    self.set_msg("LIVE files · Enter/e → nano · Esc back");
                }
                Err(e) => self.set_msg(format!("error: {e}")),
            }
        } else {
            self.open_external_path(cfg.path);
        }
    }

    pub fn add_plugin_stub(&mut self, kind: PluginKind) {
        let name = format!("custom{}", self.plugins.len() + 1);
        match plugins::add(&self.manifest, &name, kind) {
            Ok(path) => {
                if self.auto_reload_plugins {
                    let _ = plugins::reload_bar(&self.manifest);
                }
                self.set_msg(format!("added {} → {}", name, path.display()));
                self.refresh_tab(false);
            }
            Err(e) => self.set_msg(format!("error: {e}")),
        }
    }

    pub fn show_diff(&mut self, config_id: Option<&str>) {
        match rice::diff(&self.manifest, config_id, None) {
            Ok(text) => {
                self.overlay = Overlay::DiffView {
                    title: format!("LIVE↔rice · {}", config_id.unwrap_or("all")),
                    text,
                    scroll: 0,
                };
            }
            Err(e) => self.set_msg(format!("error: {e}")),
        }
    }
}
