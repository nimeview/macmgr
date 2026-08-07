use crossterm::event::{KeyCode, KeyModifiers};

use crate::core::config;
use crate::core::catalog;
use crate::core::plugins::{self, PluginKind};
use crate::core::rice;
use crate::core::snapshot;
use crate::util::text::StatusKind;

use super::state::{App, ConfirmAction, Overlay, PromptKind, Tab};

impl App {
    pub fn on_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // Filter input mode
        if matches!(self.overlay, Overlay::Filter) {
            match code {
                KeyCode::Esc => {
                    self.filter.clear();
                    self.overlay = Overlay::None;
                    self.set_msg(self.tab.hint());
                }
                KeyCode::Enter => {
                    self.overlay = Overlay::None;
                    self.set_selected(0);
                    self.set_msg(format!(
                        "filter: {}",
                        if self.filter.is_empty() {
                            "(none)"
                        } else {
                            &self.filter
                        }
                    ));
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                    self.dirty = true;
                }
                KeyCode::Char(c) if !modifiers.contains(KeyModifiers::CONTROL) => {
                    self.filter.push(c);
                    self.dirty = true;
                }
                _ => {}
            }
            return;
        }

        // Command palette (`:`)
        if let Overlay::Palette { .. } = &self.overlay {
            match code {
                KeyCode::Esc => self.close_overlay(),
                KeyCode::Enter => self.on_enter(),
                KeyCode::Down => self.move_sel(1),
                KeyCode::Up => self.move_sel(-1),
                KeyCode::Char('j') if modifiers.contains(KeyModifiers::CONTROL) => {
                    self.move_sel(1);
                }
                KeyCode::Char('k') if modifiers.contains(KeyModifiers::CONTROL) => {
                    self.move_sel(-1);
                }
                KeyCode::Backspace => {
                    if let Overlay::Palette { query, .. } = &mut self.overlay {
                        query.pop();
                    }
                    self.set_selected(0);
                    self.dirty = true;
                }
                KeyCode::Char(c) if !modifiers.contains(KeyModifiers::CONTROL) => {
                    if let Overlay::Palette { query, .. } = &mut self.overlay {
                        query.push(c);
                    }
                    self.set_selected(0);
                    self.dirty = true;
                }
                _ => {}
            }
            return;
        }

        // Prompt input (snapshot name / export / import path)
        if matches!(self.overlay, Overlay::Prompt { .. }) {
            match code {
                KeyCode::Esc => {
                    self.overlay = Overlay::None;
                    self.set_msg("cancelled");
                }
                KeyCode::Enter => self.submit_prompt(),
                KeyCode::Backspace => {
                    if let Overlay::Prompt { buffer, .. } = &mut self.overlay {
                        buffer.pop();
                    }
                    self.dirty = true;
                }
                KeyCode::Char(c) if !modifiers.contains(KeyModifiers::CONTROL) => {
                    if let Overlay::Prompt { buffer, .. } = &mut self.overlay {
                        buffer.push(c);
                    }
                    self.dirty = true;
                }
                _ => {}
            }
            return;
        }

