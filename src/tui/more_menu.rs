//! More-menu entries (label-driven, section headers are no-ops).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoreKind {
    Section,
    Action,
}

#[derive(Debug, Clone, Copy)]
pub struct MoreEntry {
    pub label: &'static str,
    #[allow(dead_code)]
    pub kind: MoreKind,
}

pub const ENTRIES: &[MoreEntry] = &[
    MoreEntry {
        label: "── Daily ──",
        kind: MoreKind::Section,
    },
    MoreEntry {
        label: "Themes…",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Profiles…",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Hotkeys CRUD…",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Yabai rules…",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Plugin catalog…",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Wallpaper picker…",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Toggle watch-mode (auto reload bar)",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Ensure yabai spaces (9)",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Rice reload",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "── Layout presets ──",
        kind: MoreKind::Section,
    },
    MoreEntry {
        label: "Apply preset: compact (runtime)",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Apply preset: comfortable (runtime)",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Apply preset: spacious (runtime)",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "── Safety / sync ──",
        kind: MoreKind::Section,
    },
    MoreEntry {
        label: "Snapshots… (named / restore)",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Undo last restore/import",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Rice sync… (LIVE ↔ My rice)",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Init rice from LIVE",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Backup LIVE configs",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Restore latest backup",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Export rice pack…",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Import rice pack…",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "── Install (rare) ──",
        kind: MoreKind::Section,
    },
    MoreEntry {
        label: "Install CORE rice",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Install FULL rice",
        kind: MoreKind::Action,
    },
    MoreEntry {
        label: "Update core stack (brew upgrade + reload)",
        kind: MoreKind::Action,
    },
];

pub fn labels() -> Vec<&'static str> {
    ENTRIES.iter().map(|e| e.label).collect()
}
