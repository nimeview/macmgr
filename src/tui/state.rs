use std::path::PathBuf;
use std::time::{Duration, Instant};

use ratatui::widgets::ListState;

use crate::core::brew::{self, BrewPkgInfo};
use crate::core::config::{self, ConfigInfo};
use crate::core::doctor::{self, Check};
use crate::core::hotkeys::Hotkey;
use crate::core::install::InstallMode;
use crate::core::logs::{self, LogSource};
use crate::core::manifest::Manifest;
use crate::core::plugins::{self, Plugin};
use crate::core::rules::YabaiRule;
use crate::core::service::{self, ServiceStatus};
use crate::core::watch::WatchHandle;
use crate::util::text::StatusKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Services,
    Packages,
    Configs,
    Plugins,
    Logs,
    Doctor,
    More,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[
            Tab::Dashboard,
            Tab::Services,
            Tab::Packages,
            Tab::Configs,
            Tab::Plugins,
            Tab::Logs,
            Tab::Doctor,
            Tab::More,
        ]
    }

    pub fn title(self) -> &'static str {
        match self {
            Tab::Dashboard => "Dash",
            Tab::Services => "Svc",
            Tab::Packages => "Pkgs",
            Tab::Configs => "Cfg",
            Tab::Plugins => "Plug",
            Tab::Logs => "Logs",
            Tab::Doctor => "Doc",
            Tab::More => "More",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Tab::Dashboard => "? help · : palette · 1-8 tabs · r refresh",
            Tab::Services => "Enter toggle · R reload rice",
            Tab::Packages => "Enter install · a any · t track · x untrack · s search · u upgrade",
            Tab::Configs => "Enter/e nano · a add · x remove · s rice · o Finder · d diff",
            Tab::Plugins => "Enter toggle · e nano · a add · i catalog · / filter",
            Tab::Logs => "Enter view · / filter",
            Tab::Doctor => "Enter jump · r refresh",
            Tab::More => "Enter run · themes/profiles/watch/rules/…",
        }
    }

    pub fn next(self) -> Tab {
        let all = Self::all();
        let i = all.iter().position(|t| *t == self).unwrap_or(0);
        all[(i + 1) % all.len()]
    }

    pub fn prev(self) -> Tab {
        let all = Self::all();
        let i = all.iter().position(|t| *t == self).unwrap_or(0);
        all[(i + all.len() - 1) % all.len()]
    }
}

#[derive(Debug, Clone)]
pub enum PromptKind {
    SnapshotName,
    ExportPath,
    ImportPath,
    /// Full binding: `alt - x : yabai -m …`
    HotkeyBinding,
    /// `id https://…` or `id git-url [path]`
    CatalogFromUrl,
    /// Any brew package: `ripgrep` or `cask:firefox`
    PkgAny,
    /// Track only (no install): same spec as PkgAny
    PkgTrack,
    /// Brew search query → opens PkgSearch overlay
    PkgSearchQuery,
    /// Config track: `id path` or bare path
    ConfigAdd,
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    PkgInstall(String),
    PkgUninstall(String),
    PkgUpgrade(String),
    PkgUntrack { name: String, force: bool },
    ConfigUntrack { id: String, force: bool },
    InstallCore,
    InstallFull,
    PurgeEnv,
    ReloadRice,
    RestoreConfigs,
    RestoreSnapshot(String),
    DeleteSnapshot(String),
    Undo,
    ImportPack(String),
    RicePull { id: Option<String> },
    RicePullAll,
    DeleteHotkey(usize),
    UpdateCore,
    ClearLog(PathBuf),
    StopService(String),
}

