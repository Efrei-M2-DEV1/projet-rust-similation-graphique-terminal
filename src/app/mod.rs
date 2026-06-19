//! Ratatui loop.
//!
//! The UI no longer drives the simulation: it only receives snapshots produced
//! by the simulation thread.

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::simulation::{SimulationHandle, SimulationSnapshot};
use crate::ui;

pub fn run(mut simulation: SimulationHandle) -> Result<()> {
    let mut terminal = setup_terminal()?;
    drain_pending_input()?;

    let result = run_loop(&mut terminal, &mut simulation);
    simulation.shutdown();

    let restore_result = restore_terminal(&mut terminal);

    result.and(restore_result)
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    simulation: &mut SimulationHandle,
) -> Result<()> {
    let mut latest_snapshot: Option<SimulationSnapshot> = None;

    loop {
        // Drain every available snapshot and keep only the most recent one.
        // This prevents the UI from lagging when the simulation produces
        // snapshots faster than the terminal can render them.
        for snapshot in simulation.snapshot_rx.try_iter() {
            latest_snapshot = Some(snapshot);
        }

        terminal.draw(|frame| {
            if let Some(snapshot) = &latest_snapshot {
                ui::render(frame, snapshot);
            } else {
                ui::render_loading(frame);
            }
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    simulation.shutdown();
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Flushes input keys left in the buffer.
/// Useful on Windows to avoid quitting immediately on launch.
fn drain_pending_input() -> Result<()> {
    while event::poll(Duration::ZERO)? {
        let _ = event::read()?;
    }

    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
