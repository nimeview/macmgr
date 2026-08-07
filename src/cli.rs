use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "macmgr",
    about = "TUI/CLI manager for your macOS tiling rice (yabai/skhd/SketchyBar)",
    version,
    arg_required_else_help = false
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Launch interactive TUI
    Tui,
    /// Show service and rice status
    Status,
    /// Diagnose the rice installation
    Doctor,
    /// Install the rice (core services or full stack)
    Install {
        #[arg(long, default_value_t = InstallScope::Core)]
        scope: InstallScope,
        #[arg(long)]
        yes: bool,
    },
    /// Uninstall core rice packages
    Uninstall {
        #[arg(long)]
        keep_configs: bool,
        #[arg(long)]
        yes: bool,
    },
    /// Manage brew services (yabai/skhd/sketchybar/borders)
    Service {
        #[command(subcommand)]
        action: ServiceCmd,
    },
    /// Manage brew packages from the rice manifest
    Pkg {
        #[command(subcommand)]
        action: PkgCmd,
    },
    /// Browse and edit rice configs
    Config {
        #[command(subcommand)]
        action: ConfigCmd,
    },
/// Rice mirror (LIVE ↔ My rice) and service reload
    Rice {
        #[command(subcommand)]
        action: RiceCmd,
    },
    /// Apply yabai layout presets
    Preset {
        #[command(subcommand)]
        action: PresetCmd,
    },
    /// Wallpaper helpers
    Wallpaper {
        #[command(subcommand)]
        action: WallpaperCmd,
    },
    /// Manage skhd hotkeys (LIVE skhdrc)
    Hotkeys {
        #[command(subcommand)]
        action: Option<HotkeysCmd>,
    },
    /// Show service logs (yabai/skhd /tmp)
    Logs {
        #[arg(default_value = "yabai-err")]
        id: String,
        #[arg(long, default_value_t = 80)]
        lines: usize,
    },
    /// Manage SketchyBar plugins (LIVE items/widgets)
    Plugin {
        #[command(subcommand)]
        action: PluginCmd,
    },
    /// SketchyBar color themes
    Theme {
        #[command(subcommand)]
        action: ThemeCmd,
    },
    /// Work/game profiles
    Profile {
        #[command(subcommand)]
        action: ProfileCmd,
    },
    /// Plugin catalog (builtin / URL)
    Catalog {
        #[command(subcommand)]
        action: CatalogCmd,
    },
    /// yabai window rules in LIVE yabairc
    Rules {
        #[command(subcommand)]
        action: RulesCmd,
    },
    /// Watch LIVE sketchybar lua and auto-reload
    Watch {
        #[arg(long, default_value_t = 800)]
        interval: u64,
    },
    /// Upgrade core brew packages + reload
    UpdateCore {
        #[arg(long)]
        yes: bool,
    },
    /// Snapshot LIVE configs (named / list / restore / undo)
    Snapshot {
        #[command(subcommand)]
        action: Option<SnapshotCmd>,
    },
    /// Export rice pack (.tar.gz) for another machine
    Export {
        /// Destination .tar.gz (default: ~/Desktop/macmgr-rice-TIMESTAMP.tar.gz)
        path: Option<String>,
    },
    /// Import rice pack (.tar.gz or directory)
    Import {
        path: String,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        install_packages: bool,
        #[arg(long, default_value_t = true)]
        reload: bool,
    },
    /// Run ensure_spaces helper
    Spaces,
    /// Short tips / cheatsheet
    Tips,
    /// Install macmgr itself into cargo bin + copy manifest
    SelfInstall,
}

#[derive(Debug, Clone, ValueEnum, Default)]
pub enum InstallScope {
    #[default]
    Core,
    Full,
}