#[derive(Debug, Clone)]
pub enum Overlay {
    None,
    Help,
    Filter,
    Prompt {
        kind: PromptKind,
        title: String,
        buffer: String,
    },
    /// Command palette (`:`)
    Palette {
        query: String,
    },
    /// Results from `brew search`
    PkgSearch {
        query: String,
        results: Vec<crate::core::brew::BrewSearchHit>,
    },
    FileBrowser {
        config_id: String,
        root: PathBuf,
        files: Vec<PathBuf>,
    },
    LogView {
        title: String,
        path: PathBuf,
        text: String,
        scroll: usize,
    },
    DiffView {
        title: String,
        text: String,
        scroll: usize,
    },
    Hotkeys,
    WallpaperPicker {
        files: Vec<PathBuf>,
    },
    Catalog,
    Themes,
    Profiles,
    Rules,
    Snapshots,
    RiceSync,
    Confirm {
        message: String,
        action: ConfirmAction,
        yes: bool,
        return_selected: usize,
    },
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub text: String,
    pub kind: StatusKind,
    pub until: Instant,
}

pub struct App {
    pub manifest: Manifest,
    pub tab: Tab,
    pub services: Vec<ServiceStatus>,
    pub packages: Vec<BrewPkgInfo>,
    pub configs: Vec<ConfigInfo>,
    pub plugins: Vec<Plugin>,
    pub checks: Vec<Check>,
    pub hotkeys: Vec<Hotkey>,
    pub log_sources: Vec<LogSource>,
    pub rules: Vec<YabaiRule>,
    pub catalog: Vec<crate::core::catalog::CatalogItem>,
    pub themes: Vec<crate::core::themes::Theme>,
    pub profiles: Vec<crate::core::profiles::Profile>,
    pub snapshots: Vec<crate::core::snapshot::SnapshotInfo>,
    pub rice_status: Vec<crate::core::rice::RiceStatus>,
    pub more_items: Vec<&'static str>,
    pub list_state: ListState,
    pub overlay: Overlay,
    pub message: String,
    pub status_kind: StatusKind,
    pub filter: String,
    pub should_quit: bool,
    pub pending: Option<Box<dyn FnOnce() -> String + Send>>,
    /// Like pending, but returns to TUI immediately after (no "Press Enter").
    pub pending_editor: Option<Box<dyn FnOnce() -> String + Send>>,
    pub brew_cached_at: Option<Instant>,
    pub services_cached_at: Option<Instant>,
    pub auto_reload_plugins: bool,
    pub watch: Option<WatchHandle>,
    pub last_viewport: usize,
    /// Redraw only when true (event-driven UI).
    pub dirty: bool,
    pub toasts: Vec<Toast>,
    pub last_log_poll: Instant,
    /// Last seen byte-len per err log path (change detection).
    pub log_sizes: std::collections::HashMap<String, u64>,
}

impl App {
    pub fn new(manifest: Manifest) -> Self {
        let mut app = Self {
            manifest,
            tab: Tab::Dashboard,
            services: Vec::new(),
            packages: Vec::new(),
            configs: Vec::new(),
            plugins: Vec::new(),
            checks: Vec::new(),
            hotkeys: Vec::new(),
            log_sources: logs::sources(),
            rules: Vec::new(),
            catalog: Vec::new(),
            themes: Vec::new(),
            profiles: Vec::new(),
            snapshots: Vec::new(),
            rice_status: Vec::new(),
            more_items: super::more_menu::labels(),
            list_state: ListState::default(),
            overlay: Overlay::None,
            message: Tab::Dashboard.hint().into(),
            status_kind: StatusKind::Info,
            filter: String::new(),
            should_quit: false,
            pending: None,
            pending_editor: None,
            brew_cached_at: None,
            services_cached_at: None,
            auto_reload_plugins: true,
            watch: None,
            last_viewport: 20,
            dirty: true,
            toasts: Vec::new(),
            last_log_poll: Instant::now(),
            log_sizes: std::collections::HashMap::new(),
        };
        app.list_state.select(Some(0));
        app.bootstrap_light();
        app
    }

    /// Minimal startup — no brew/doctor.
    pub fn bootstrap_light(&mut self) {
        self.services = service::list_statuses_fast(&self.manifest);
        self.services_cached_at = Some(Instant::now());
        self.configs = config::list_configs_fast(&self.manifest);
        self.plugins = plugins::list(&self.manifest).unwrap_or_default();
        self.log_sources = logs::sources();
        self.dirty = true;
    }

    pub fn selected(&self) -> usize {
        self.list_state.selected().unwrap_or(0)
    }

