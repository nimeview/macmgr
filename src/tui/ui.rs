use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap};
use ratatui::Frame;

use crate::util::text::{truncate_chars, StatusKind};

use super::state::{App, Overlay, Tab};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area());

    app.last_viewport = chunks[1].height.saturating_sub(2) as usize;

    draw_tabs(f, app, chunks[0]);

    if matches!(app.overlay, Overlay::None | Overlay::Filter) {
        match app.tab {
            Tab::Dashboard => draw_dashboard(f, app, chunks[1]),
            Tab::Services => draw_list_tab(f, app, chunks[1], " Services "),
            Tab::Packages => draw_list_tab(f, app, chunks[1], " Packages "),
            Tab::Configs => draw_list_tab(f, app, chunks[1], " LIVE Configs "),
            Tab::Plugins => draw_list_tab(f, app, chunks[1], " Plugins "),
            Tab::Logs => draw_list_tab(f, app, chunks[1], " Logs "),
            Tab::Doctor => draw_list_tab(f, app, chunks[1], " Doctor "),
            Tab::More => draw_list_tab(f, app, chunks[1], " More "),
        }
    } else {
        draw_overlay(f, app, chunks[1]);
    }
    draw_status(f, app, chunks[2]);
    draw_toasts(f, app, chunks[1]);
}

fn draw_toasts(f: &mut Frame, app: &App, area: Rect) {
    if app.toasts.is_empty() {
        return;
    }
    let height = (app.toasts.len() as u16).saturating_mul(1).saturating_add(2).min(area.height);
    if height < 2 {
        return;
    }
    let width = area.width.min(72).max(20);
    let x = area.x + area.width.saturating_sub(width).saturating_sub(1);
    let y = area.y + 1;
    let toast_area = Rect {
        x,
        y,
        width,
        height,
    };
    f.render_widget(Clear, toast_area);
    let lines: Vec<Line> = app
        .toasts
        .iter()
        .map(|t| {
            let color = match t.kind {
                StatusKind::Ok => Color::Green,
                StatusKind::Error => Color::Red,
                StatusKind::Info => Color::Yellow,
            };
            Line::from(Span::styled(
                format!(" {} ", truncate_chars(&t.text, (width as usize).saturating_sub(2))),
                Style::default().fg(color),
            ))
        })
        .collect();
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" toast ")
                .border_style(Style::default().fg(Color::DarkGray)),
        ),
        toast_area,
    );
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::all()
        .iter()
        .enumerate()
        .map(|(i, t)| Line::from(format!(" {}/{} ", i + 1, t.title())))
        .collect();
    let idx = Tab::all()
        .iter()
        .position(|t| *t == app.tab)
        .unwrap_or(0);
    let watch = if app.watch.is_some() { " · WATCH" } else { "" };
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" macmgr · {}{watch}", app.manifest.name)),
        )
        .select(idx)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, area);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let color = match app.status_kind {
        StatusKind::Ok => Color::Green,
        StatusKind::Error => Color::Red,
        StatusKind::Info => Color::Yellow,
    };
    let filter = if let Overlay::Prompt { buffer, .. } = &app.overlay {
        format!(" [>{buffer}] ")
    } else if !app.filter.is_empty() || matches!(app.overlay, Overlay::Filter) {
        format!(" [/{}] ", app.filter)
    } else {
        String::new()
    };
    let title = format!(" status{filter}");
    let p = Paragraph::new(app.message.as_str())
        .block(Block::default().borders(Borders::ALL).title(title))
        .style(Style::default().fg(color));
    f.render_widget(p, area);
}

