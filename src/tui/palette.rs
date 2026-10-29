//! Command palette (`:`) — searchable shortcuts into TUI actions.

#[derive(Debug, Clone, Copy)]
pub struct PaletteCmd {
    pub id: &'static str,
    pub label: &'static str,
    pub keywords: &'static str,
}

pub const COMMANDS: &[PaletteCmd] = &[
    PaletteCmd {
        id: "tab:dash",
        label: "Go · Dashboard",
        keywords: "home dash overview",
    },
    PaletteCmd {
        id: "tab:svc",
        label: "Go · Services",
        keywords: "yabai skhd sketchybar borders start stop",
    },
    PaletteCmd {
        id: "tab:pkgs",
        label: "Go · Packages",
        keywords: "brew install cask",
    },
    PaletteCmd {
        id: "tab:cfg",
        label: "Go · Configs (LIVE)",
        keywords: "edit nano sketchybar skhd yabai",
    },
    PaletteCmd {
        id: "tab:plug",
        label: "Go · Plugins",
        keywords: "widget item sketchybar lua",
    },
    PaletteCmd {
        id: "tab:logs",
        label: "Go · Logs",
        keywords: "stderr yabai skhd",
    },
    PaletteCmd {
        id: "tab:doc",
        label: "Go · Doctor",
        keywords: "sip accessibility check diagnose",
    },
    PaletteCmd {
        id: "tab:more",
        label: "Go · More",
        keywords: "extras menu",
    },
    PaletteCmd {
        id: "pkg:search",
        label: "Brew · Search packages",
        keywords: "search formula cask find",
    },
    PaletteCmd {
        id: "pkg:any",
        label: "Brew · Install by name (+track)",
        keywords: "install adhoc any track",
    },
    PaletteCmd {
        id: "pkg:track",
        label: "Brew · Track in user.toml",
        keywords: "manifest add package user",
    },
    PaletteCmd {
        id: "config:add",
        label: "Config · Add path to user.toml",
        keywords: "manifest track config nvim",
    },
    PaletteCmd {
        id: "rice:sync",
        label: "Rice · Sync LIVE ↔ My rice",
        keywords: "push pull pin sync diff",
    },
    PaletteCmd {
        id: "rice:init",
        label: "Rice · Init from LIVE",
        keywords: "create mirror git",
    },
    PaletteCmd {
        id: "snap:list",
        label: "Snapshots · Browse / restore",
        keywords: "backup undo named",
    },
    PaletteCmd {
        id: "snap:create",
        label: "Snapshots · Create named",
        keywords: "save backup",
    },
    PaletteCmd {
        id: "snap:undo",
        label: "Snapshots · Undo last",
        keywords: "rollback restore",
    },
    PaletteCmd {
        id: "export",
        label: "Pack · Export .tar.gz",
        keywords: "export archive share",
    },
    PaletteCmd {
        id: "import",
        label: "Pack · Import .tar.gz",
        keywords: "import restore pack",
    },
    PaletteCmd {
        id: "hotkeys",
        label: "Hotkeys · CRUD skhd",
        keywords: "skhd binding keys",
    },
    PaletteCmd {
        id: "catalog:pull",
        label: "Catalog · Pull community feeds",
        keywords: "feed community sync pull plugins",
    },
    PaletteCmd {
        id: "catalog",
        label: "Catalog · Plugin install",
        keywords: "plugin clock weather git url",
    },
    PaletteCmd {
        id: "themes",
        label: "Themes · Apply bar colors",
        keywords: "color theme",
    },
    PaletteCmd {
        id: "profiles",
        label: "Profiles · work / game",
        keywords: "profile work game",
    },
    PaletteCmd {
        id: "rules",
        label: "Yabai · Rules",
        keywords: "window rule manage",
    },
    PaletteCmd {
        id: "reload:bar",
        label: "Reload · SketchyBar (safe)",
        keywords: "reload restart bar validate",
    },
    PaletteCmd {
        id: "reload:rice",
        label: "Reload · Core rice services",
        keywords: "yabai skhd borders restart",
    },
    PaletteCmd {
        id: "watch",
        label: "Watch · Toggle auto-reload .lua",
        keywords: "watch mode auto",
    },
    PaletteCmd {
        id: "doctor",
        label: "Doctor · Re-run checks",
        keywords: "diagnose sip",
    },
    PaletteCmd {
        id: "help",
        label: "Help · Keybindings",
        keywords: "keys ? help",
    },
];

pub fn filtered(query: &str) -> Vec<&'static PaletteCmd> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return COMMANDS.iter().collect();
    }
    COMMANDS
        .iter()
        .filter(|c| {
            c.id.contains(&q) || c.label.to_lowercase().contains(&q) || c.keywords.contains(&q)
        })
        .collect()
}
