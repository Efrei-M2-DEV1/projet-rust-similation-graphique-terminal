//! Boucle principale de l'application.
//!
//! Ce module s'occupe de:
//! - préparer et restaurer le terminal
//! - lire les événements clavier
//! - rafraîchir l'état de simulation
//! - demander le rendu à la couche `ui`

use std::collections::HashSet;
use std::io;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode,
    enable_raw_mode,
    EnterAlternateScreen,
    LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::communication::{self, CommHub, TickClock};
use crate::robots::{CollectorRobot, Robot, RobotTickContext, ScoutRobot};
use crate::ui;
use crate::utils::Position;
use crate::world::{Map, ResourceKind};

const SCOUT_COUNT: usize = 3;
const COLLECTOR_COUNT: usize = 3;

#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceCounters {
    pub collected_energy: u32,
    pub collected_crystals: u32,
}

pub struct App {
    pub map: Map,
    pub stats: ResourceCounters,
    pub scouts: Vec<Position>,
    pub collectors: Vec<Position>,
    scout_robots: Vec<ScoutRobot>,
    collector_robots: Vec<CollectorRobot>,
    hub: CommHub,
    clock: TickClock,
    known_resources: usize,
    known_obstacles: usize,
    should_quit: bool,
    tick: u64,
}

impl App {
    pub fn new(map: Map) -> Self {
        let (mut hub, mut clock) = communication::setup(map.base());
        let mut scout_robots = Vec::with_capacity(SCOUT_COUNT);
        let mut collector_robots = Vec::with_capacity(COLLECTOR_COUNT);

        for index in 0..SCOUT_COUNT {
            let handle = communication::register_robot(&mut hub, &mut clock);
            scout_robots.push(ScoutRobot::new(
                handle,
                map.base(),
                0x5C0A_0000 + index as u64,
            ));
        }

        for _ in 0..COLLECTOR_COUNT {
            let handle = communication::register_robot(&mut hub, &mut clock);
            collector_robots.push(CollectorRobot::new(handle, map.base()));
        }

        let mut app = Self {
            map,
            stats: ResourceCounters::default(),
            scouts: Vec::new(),
            collectors: Vec::new(),
            scout_robots,
            collector_robots,
            hub,
            clock,
            known_resources: 0,
            known_obstacles: 0,
            should_quit: false,
            tick: 0,
        };
        app.refresh_robot_positions();
        app
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn handle_key(&mut self, _key: KeyEvent) {
        self.should_quit = true;
    }

    pub fn update(&mut self) {
        self.tick = self.clock.force_tick();
        let mut occupied = self.occupied_positions();

        for scout in &mut self.scout_robots {
            let mut ctx = RobotTickContext::new(&mut self.map, &mut occupied);
            scout.tick(&mut ctx);
        }

        for collector in &mut self.collector_robots {
            let mut ctx = RobotTickContext::new(&mut self.map, &mut occupied);
            collector.tick(&mut ctx);
        }

        self.hub.poll();
        self.sync_from_base();
        self.refresh_robot_positions();
    }

    pub fn remaining_energy(&self) -> usize {
        self.map.count_resources(ResourceKind::Energy)
    }

    pub fn remaining_crystals(&self) -> usize {
        self.map.count_resources(ResourceKind::Crystal)
    }

    pub fn known_resources(&self) -> usize {
        self.known_resources
    }

    pub fn known_obstacles(&self) -> usize {
        self.known_obstacles
    }

    fn occupied_positions(&self) -> HashSet<Position> {
        let base = self.map.base();
        self.scout_robots
            .iter()
            .map(|robot| robot.position())
            .chain(self.collector_robots.iter().map(|robot| robot.position()))
            .filter(|position| *position != base)
            .collect()
    }

    fn refresh_robot_positions(&mut self) {
        self.scouts = self.scout_robots.iter().map(|robot| robot.position()).collect();
        self.collectors = self
            .collector_robots
            .iter()
            .map(|robot| robot.position())
            .collect();
    }

    fn sync_from_base(&mut self) {
        let base_ref = self.hub.base();
        let Ok(base) = base_ref.lock() else {
            return;
        };

        self.stats.collected_energy = base.stored_energy();
        self.stats.collected_crystals = base.stored_crystals();
        self.known_resources = base.known_resource_count();
        self.known_obstacles = base.known_obstacles().len();
    }
}

pub fn run(mut app: App) -> Result<()> {
    let mut terminal = setup_terminal()?;
    let run_result = run_loop(&mut terminal, &mut app);
    let restore_result = restore_terminal(&mut terminal);

    run_result.and(restore_result)
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