fn draw_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let running = app.services.iter().filter(|s| s.core && s.running).count();
    let core_total = app.services.iter().filter(|s| s.core).count();
    let installed = app.packages.iter().filter(|p| p.installed).count();
    let outdated = app.packages.iter().filter(|p| p.outdated).count();
    let warn = app.checks.iter().filter(|c| !c.ok).count();
    let diffs = app
        .configs
        .iter()
        .filter(|c| c.in_sync == Some(false))
        .count();

    let text = vec![
        Line::from(Span::styled(
            "Rice overview (LIVE ~/.config)",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("  Services:   {running}/{core_total} core running")),
        Line::from(format!(
            "  Packages:   {installed}/{} · {outdated} outdated",
            app.packages.len()
        )),
        Line::from(format!(
            "  Configs:    {} present · {diffs} DIFF vs repo",
            app.configs.iter().filter(|c| c.exists).count()
        )),
        Line::from(format!("  Plugins:    {}", app.plugins.len())),
        Line::from(format!("  Doctor:     {warn} warnings")),
        Line::from(format!(
            "  Watch:      {}",
            if app.watch.is_some() { "ON" } else { "off" }
        )),
        Line::from(""),
        Line::from("  Keys: 1-8 tabs · ? help · / filter · r refresh · q quit"),
        Line::from("  Enter here jumps to Doctor if there are warnings"),
    ];
    f.render_widget(
        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(" Dashboard "))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_list_tab(f: &mut Frame, app: &mut App, area: Rect, title: &str) {
    let indices = app.filtered_indices();
    let items: Vec<ListItem> = indices
        .iter()
        .map(|&real| {
            let (line, ok_style) = row_for(app, real);
            ListItem::new(line).style(ok_style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("{title}· {} ", app.tab.hint())),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn row_for(app: &App, real: usize) -> (String, Style) {
    match app.tab {
        Tab::Services => {
            let s = &app.services[real];
            let mark = if s.running { "●" } else { "○" };
            (
                format!(
                    " {} {:<12} {:<8} {}",
                    mark,
                    s.id,
                    if s.running { "running" } else { "stopped" },
                    if s.core { "core" } else { "extra" }
                ),
                if s.running {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                },
            )
        }
        Tab::Packages => {
            let p = &app.packages[real];
            let st = if p.outdated {
                "outdated"
            } else if p.installed {
                "installed"
            } else {
                "missing"
            };
            (
                format!(" {:<22} {:<8} {st}", p.name, p.kind),
                if p.outdated {
                    Style::default().fg(Color::Yellow)
                } else if p.installed {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Gray)
                },
            )
        }
        Tab::Configs => {
            let c = &app.configs[real];
            let sync = match c.in_sync {
                Some(true) => "sync",
                Some(false) => "DIFF",
                None => "—",
            };
            (
                format!(
                    " {:<12} {:<7} {:<4} {:<4} {}",
                    c.id,
                    if c.exists { "LIVE" } else { "MISS" },
                    if c.is_dir { "dir" } else { "file" },
                    sync,
                    c.path.display()
                ),
                if !c.exists {
                    Style::default().fg(Color::Red)
                } else if c.in_sync == Some(false) {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                },
            )
        }
        Tab::Plugins => {
            let p = &app.plugins[real];
            (
                format!(
                    " {}  {:<8} {:<20} {}",
                    if p.enabled { "ON " } else { "off" },
                    p.kind.as_str(),
                    p.id,
                    p.path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?")
                ),
                if p.enabled {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Gray)
                },
            )
        }
        Tab::Logs => {
            let s = &app.log_sources[real];
            (
                format!(
                    " {:<14} {:<4} {}",
                    s.label,
                    if s.path.exists() { "ok" } else { "—" },
                    s.path.display()
                ),
                Style::default(),
            )
        }
        Tab::Doctor => {
            let c = &app.checks[real];
            (
                format!(
                    " {}  {:<20} {}",
                    if c.ok { "OK" } else { "!!" },
                    c.name,
                    truncate_chars(&c.detail, 60)
                ),
                if c.ok {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                },
            )
        }
        Tab::More => {
            let label = app.more_items[real];
            if label.starts_with("──") {
                (
                    format!(" {label}"),
                    Style::default().fg(Color::DarkGray),
                )
            } else {
                (format!("  {label}"), Style::default())
            }
        }
        Tab::Dashboard => (String::new(), Style::default()),
    }
}

fn draw_overlay(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_widget(Clear, area);
    match &app.overlay {
        Overlay::Help => {
            let text = vec![
                Line::from("Help"),
                Line::from("  1-8 tabs · h/l · j/k · Enter · / filter · : palette · ? close"),
                Line::from("  Configs: Enter/e → nano · s rice sync · o Finder · d diff"),
                Line::from("  Rice sync: Enter/p push · P pull · d diff · a/A all · i init"),
                Line::from("  Hotkeys: Enter toggle · a add (;;) · d delete · e nano skhdrc"),
                Line::from("  Catalog: Enter install · u git/URL · f/p pull community feeds"),
                Line::from("  Packages: Enter install · a any · s brew search · u upgrade"),
                Line::from("  : command palette · toast alerts from service logs"),
                Line::from("  More: snapshots, undo, export/import, rice sync…"),
                Line::from("  Snapshots: Enter restore · n name · u undo · d delete"),
                Line::from("  R reload (Plugins: bar · validates Lua first)"),
                Line::from(format!(
                    "  editor={} · override MACMGR_EDITOR",
                    crate::util::editor::resolve_simple_editor()
                )),
            ];
            f.render_widget(
                Paragraph::new(text)
                    .block(Block::default().borders(Borders::ALL).title(" Help · Esc/? ")),
                area,
            );
        }
        Overlay::Confirm {
            message, yes, ..
        } => {
            let body = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {message}"),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(format!(
                    "    {} Yes     {} No",
                    if *yes { "[>]" } else { "[ ]" },
                    if !*yes { "[>]" } else { "[ ]" }
                )),
                Line::from("  Enter/y confirm · n/Esc cancel"),
            ];
            f.render_widget(
                Paragraph::new(body)
                    .block(Block::default().borders(Borders::ALL).title(" Confirm ")),
                area,
            );
        }
        Overlay::Prompt { title, buffer, .. } => {
            let body = vec![
                Line::from(""),
                Line::from(format!("  {title}")),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  > {buffer}█"),
                    Style::default().fg(Color::Cyan),
                )),
                Line::from(""),
                Line::from("  Enter confirm · Esc cancel"),
            ];
            f.render_widget(
                Paragraph::new(body)
                    .block(Block::default().borders(Borders::ALL).title(" Input ")),
                area,
            );
        }
        Overlay::FileBrowser { root, files, config_id, .. } => {
            let items: Vec<ListItem> = files
                .iter()
                .map(|p| {
                    let rel = p.strip_prefix(root).unwrap_or(p);
                    ListItem::new(format!(" {}", rel.display()))
                })
                .collect();
            f.render_stateful_widget(
                List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!(" LIVE {config_id} · Enter/e → nano · Esc ")),
                    )
                    .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan))
                    .highlight_symbol("> "),
                area,
                &mut app.list_state,
            );
        }
        Overlay::LogView {
            title,
            path,
            text,
            scroll,
        } => {
            let lines: Vec<_> = text.lines().collect();
            let slice: Vec<Line> = lines
                .iter()
                .skip(*scroll)
                .take(area.height.saturating_sub(2) as usize)
                .map(|l| Line::from(*l))
                .collect();
            f.render_widget(
                Paragraph::new(slice).block(Block::default().borders(Borders::ALL).title(
                    format!(" {title} · {} · Esc ", path.display()),
                )),
                area,
            );
        }
        Overlay::DiffView {
            title,
            text,
            scroll,
        } => {
            let lines: Vec<_> = text.lines().collect();
            let slice: Vec<Line> = lines
                .iter()
                .skip(*scroll)
                .take(area.height.saturating_sub(2) as usize)
                .map(|l| Line::from(*l))
                .collect();
            f.render_widget(
                Paragraph::new(slice).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" Diff · {title} · Esc ")),
                ),
                area,
            );
        }
        Overlay::Hotkeys
        | Overlay::Catalog
        | Overlay::Themes
        | Overlay::Profiles
        | Overlay::Rules
        | Overlay::Snapshots
        | Overlay::RiceSync
        | Overlay::PkgSearch { .. }
        | Overlay::WallpaperPicker { .. } => {
            draw_overlay_list(f, app, area);
        }
        Overlay::Palette { query } => {
            let items = super::palette::filtered(query);
            let rows: Vec<ListItem> = items
                .iter()
                .map(|c| {
                    ListItem::new(format!(" {:<28} {}", c.label, c.id))
                })
                .collect();
            f.render_stateful_widget(
                List::new(rows)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!(" : {query}█ · Enter · Esc · C-j/C-k ")),
                    )
                    .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan))
                    .highlight_symbol("> "),
                area,
                &mut app.list_state,
            );
        }
        Overlay::None | Overlay::Filter => {}
    }
}

