use std::io::{stdin, Write};
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};

use crate::core::manifest::Manifest;
use crate::util::terminal::TerminalGuard;

use super::state::App;
use super::ui;

pub fn run(manifest: Manifest) -> Result<()> {
    let mut guard = TerminalGuard::enter()?;
    let mut app = App::new(manifest);

    loop {
        if app.dirty {
            let term = guard.terminal();
            term.draw(|f| ui::draw(f, &mut app))?;
            app.dirty = false;
        }

        if let Some(job) = app.pending_editor.take() {
            let msg = guard.suspend(|| {
                print!("\x1b[2J\x1b[H");
                let _ = std::io::stdout().flush();
                job()
            })?;
            app.refresh_tab(false);
            app.set_msg(msg);
            continue;
        }

        if let Some(job) = app.pending.take() {
            let msg = guard.suspend(|| {
                print!("\x1b[2J\x1b[H");
                println!("macmgr · running…\n");
                let _ = std::io::stdout().flush();
                let msg = job();
                println!("\n{msg}");
                let _ = std::io::stdout().flush();
                let mut buf = String::new();
                let _ = stdin().read_line(&mut buf);
                msg.lines()
                    .find(|l| !l.trim().is_empty() && !l.contains("Press Enter"))
                    .unwrap_or("done")
                    .to_string()
            })?;
            app.refresh_tab(false);
            app.set_msg(msg);
            continue;
        }

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.on_key(key.code, key.modifiers);
                }
            }
        }

        let _ = app.tick_toasts();
        if app.last_log_poll.elapsed() > Duration::from_secs(5) {
            app.poll_log_errors();
        }

        if app.should_quit {
            if let Some(w) = app.watch.take() {
                w.stop();
            }
            break;
        }
    }
    Ok(())
}