        // Overlay keys
        match &self.overlay {
            Overlay::Help => {
                if matches!(code, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')) {
                    self.overlay = Overlay::None;
                    self.set_msg(self.tab.hint());
                }
                return;
            }
            Overlay::Snapshots => {
                match code {
                    KeyCode::Esc => self.close_overlay(),
                    KeyCode::Enter => self.on_enter(),
                    KeyCode::Down | KeyCode::Char('j') => self.move_sel(1),
                    KeyCode::Up | KeyCode::Char('k') => self.move_sel(-1),
                    KeyCode::Char('/') => {
                        self.overlay = Overlay::Filter;
                        self.set_msg("filter: (type + Enter)");
                    }
                    KeyCode::Char('n') => {
                        self.overlay = Overlay::Prompt {
                            kind: PromptKind::SnapshotName,
                            title: "Snapshot name".into(),
                            buffer: String::new(),
                        };
                        self.set_msg("name · Enter create · Esc cancel");
                    }
                    KeyCode::Char('u') => {
                        let tip =
                            snapshot::undo_available().unwrap_or_else(|| "(empty)".into());
                        self.open_confirm(
                            format!("Undo → `{tip}`?"),
                            ConfirmAction::Undo,
                        );
                    }
                    KeyCode::Char('d') => {
                        let indices = self.filtered_indices();
                        if let Some(&real) = indices.get(self.selected()) {
                            if let Some(s) = self.snapshots.get(real) {
                                let id = s.meta.id.clone();
                                self.open_confirm(
                                    format!("Delete snapshot `{id}`?"),
                                    ConfirmAction::DeleteSnapshot(id),
                                );
                            }
                        }
                    }
                    _ => {}
                }
                return;
            }
            Overlay::PkgSearch { .. } => {
                match code {
                    KeyCode::Esc => self.close_overlay(),
                    KeyCode::Enter => self.on_enter(),
                    KeyCode::Down | KeyCode::Char('j') => self.move_sel(1),
                    KeyCode::Up | KeyCode::Char('k') => self.move_sel(-1),
                    KeyCode::Char('s') => self.open_pkg_search_prompt(),
                    _ => {}
                }
                return;
            }
            Overlay::RiceSync => {
                match code {
                    KeyCode::Esc => self.close_overlay(),
                    KeyCode::Enter | KeyCode::Char('p') => self.on_enter(),
                    KeyCode::Down | KeyCode::Char('j') => self.move_sel(1),
                    KeyCode::Up | KeyCode::Char('k') => self.move_sel(-1),
                    KeyCode::Char('/') => {
                        self.overlay = Overlay::Filter;
                        self.set_msg("filter: (type + Enter)");
                    }
                    KeyCode::Char('P') => {
                        let indices = self.filtered_indices();
                        if let Some(&real) = indices.get(self.selected()) {
                            if let Some(s) = self.rice_status.get(real) {
                                let id = s.id.clone();
                                self.open_confirm(
                                    format!("Pull rice → LIVE `{id}`? (undo first)"),
                                    ConfirmAction::RicePull { id: Some(id) },
                                );
                            }
                        }
                    }
                    KeyCode::Char('d') => {
                        let indices = self.filtered_indices();
                        if let Some(&real) = indices.get(self.selected()) {
                            if let Some(s) = self.rice_status.get(real) {
                                match rice::diff(&self.manifest, Some(&s.id), None) {
                                    Ok(text) => {
                                        self.overlay = Overlay::DiffView {
                                            title: format!("rice · {}", s.id),
                                            text,
                                            scroll: 0,
                                        };
                                    }
                                    Err(e) => self.set_msg(format!("error: {e}")),
                                }
                            }
                        }
                    }
                    KeyCode::Char('i') => match rice::init(&self.manifest, true) {
                        Ok(p) => {
                            self.rice_status = rice::status(&self.manifest);
                            self.set_msg(format!("rice init → {}", p.display()));
                        }
                        Err(e) => self.set_msg(format!("error: {e}")),
                    },
                    KeyCode::Char('a') => {
                        // push all
                        match rice::push(&self.manifest, None, None) {
                            Ok(n) => {
                                self.rice_status = rice::status(&self.manifest);
                                self.set_msg(format!("pushed all ({n})"));
                            }
                            Err(e) => self.set_msg(format!("error: {e}")),
                        }
                    }
                    KeyCode::Char('A') => self.open_confirm(
                        "Pull ALL rice → LIVE?".into(),
                        ConfirmAction::RicePullAll,
                    ),
                    _ => {}
                }
                return;
            }
            Overlay::FileBrowser { .. }
            | Overlay::Themes
            | Overlay::Profiles
            | Overlay::Rules
            | Overlay::WallpaperPicker { .. } => {
                match code {
                    KeyCode::Esc => self.close_overlay(),
                    KeyCode::Enter => self.on_enter(),
                    KeyCode::Down | KeyCode::Char('j') => self.move_sel(1),
                    KeyCode::Up | KeyCode::Char('k') => self.move_sel(-1),
                    KeyCode::Char('/') => {
                        self.overlay = Overlay::Filter;
                        self.set_msg("filter: (type + Enter)");
                    }
                    KeyCode::Char('e') | KeyCode::Char('E')
                        if matches!(self.overlay, Overlay::FileBrowser { .. }) =>
                    {
                        if let Overlay::FileBrowser { files, .. } = &self.overlay {
                            if let Some(path) = files.get(self.selected()).cloned() {
                                self.open_external_path(path);
                            }
                        }
                    }
                    _ => {}
                }
                return;
            }
            Overlay::Hotkeys => {
                match code {
                    KeyCode::Esc => self.close_overlay(),
                    KeyCode::Enter => self.on_enter(),
                    KeyCode::Down | KeyCode::Char('j') => self.move_sel(1),
                    KeyCode::Up | KeyCode::Char('k') => self.move_sel(-1),
                    KeyCode::Char('/') => {
                        self.overlay = Overlay::Filter;
                        self.set_msg("filter: (type + Enter)");
                    }
                    KeyCode::Char('a') => {
                        self.overlay = Overlay::Prompt {
                            kind: PromptKind::HotkeyBinding,
                            title: "New hotkey (keys : action)".into(),
                            buffer: "alt - x : ".into(),
                        };
                        self.set_msg("type binding · ;; for multi-line · Enter save · Esc");
                    }
                    KeyCode::Char('d') => {
                        let indices = self.filtered_indices();
                        if let Some(&real) = indices.get(self.selected()) {
                            let n = real + 1;
                            self.open_confirm(
                                format!("Delete hotkey #{n}?"),
                                ConfirmAction::DeleteHotkey(n),
                            );
                        }
                    }
                    KeyCode::Char('e') | KeyCode::Char('E') => {
                        if let Some(cfg) = self.manifest.find_config("skhd") {
                            let path = crate::core::paths::expand_tilde(&cfg.path);
                            self.open_external_path(path);
                        }
                    }
                    _ => {}
                }
                return;
            }
            Overlay::Catalog => {
                match code {
                    KeyCode::Esc => self.close_overlay(),
                    KeyCode::Enter => self.on_enter(),
                    KeyCode::Down | KeyCode::Char('j') => self.move_sel(1),
                    KeyCode::Up | KeyCode::Char('k') => self.move_sel(-1),
                    KeyCode::Char('/') => {
                        self.overlay = Overlay::Filter;
                        self.set_msg("filter: (type + Enter)");
                    }
                    KeyCode::Char('u') => {
                        self.overlay = Overlay::Prompt {
                            kind: PromptKind::CatalogFromUrl,
                            title: "Install plugin: id url [path.lua]".into(),
                            buffer: String::new(),
                        };
                        self.set_msg("id https://…git [path] · Enter");
                    }
                    KeyCode::Char('f') | KeyCode::Char('p') => {
                        match catalog::pull_feeds() {
                            Ok(r) => {
                                self.catalog = catalog::list_all().unwrap_or_default();
                                self.set_selected(0);
                                self.set_msg(format!(
                                    "feeds pulled · {} plugins from {} feed(s)",
                                    r.plugins, r.feeds
                                ));
                            }
                            Err(e) => self.set_msg(format!("error: {e}")),
                        }
                    }
                    _ => {}
                }
                return;
            }
            Overlay::LogView { path, .. } => {
                match code {
                    KeyCode::Esc => self.close_overlay(),
                    KeyCode::Char('c') => {
                        let path = path.clone();
                        self.open_confirm(
                            "Clear this log?".into(),
                            ConfirmAction::ClearLog(path),
                        );
                    }
                    KeyCode::Down | KeyCode::Char('j') => self.move_sel(1),
                    KeyCode::Up | KeyCode::Char('k') => self.move_sel(-1),
                    _ => {}
                }
                return;
            }
            Overlay::DiffView { .. } => {
                match code {
                    KeyCode::Esc | KeyCode::Char('q') => self.close_overlay(),
                    KeyCode::Down | KeyCode::Char('j') => self.move_sel(1),
                    KeyCode::Up | KeyCode::Char('k') => self.move_sel(-1),
                    _ => {}
                }
                return;
            }
            Overlay::Confirm { .. } => {
                match code {
                    KeyCode::Esc | KeyCode::Char('n') => self.close_overlay(),
                    KeyCode::Enter | KeyCode::Char('y') => self.on_enter(),
                    KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Char('h')
                    | KeyCode::Char('l')
                    | KeyCode::Tab => self.move_sel(1),
                    _ => {}
                }
                return;
            }
            Overlay::None | Overlay::Filter | Overlay::Prompt { .. } | Overlay::Palette { .. } => {}
        }