fn draw_overlay_list(f: &mut Frame, app: &mut App, area: Rect) {
    let indices = app.filtered_indices();
    let title = match &app.overlay {
        Overlay::Catalog => " Catalog · Enter · u URL · f pull feeds ",
        Overlay::Hotkeys => " Hotkeys · Enter · a add (;;) · d del · e ",
        Overlay::Themes => " Themes · Enter apply ",
        Overlay::Profiles => " Profiles · Enter apply ",
        Overlay::Rules => " Yabai rules · Enter toggle ",
        Overlay::Snapshots => " Snapshots · Enter restore · n/u/d ",
        Overlay::RiceSync => " LIVE↔rice · Enter/p push · P pull · d ",
        Overlay::PkgSearch { query, .. } => {
            // title needs owned string — handled below
            let _ = query;
            " Brew search "
        }
        Overlay::WallpaperPicker { .. } => " Wallpapers · Enter set ",
        _ => " List ",
    };
    let title_owned;
    let title = match &app.overlay {
        Overlay::PkgSearch { query, results } => {
            title_owned = format!(" brew `{query}` · {} hits · Enter install · s ", results.len());
            title_owned.as_str()
        }
        _ => title,
    };
    let items: Vec<ListItem> = indices
        .iter()
        .filter_map(|&real| overlay_row(app, real))
        .map(|(line, style)| ListItem::new(line).style(style))
        .collect();
    f.render_stateful_widget(
        List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan))
            .highlight_symbol("> "),
        area,
        &mut app.list_state,
    );
}