    pub fn set_selected(&mut self, idx: usize) {
        self.list_state.select(Some(idx));
        self.dirty = true;
    }

    pub fn set_msg(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        self.status_kind = crate::util::text::status_severity(&msg);
        self.message = msg.clone();
        // Ambient toast for errors / successes (not every info hint)
        if matches!(self.status_kind, StatusKind::Error | StatusKind::Ok)
            && !msg.starts_with("filter:")
            && msg != "cancelled"
            && msg != "refreshed"
        {
            self.push_toast(msg, self.status_kind, 4);
        }
        self.dirty = true;
    }

    pub fn push_toast(&mut self, text: impl Into<String>, kind: StatusKind, secs: u64) {
        let text = text.into();
        if text.is_empty() {
            return;
        }
        self.toasts.push(Toast {
            text,
            kind,
            until: Instant::now() + Duration::from_secs(secs),
        });
        // Keep only last 3
        if self.toasts.len() > 3 {
            let n = self.toasts.len() - 3;
            self.toasts.drain(0..n);
        }
        self.dirty = true;
    }

    /// Expire toasts; return true if UI should redraw.
    pub fn tick_toasts(&mut self) -> bool {
        let before = self.toasts.len();
        self.toasts.retain(|t| t.until > Instant::now());
        if self.toasts.len() != before {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Watch stderr logs for new error lines → toast.
    pub fn poll_log_errors(&mut self) {
        self.last_log_poll = Instant::now();
        let mut alerts: Vec<(String, String)> = Vec::new();
        for src in &self.log_sources {
            if !src.id.contains("err") {
                continue;
            }
            if !src.path.is_file() {
                continue;
            }
            let meta = match std::fs::metadata(&src.path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let len = meta.len();
            let key = src.path.display().to_string();
            let prev = self.log_sizes.get(&key).copied().unwrap_or(0);
            if len <= prev {
                self.log_sizes.insert(key, len);
                continue;
            }
            let tail = logs::read_tail(&src.path, 8);
            self.log_sizes.insert(key, len);
            if prev == 0 && len > 0 {
                continue;
            }
            let interesting = tail.lines().rev().find(|l| {
                let l = l.to_lowercase();
                !l.trim().is_empty()
                    && (l.contains("error")
                        || l.contains("failed")
                        || l.contains("fatal")
                        || l.contains("panic")
                        || l.contains("cannot"))
            });
            if let Some(line) = interesting {
                alerts.push((
                    src.id.clone(),
                    format!(
                        "{}: {}",
                        src.label,
                        crate::util::text::truncate_chars(line.trim(), 70)
                    ),
                ));
            }
        }
        for (id, msg) in alerts {
            self.push_toast(msg, StatusKind::Error, 6);
            self.status_kind = StatusKind::Error;
            self.message = format!("log alert · see Logs / {id}");
            self.dirty = true;
        }
    }

    fn services_stale(&self) -> bool {
        self.services_cached_at
            .map(|t| t.elapsed() > Duration::from_secs(3))
            .unwrap_or(true)
    }

    fn brew_stale(&self) -> bool {
        self.brew_cached_at
            .map(|t| t.elapsed() > Duration::from_secs(60))
            .unwrap_or(true)
    }

    /// Load only data needed for the current tab.
    pub fn ensure_tab_data(&mut self) {
        match self.tab {
            Tab::Dashboard | Tab::Services => {
                if self.services_stale() {
                    self.services = service::list_statuses_fast(&self.manifest);
                    self.services_cached_at = Some(Instant::now());
                }
            }
            Tab::Packages => {
                if self.packages.is_empty() || self.brew_stale() {
                    self.packages = brew::package_infos(&self.manifest);
                    self.brew_cached_at = Some(Instant::now());
                }
            }
            Tab::Configs => {
                if self.configs.is_empty() {
                    self.configs = config::list_configs_fast(&self.manifest);
                }
            }
            Tab::Plugins => {
                if self.plugins.is_empty() {
                    self.plugins = plugins::list(&self.manifest).unwrap_or_default();
                }
            }
            Tab::Logs => {
                self.log_sources = logs::sources();
            }
            Tab::Doctor => {
                if self.checks.is_empty() {
                    self.checks = doctor::run_checks(&self.manifest);
                }
            }
            Tab::More => {}
        }
        self.clamp_selected();
    }

    /// Explicit `r` refresh — heavier, scoped to current tab.
    pub fn refresh_tab(&mut self, force: bool) {
        if !force {
            self.ensure_tab_data();
            self.dirty = true;
            return;
        }
        match self.tab {
            Tab::Services => {
                self.services = service::list_statuses_full(&self.manifest);
                self.services_cached_at = Some(Instant::now());
            }
            Tab::Dashboard => {
                self.services = service::list_statuses_full(&self.manifest);
                self.services_cached_at = Some(Instant::now());
                self.configs = config::list_configs_fast(&self.manifest);
                self.plugins = plugins::list(&self.manifest).unwrap_or_default();
                self.checks = doctor::run_checks(&self.manifest);
                self.packages = brew::package_infos(&self.manifest);
                self.brew_cached_at = Some(Instant::now());
            }
            Tab::Packages => {
                self.packages = brew::package_infos(&self.manifest);
                self.brew_cached_at = Some(Instant::now());
            }
            Tab::Configs => {
                self.configs = config::list_configs_with_sync(&self.manifest, true);
            }
            Tab::Plugins => {
                self.plugins = plugins::list(&self.manifest).unwrap_or_default();
            }
            Tab::Logs => {
                self.log_sources = logs::sources();
            }
            Tab::Doctor => {
                self.checks = doctor::run_checks(&self.manifest);
                self.services = service::list_statuses_full(&self.manifest);
                self.services_cached_at = Some(Instant::now());
            }
            Tab::More => {
                self.themes = crate::core::themes::list().unwrap_or_default();
                self.profiles = crate::core::profiles::list().unwrap_or_default();
                self.catalog = crate::core::catalog::list_all().unwrap_or_default();
                self.rules = crate::core::rules::list(&self.manifest).unwrap_or_default();
            }
        }
        self.clamp_selected();
        self.dirty = true;
    }

    pub fn filtered_indices(&self) -> Vec<usize> {
        let q = self.filter.to_lowercase();
        let match_q = |s: &str| q.is_empty() || s.to_lowercase().contains(&q);
        match &self.overlay {
            Overlay::Hotkeys => self
                .hotkeys
                .iter()
                .enumerate()
                .filter(|(_, h)| match_q(&h.keys) || match_q(&h.action))
                .map(|(i, _)| i)
                .collect(),
            Overlay::Catalog => self
                .catalog
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    match_q(&item.entry.id)
                        || match_q(&item.entry.label)
                        || match_q(&item.entry.source)
                })
                .map(|(i, _)| i)
                .collect(),
            Overlay::Themes => self
                .themes
                .iter()
                .enumerate()
                .filter(|(_, t)| match_q(&t.id) || match_q(&t.label))
                .map(|(i, _)| i)
                .collect(),
            Overlay::Profiles => self
                .profiles
                .iter()
                .enumerate()
                .filter(|(_, p)| match_q(&p.id) || match_q(&p.label))
                .map(|(i, _)| i)
                .collect(),
            Overlay::Rules => self
                .rules
                .iter()
                .enumerate()
                .filter(|(_, r)| match_q(&r.summary) || match_q(&r.raw))
                .map(|(i, _)| i)
                .collect(),
            Overlay::Snapshots => self
                .snapshots
                .iter()
                .enumerate()
                .filter(|(_, s)| match_q(&s.meta.id) || match_q(&s.meta.label))
                .map(|(i, _)| i)
                .collect(),
            Overlay::RiceSync => self
                .rice_status
                .iter()
                .enumerate()
                .filter(|(_, s)| match_q(&s.id))
                .map(|(i, _)| i)
                .collect(),
            Overlay::Palette { query } => {
                let items = super::palette::filtered(query);
                (0..items.len()).collect()
            }
            Overlay::PkgSearch { results, .. } => results
                .iter()
                .enumerate()
                .filter(|(_, h)| match_q(&h.name) || match_q(&h.kind))
                .map(|(i, _)| i)
                .collect(),
            Overlay::WallpaperPicker { files } => files
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(match_q)
                        .unwrap_or(false)
                })
                .map(|(i, _)| i)
                .collect(),
            Overlay::FileBrowser { files, .. } => (0..files.len()).collect(),
            _ => match self.tab {
                Tab::Services => (0..self.services.len()).collect(),
                Tab::Packages => self
                    .packages
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| match_q(&p.name))
                    .map(|(i, _)| i)
                    .collect(),
                Tab::Configs => self
                    .configs
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| match_q(&c.id))
                    .map(|(i, _)| i)
                    .collect(),
                Tab::Plugins => self
                    .plugins
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| match_q(&p.id))
                    .map(|(i, _)| i)
                    .collect(),
                Tab::Logs => self
                    .log_sources
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| match_q(&s.id) || match_q(&s.label))
                    .map(|(i, _)| i)
                    .collect(),
                Tab::Doctor => (0..self.checks.len()).collect(),
                Tab::More => (0..self.more_items.len()).collect(),
                Tab::Dashboard => Vec::new(),
            },
        }
    }

    pub fn list_len(&self) -> usize {
        match &self.overlay {
            Overlay::None | Overlay::Filter | Overlay::Help | Overlay::Prompt { .. } => {
                self.filtered_indices().len()
            }
            Overlay::FileBrowser { files, .. } => files.len(),
            Overlay::LogView { .. } | Overlay::DiffView { .. } => 1,
            Overlay::Confirm { .. } => 2,
            Overlay::Hotkeys
            | Overlay::Catalog
            | Overlay::Themes
            | Overlay::Profiles
            | Overlay::Rules
            | Overlay::Snapshots
            | Overlay::RiceSync
            | Overlay::Palette { .. }
            | Overlay::PkgSearch { .. }
            | Overlay::WallpaperPicker { .. } => self.filtered_indices().len(),
        }
    }

    pub fn clamp_selected(&mut self) {
        let len = self.list_len();
        if len == 0 {
            self.list_state.select(None);
        } else {
            let idx = self.selected().min(len - 1);
            self.list_state.select(Some(idx));
        }
    }

    pub fn move_sel(&mut self, delta: isize) {
        if let Overlay::LogView { scroll, .. } | Overlay::DiffView { scroll, .. } =
            &mut self.overlay
        {
            if delta > 0 {
                *scroll = scroll.saturating_add(1);
            } else {
                *scroll = scroll.saturating_sub(1);
            }
            self.dirty = true;
            return;
        }
        if let Overlay::Confirm { yes, .. } = &mut self.overlay {
            *yes = !*yes;
            self.dirty = true;
            return;
        }
        let len = self.list_len() as isize;
        if len == 0 {
            return;
        }
        let mut idx = self.selected() as isize + delta;
        if idx < 0 {
            idx = len - 1;
        } else if idx >= len {
            idx = 0;
        }
        self.set_selected(idx as usize);
    }

    pub fn open_confirm(&mut self, message: String, action: ConfirmAction) {
        let return_selected = self.selected();
        self.overlay = Overlay::Confirm {
            message,
            action,
            yes: true,
            return_selected,
        };
        self.dirty = true;
    }

    pub fn close_overlay(&mut self) {
        if let Overlay::Confirm {
            return_selected, ..
        } = self.overlay
        {
            self.overlay = Overlay::None;
            self.set_selected(return_selected);
            self.set_msg("cancelled");
            return;
        }
        self.overlay = Overlay::None;
        self.filter.clear();
        self.set_msg(self.tab.hint());
    }

    pub fn switch_tab(&mut self, tab: Tab) {
        self.tab = tab;
        self.filter.clear();
        self.set_selected(0);
        self.ensure_tab_data();
        self.set_msg(self.tab.hint());
    }
}

#[allow(dead_code)]
pub fn install_mode_label(mode: &InstallMode) -> &'static str {
    match mode {
        InstallMode::Core => "CORE",
        InstallMode::Full => "FULL",
    }
}