        // Main
        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') => {
                self.overlay = Overlay::Help;
                self.dirty = true;
            }
            KeyCode::Char(':') => {
                self.open_palette();
            }
            KeyCode::Char('/')
                if matches!(
                    self.tab,
                    Tab::Packages | Tab::Plugins | Tab::Configs | Tab::Logs
                ) =>
            {
                self.overlay = Overlay::Filter;
                self.set_msg("filter: (type + Enter, Esc clear)");
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                app_switch_next(self);
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                app_switch_prev(self);
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_sel(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_sel(-1),
            KeyCode::Enter | KeyCode::Char(' ') => self.on_enter(),
            KeyCode::Char('r') => {
                self.set_msg("refreshing…");
                self.status_kind = StatusKind::Info;
                self.dirty = true;
                self.refresh_tab(true);
                self.set_msg("refreshed");
            }
            KeyCode::Char('R') if self.tab == Tab::Plugins => {
                match plugins::reload_bar(&self.manifest) {
                    Ok(()) => self.set_msg("sketchybar reloaded"),
                    Err(e) => self.set_msg(format!("error: {e}")),
                }
            }
            KeyCode::Char('R') => {
                self.open_confirm("Reload core rice services?".into(), ConfirmAction::ReloadRice);
            }
            KeyCode::Char('e') | KeyCode::Char('E') if self.tab == Tab::Configs => {
                let indices = self.filtered_indices();
                if let Some(&real) = indices.get(self.selected()) {
                    if let Some(cfg) = self.configs.get(real).cloned() {
                        if cfg.is_dir {
                            // browse first; files open in nano via Enter/e
                            self.open_live_config(real);
                        } else {
                            self.open_external_config(&cfg.id);
                        }
                    }
                }
            }
            KeyCode::Char('e') | KeyCode::Char('E') if self.tab == Tab::Plugins => {
                let indices = self.filtered_indices();
                if let Some(&real) = indices.get(self.selected()) {
                    if let Some(p) = self.plugins.get(real).cloned() {
                        self.open_external_path(p.path);
                    }
                }
            }
            KeyCode::Char('o') if self.tab == Tab::Configs => {
                let indices = self.filtered_indices();
                if let Some(&real) = indices.get(self.selected()) {
                    if let Some(cfg) = self.configs.get(real) {
                        match config::open(&self.manifest, &cfg.id) {
                            Ok(()) => self.set_msg(format!("revealed {}", cfg.id)),
                            Err(e) => self.set_msg(format!("error: {e}")),
                        }
                    }
                }
            }
            KeyCode::Char('s') if self.tab == Tab::Configs => {
                self.open_rice_sync();
            }
            KeyCode::Char('d') if self.tab == Tab::Configs => {
                let indices = self.filtered_indices();
                let id = indices
                    .get(self.selected())
                    .and_then(|&i| self.configs.get(i))
                    .map(|c| c.id.clone());
                self.show_diff(id.as_deref());
            }
            KeyCode::Char('d') if self.tab == Tab::Plugins => {
                let indices = self.filtered_indices();
                if let Some(&real) = indices.get(self.selected()) {
                    if let Some(p) = self.plugins.get(real).cloned() {
                        match plugins::disable(&self.manifest, &p.id) {
                            Ok(()) => {
                                if self.auto_reload_plugins {
                                    let _ = plugins::reload_bar(&self.manifest);
                                }
                                self.set_msg(format!("disabled {}", p.id));
                                self.refresh_tab(false);
                            }
                            Err(e) => self.set_msg(format!("error: {e}")),
                        }
                    }
                }
            }
            KeyCode::Char('a') if self.tab == Tab::Plugins => {
                self.add_plugin_stub(PluginKind::Widget);
            }
            KeyCode::Char('A') if self.tab == Tab::Plugins => {
                self.add_plugin_stub(PluginKind::Item);
            }
            KeyCode::Char('i') if self.tab == Tab::Plugins => {
                let _ = crate::core::catalog::ensure_defaults();
                self.catalog = crate::core::catalog::list_all().unwrap_or_default();
                self.overlay = Overlay::Catalog;
                self.set_selected(0);
                self.set_msg("Catalog · Enter install · u URL · f pull feeds");
            }
            KeyCode::Char('a') if self.tab == Tab::Packages => {
                self.overlay = Overlay::Prompt {
                    kind: PromptKind::PkgAny,
                    title: "brew install (name or cask:name)".into(),
                    buffer: String::new(),
                };
                self.set_msg("package name · Enter install · Esc");
            }
            KeyCode::Char('s') if self.tab == Tab::Packages => {
                self.open_pkg_search_prompt();
            }
            KeyCode::Char('u') if self.tab == Tab::Packages => {
                let indices = self.filtered_indices();
                if let Some(&real) = indices.get(self.selected()) {
                    if let Some(pkg) = self.packages.get(real) {
                        self.open_confirm(
                            format!("Upgrade {}?", pkg.name),
                            ConfirmAction::PkgUpgrade(pkg.name.clone()),
                        );
                    }
                }
            }
            KeyCode::Char('b') if self.tab == Tab::Configs => {
                match config::backup_all(&self.manifest) {
                    Ok(p) => self.set_msg(format!("backup → {}", p.display())),
                    Err(e) => self.set_msg(format!("error: {e}")),
                }
            }
            KeyCode::Char(c @ '1'..='8') => {
                let idx = (c as u8 - b'1') as usize;
                if let Some(tab) = Tab::all().get(idx).copied() {
                    self.switch_tab(tab);
                }
            }
            _ => {}
        }
    }
}

fn app_switch_next(app: &mut App) {
    let tab = app.tab.next();
    app.switch_tab(tab);
}

fn app_switch_prev(app: &mut App) {
    let tab = app.tab.prev();
    app.switch_tab(tab);
}
