//! Boucle principale de l'application.
//!
//! Ce module s'occupe de:
//! - préparer et restaurer le terminal
//! - lire les événements clavier
//! - rafraîchir l'état de simulation
//! - demander le rendu à la couche `ui`

use std::io;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode,
    enable_raw_mode,
    EnterAlternateScreen,
    LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::ui;
use crate::utils::Position;
use crate::world::{Map, ResourceKind};

#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceCounters {
    pub collected_energy: u32,
    pub collected_crystals: u32,
}

#[derive(Debug)]
pub struct App {
    pub map: Map,
    pub stats: ResourceCounters,
    pub scouts: Vec<Position>,
    pub collectors: Vec<Position>,
    should_quit: bool,
    tick: u64,
}

impl App {
    pub fn new(map: Map) -> Self {
        Self {
            map,
            stats: ResourceCounters::default(),
            scouts: Vec::new(),
            collectors: Vec::new(),
            should_quit: false,
            tick: 0,
        }
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        // Sous Windows, des KeyRelease arrivent au démarrage et quittaient
        // l'app instantanément si on ne filtre pas.
        if key.kind != KeyEventKind::Press {
            return;
        }
        self.should_quit = true;
    }

    pub fn update(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    pub fn remaining_energy(&self) -> usize {
        self.map.count_resources(ResourceKind::Energy)
    }

    pub fn remaining_crystals(&self) -> usize {
        self.map.count_resources(ResourceKind::Crystal)
    }
}

pub fn run(mut app: App) -> Result<()> {
    let mut terminal = setup_terminal()?;
    drain_pending_input()?;
    let run_result = run_loop(&mut terminal, &mut app);
    let restore_result = restore_terminal(&mut terminal);

    run_result.and(restore_result)
}

/// Vide les touches restées dans le buffer (Enter de la commande, etc.).
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

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let tick_rate = Duration::from_millis(120);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| {
            ui::render(frame, app);
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.update();
            last_tick = Instant::now();
        }

        if app.should_quit() {
            break;
        }
    }

    Ok(())
}