impl std::fmt::Display for InstallScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallScope::Core => write!(f, "core"),
            InstallScope::Full => write!(f, "full"),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum ServiceCmd {
    List,
    Start {
        name: String,
    },
    Stop {
        name: String,
    },
    Restart {
        name: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum PkgCmd {
    List,
    Search {
        query: String,
    },
    Install {
        name: String,
    },
    Upgrade {
        name: Option<String>,
    },
    Uninstall {
        name: String,
        #[arg(long)]
        yes: bool,
    },
    UpdateAll {
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigCmd {
    List,
    /// List LIVE files inside a config (esp. directories)
    Files {
        id: String,
    },
    /// Edit LIVE config in $EDITOR
    Edit {
        id: String,
    },
    /// Reveal LIVE config in Finder
    Open {
        id: String,
    },
    Backup,
    Restore {
        #[arg(long)]
        yes: bool,
    },
    Diff,
    /// Validate LIVE sketchybar Lua (syntax) without reloading
    Validate,
}

#[derive(Debug, Subcommand)]
pub enum RiceCmd {
    /// Reload core rice services
    Reload,
    /// Create My rice mirror from LIVE (+ optional git init)
    Init {
        #[arg(long)]
        git: bool,
    },
    /// Status LIVE ↔ rice per config
    Status,
    /// Diff LIVE vs rice (optional config id / file)
    Diff {
        id: Option<String>,
        #[arg(long)]
        file: Option<String>,
    },
    /// Push LIVE → rice (selective)
    Push {
        id: Option<String>,
        #[arg(long)]
        file: Option<String>,
    },
    /// Pull rice → LIVE (selective, undo first)
    Pull {
        id: Option<String>,
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        yes: bool,
    },
    /// git status in My rice
    GitStatus,
    /// git add + commit My rice (after rice push)
    Commit {
        #[arg(short = 'm', long)]
        message: Option<String>,
    },
    /// git push My rice to origin
    GitPush,
    /// Legacy: push/pull all (prefer push/pull)
    Sync {
        #[arg(long, default_value = "to-repo")]
        direction: String,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum PresetCmd {
    List,
    Apply {
        id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum WallpaperCmd {
    List,
    Set {
        /// File name, substring, or numeric index
        target: String,
    },
}

#[derive(Debug, Clone, ValueEnum, Default)]
pub enum PluginKindArg {
    #[default]
    Item,
    Widget,
}

#[derive(Debug, Subcommand)]
pub enum PluginCmd {
    /// List SketchyBar items/widgets and ON/off state
    List,
    /// Enable a plugin (uncomment require in init.lua)
    Enable {
        id: String,
        /// Also reload sketchybar
        #[arg(long)]
        reload: bool,
    },
    /// Disable a plugin (comment require in init.lua)
    Disable {
        id: String,
        #[arg(long)]
        reload: bool,
    },
    /// Toggle enable/disable
    Toggle {
        id: String,
        #[arg(long)]
        reload: bool,
    },
    /// Create a new plugin from stub and enable it
    Add {
        name: String,
        #[arg(long, default_value_t = PluginKindArg::Widget)]
        kind: PluginKindArg,
        #[arg(long)]
        reload: bool,
    },
    /// Disable plugin; optionally delete the .lua file
    Remove {
        id: String,
        #[arg(long)]
        delete: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        reload: bool,
    },
    /// Edit plugin source in $EDITOR
    Edit {
        id: String,
    },
    /// Reload sketchybar only
    Reload,
}

impl std::fmt::Display for PluginKindArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginKindArg::Item => write!(f, "item"),
            PluginKindArg::Widget => write!(f, "widget"),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum SnapshotCmd {
    /// Create a new snapshot (default)
    Create {
        #[arg(long, short = 'n')]
        name: Option<String>,
    },
    /// List snapshots
    List,
    /// Restore a snapshot by id/label (auto undo point first)
    Restore {
        id: String,
        #[arg(long)]
        yes: bool,
    },
    /// Undo last restore/import/sync (pop undo stack)
    Undo {
        #[arg(long)]
        yes: bool,
    },
    /// Delete a snapshot
    Delete {
        id: String,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum ThemeCmd {
    List,
    Apply {
        id: String,
        #[arg(long)]
        reload: bool,
    },
    Backup,
}

#[derive(Debug, Subcommand)]
pub enum ProfileCmd {
    List,
    Apply {
        id: String,
    },
    Save {
        id: String,
        #[arg(long, default_value = "")]
        label: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum HotkeysCmd {
    /// List bindings (default)
    List,
    /// Toggle enable/disable (`# macmgr:off`)
    Toggle {
        /// 1-based index from list
        n: usize,
    },
    /// Add binding
    Add {
        #[arg(long)]
        keys: String,
        #[arg(long)]
        action: String,
    },
    /// Delete binding
    Delete {
        n: usize,
        #[arg(long)]
        yes: bool,
    },
    /// Edit keys and/or action
    Set {
        n: usize,
        #[arg(long)]
        keys: Option<String>,
        #[arg(long)]
        action: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum CatalogCmd {
    List,
    /// Install from catalog id
    Install {
        id: String,
        #[arg(long)]
        reload: bool,
    },
    /// Install directly from raw URL or git repo
    FromUrl {
        url: String,
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "widget")]
        kind: String,
        /// Subpath inside git repo (e.g. items/foo.lua)
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        reload: bool,
    },
    /// Add entry to ~/.config/macmgr/catalog.toml
    Add {
        id: String,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        source: String,
        #[arg(long, default_value = "widget")]
        kind: String,
        #[arg(long)]
        path: Option<String>,
        #[arg(long, default_value = "")]
        description: String,
    },
    /// List configured community feeds
    Feeds,
    /// Add a community feed URL (https:// · file: · builtin:community)
    FeedAdd {
        url: String,
        #[arg(long)]
        id: Option<String>,
    },
    /// Pull community feeds into local cache
    Pull,
}

#[derive(Debug, Subcommand)]
pub enum RulesCmd {
    List,
    Toggle {
        /// 1-based line number as shown in list
        line: usize,
    },
    Add {
        app: String,
        #[arg(long, default_value_t = true)]
        manage_off: bool,
    },
}