fn overlay_row(app: &App, real: usize) -> Option<(String, Style)> {
    match &app.overlay {
        Overlay::PkgSearch { results, .. } => {
            let h = results.get(real)?;
            Some((
                format!(" {:<8} {}", h.kind, h.name),
                if h.kind == "cask" {
                    Style::default().fg(Color::Magenta)
                } else {
                    Style::default()
                },
            ))
        }
        Overlay::Hotkeys => {
            let hk = app.hotkeys.get(real)?;
            let multi = if hk.is_multiline() { " …" } else { "" };
            Some((
                format!(
                    " {} {:>3} {:<24} {}{multi}",
                    if hk.enabled { "ON " } else { "off" },
                    real + 1,
                    truncate_chars(&hk.keys, 24),
                    truncate_chars(&hk.action.replace('\n', " · "), 38)
                ),
                if hk.enabled {
                    Style::default()
                } else {
                    Style::default().fg(Color::Gray)
                },
            ))
        }
        Overlay::Catalog => {
            let item = app.catalog.get(real)?;
            let e = &item.entry;
            let tag = match item.origin {
                crate::core::catalog::CatalogOrigin::Local => "loc",
                crate::core::catalog::CatalogOrigin::Community => "feed",
            };
            let src = if e.path.is_empty() {
                e.source.as_str()
            } else {
                e.path.as_str()
            };
            Some((
                format!(
                    " {:<4} {:<10} {:<7} {:<14} {}",
                    tag,
                    e.id,
                    e.kind,
                    truncate_chars(&e.label, 14),
                    truncate_chars(src, 24)
                ),
                if item.origin == crate::core::catalog::CatalogOrigin::Community {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                },
            ))
        }
        Overlay::Themes => {
            let t = app.themes.get(real)?;
            Some((format!(" {:<12} {}", t.id, t.label), Style::default()))
        }
        Overlay::Profiles => {
            let p = app.profiles.get(real)?;
            Some((format!(" {:<12} {}", p.id, p.label), Style::default()))
        }
        Overlay::Rules => {
            let r = app.rules.get(real)?;
            Some((
                format!(
                    " {} L{:<4} {}",
                    if r.enabled { "ON " } else { "off" },
                    r.index + 1,
                    r.summary
                ),
                if r.enabled {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Gray)
                },
            ))
        }
        Overlay::Snapshots => {
            let s = app.snapshots.get(real)?;
            let kind = match s.meta.kind {
                crate::core::snapshot::SnapshotKind::Manual => "MAN",
                crate::core::snapshot::SnapshotKind::Undo => "UND",
                crate::core::snapshot::SnapshotKind::Auto => "AUT",
            };
            Some((
                format!(
                    " [{kind}] {:<26} {}",
                    truncate_chars(&s.meta.id, 26),
                    s.meta.label
                ),
                match s.meta.kind {
                    crate::core::snapshot::SnapshotKind::Undo => Style::default().fg(Color::Yellow),
                    crate::core::snapshot::SnapshotKind::Auto => Style::default().fg(Color::Gray),
                    crate::core::snapshot::SnapshotKind::Manual => Style::default(),
                },
            ))
        }
        Overlay::RiceSync => {
            let s = app.rice_status.get(real)?;
            let mark = match s.in_sync {
                Some(true) => "sync",
                Some(false) => "DIFF",
                None => "—",
            };
            Some((
                format!(
                    " {:<12} {:<4} live={} rice={}",
                    s.id,
                    mark,
                    if s.live_exists { "yes" } else { "no" },
                    if s.rice_exists { "yes" } else { "no" }
                ),
                match s.in_sync {
                    Some(false) => Style::default().fg(Color::Yellow),
                    Some(true) => Style::default().fg(Color::Green),
                    None => Style::default(),
                },
            ))
        }
        Overlay::WallpaperPicker { files } => {
            let p = files.get(real)?;
            Some((
                format!(
                    " {}",
                    p.file_name().and_then(|n| n.to_str()).unwrap_or("?")
                ),
                Style::default(),
            ))
        }
        _ => None,
    }
}
