//! Ratatui rendering.
//!
//! This layer never mutates any state. It turns a `SimulationSnapshot` into a
//! terminal interface.
//!
//! Architecture note:
//! - the UI does not drive the robots;
//! - the UI does not modify the map;
//! - the UI only renders the latest received snapshot.
//!
//! It draws:
//! - energy/crystal progress bars;
//! - a Robot Fleet panel;
//! - a readable Event Log;
//! - an embedded legend.

use std::collections::HashSet;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::robots::RobotKind;
use crate::simulation::{ResourceView, SimulationSnapshot};
use crate::utils::Position;
use crate::world::ResourceKind;

const PROGRESS_BAR_WIDTH: usize = 18;

pub fn render_loading(frame: &mut Frame<'_>) {
    let area = frame.area();

    let widget = Paragraph::new(Text::from(vec![
        Line::from(vec![Span::styled(
            "Demarrage de la simulation concurrente...",
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from("Initialisation des threads robots, du hub et des channels."),
    ]))
    .block(
        Block::default()
            .title(" Mars Resource Ops ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightGreen)),
    );

    frame.render_widget(widget, area);
}

pub fn render(frame: &mut Frame<'_>, snapshot: &SimulationSnapshot) {
    let area = frame.area();

    if area.width < 105 || area.height < 30 {
        render_terminal_too_small(frame, snapshot);
        return;
    }

    let root = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(70), Constraint::Length(46)])
        .split(area);

    let map_area = root[0];
    let side_area = root[1];

    let side_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(14),
            Constraint::Length(8),
            Constraint::Min(8),
        ])
        .split(side_area);

    let map_widget = Paragraph::new(Text::from(render_map_lines(snapshot))).block(
        Block::default()
            .title(
                " Mars Resource Map  |  x scouts  o collectors  # base  |  Press any key to quit ",
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightCyan)),
    );

    let mission_widget = Paragraph::new(Text::from(mission_control_lines(snapshot))).block(
        Block::default()
            .title(" Mission Control ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightGreen)),
    );

    let fleet_widget = Paragraph::new(Text::from(robot_fleet_lines(snapshot))).block(
        Block::default()
            .title(" Robot Fleet ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta)),
    );

    let event_widget = Paragraph::new(Text::from(event_lines(snapshot))).block(
        Block::default()
            .title(" Event Log ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightBlue)),
    );

    frame.render_widget(map_widget, map_area);
    frame.render_widget(mission_widget, side_split[0]);
    frame.render_widget(fleet_widget, side_split[1]);
    frame.render_widget(event_widget, side_split[2]);
}

fn render_terminal_too_small(frame: &mut Frame<'_>, snapshot: &SimulationSnapshot) {
    let area = frame.area();

    let warning = Paragraph::new(Text::from(vec![
        Line::from(vec![Span::styled(
            "Terminal trop petit",
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from("Agrandis la fenetre pour afficher correctement la simulation."),
        Line::from(format!(
            "Taille actuelle : {} colonnes x {} lignes",
            area.width, area.height
        )),
        Line::from("Taille recommandee : au moins 105 colonnes x 30 lignes"),
        Line::from(""),
        Line::from(format!("Carte : {} x {}", snapshot.width, snapshot.height)),
    ]))
    .block(
        Block::default()
            .title(" Mars Resource Ops ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightRed)),
    );

    frame.render_widget(warning, area);
}

fn render_map_lines(snapshot: &SimulationSnapshot) -> Vec<Line<'static>> {
    let obstacle_set = snapshot.obstacles.iter().copied().collect::<HashSet<_>>();

    let mut lines = Vec::with_capacity(snapshot.height);

    for y in 0..snapshot.height as i32 {
        let mut spans = Vec::with_capacity(snapshot.width);

        for x in 0..snapshot.width as i32 {
            let position = Position::new(x, y);

            let span = if position == snapshot.base {
                base_span()
            } else if let Some(robot) = snapshot
                .robots
                .iter()
                .find(|robot| robot.position == position)
            {
                robot_span(robot.kind)
            } else if let Some(resource) = snapshot
                .resources
                .iter()
                .find(|resource| resource.position == position)
            {
                resource_span(resource)
            } else if obstacle_set.contains(&position) {
                obstacle_span()
            } else {
                empty_span()
            };

            spans.push(span);
        }

        lines.push(Line::from(spans));
    }

    lines
}

fn base_span() -> Span<'static> {
    Span::styled(
        "#",
        Style::default()
            .fg(Color::LightGreen)
            .add_modifier(Modifier::BOLD),
    )
}

fn robot_span(kind: RobotKind) -> Span<'static> {
    match kind {
        RobotKind::Scout => Span::styled(
            "x",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        RobotKind::Collector => Span::styled(
            "o",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
    }
}

fn resource_span(resource: &ResourceView) -> Span<'static> {
    match resource.kind {
        ResourceKind::Energy => Span::styled(
            "E",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        ResourceKind::Crystal => Span::styled(
            "C",
            Style::default()
                .fg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        ),
    }
}

fn obstacle_span() -> Span<'static> {
    Span::styled(
        "O",
        Style::default()
            .fg(Color::LightCyan)
            .add_modifier(Modifier::BOLD),
    )
}

fn empty_span() -> Span<'static> {
    Span::styled(".", Style::default().fg(Color::DarkGray))
}

fn mission_control_lines(snapshot: &SimulationSnapshot) -> Vec<Line<'static>> {
    let spinner = ["|", "/", "-", "\\"][(snapshot.tick as usize) % 4];

    let remaining_energy = sum_remaining(snapshot, ResourceKind::Energy);
    let remaining_crystals = sum_remaining(snapshot, ResourceKind::Crystal);

    let total_energy = snapshot.collected_energy + remaining_energy;
    let total_crystals = snapshot.collected_crystals + remaining_crystals;

    let energy_percent = percent(snapshot.collected_energy, total_energy);
    let crystal_percent = percent(snapshot.collected_crystals, total_crystals);

    let scouts = snapshot
        .robots
        .iter()
        .filter(|robot| robot.kind == RobotKind::Scout)
        .count();

    let collectors = snapshot
        .robots
        .iter()
        .filter(|robot| robot.kind == RobotKind::Collector)
        .count();

    vec![
        Line::from(vec![
            Span::styled("Etat: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("Simulation concurrente {spinner}"),
                Style::default().fg(Color::LightGreen),
            ),
        ]),
        Line::from(vec![
            Span::styled("Tick: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(snapshot.tick.to_string()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Energie  ", Style::default().fg(Color::Green)),
            Span::raw(format!(
                "{} / {}  {}%",
                snapshot.collected_energy, total_energy, energy_percent
            )),
        ]),
        progress_line(snapshot.collected_energy, total_energy, Color::Green),
        Line::from(vec![
            Span::styled("Cristaux ", Style::default().fg(Color::LightMagenta)),
            Span::raw(format!(
                "{} / {}  {}%",
                snapshot.collected_crystals, total_crystals, crystal_percent
            )),
        ]),
        progress_line(
            snapshot.collected_crystals,
            total_crystals,
            Color::LightMagenta,
        ),
        Line::from(""),
        Line::from(vec![
            Span::styled("Robots: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{scouts} scouts / {collectors} collectors")),
        ]),
        Line::from(vec![
            Span::styled("Connues: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "{} ressources / {} obstacles",
                snapshot.known_resources, snapshot.known_obstacles
            )),
        ]),
        Line::from(vec![
            Span::styled("Legende: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled("O", Style::default().fg(Color::LightCyan)),
            Span::raw(" obstacle  "),
            Span::styled("E", Style::default().fg(Color::Green)),
            Span::raw(" energie  "),
            Span::styled("C", Style::default().fg(Color::LightMagenta)),
            Span::raw(" cristal"),
        ]),
    ]
}

fn progress_line(value: u32, total: u32, color: Color) -> Line<'static> {
    let filled = if total == 0 {
        0
    } else {
        ((value as f64 / total as f64) * PROGRESS_BAR_WIDTH as f64).round() as usize
    }
    .min(PROGRESS_BAR_WIDTH);

    let empty = PROGRESS_BAR_WIDTH.saturating_sub(filled);

    Line::from(vec![
        Span::raw("["),
        Span::styled(
            "█".repeat(filled),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled("░".repeat(empty), Style::default().fg(Color::DarkGray)),
        Span::raw("]"),
    ])
}

fn robot_fleet_lines(snapshot: &SimulationSnapshot) -> Vec<Line<'static>> {
    let mut robots = snapshot.robots.clone();
    robots.sort_by_key(|robot| robot.id);

    let mut lines = Vec::new();

    for pair in robots.chunks(2) {
        let left = robot_short_label(&pair[0]);

        if let Some(right_robot) = pair.get(1) {
            let right = robot_short_label(right_robot);

            lines.push(Line::from(vec![left, Span::raw("   "), right]));
        } else {
            lines.push(Line::from(vec![left]));
        }
    }

    if lines.is_empty() {
        lines.push(Line::from("Aucun robot actif."));
    }

    lines
}

fn robot_short_label(robot: &crate::robots::RobotSnapshot) -> Span<'static> {
    let kind = match robot.kind {
        RobotKind::Scout => "S",
        RobotKind::Collector => "C",
    };

    let cargo = robot
        .cargo
        .map(|cargo| format!(" {}{}", resource_short_label(cargo.kind), cargo.amount))
        .unwrap_or_default();

    let label = format!("R{} {} {:>9}{}", robot.id.0, kind, robot.state.label(), cargo);

    let color = match robot.kind {
        RobotKind::Scout => Color::Red,
        RobotKind::Collector => Color::Magenta,
    };

    Span::styled(label, Style::default().fg(color))
}

fn event_lines(snapshot: &SimulationSnapshot) -> Vec<Line<'static>> {
    if snapshot.events.is_empty() {
        return vec![Line::from("Aucun evenement pour le moment.")];
    }

    snapshot
        .events
        .iter()
        .rev()
        .map(|event| {
            let style = event_style(event);

            Line::from(vec![
                Span::styled("• ", Style::default().fg(Color::LightBlue)),
                Span::styled(event.clone(), style),
            ])
        })
        .collect()
}

fn event_style(event: &str) -> Style {
    if event.contains("depose") {
        Style::default()
            .fg(Color::LightGreen)
            .add_modifier(Modifier::BOLD)
    } else if event.contains("collecte") {
        Style::default().fg(Color::Yellow)
    } else if event.contains("decouvre") {
        Style::default().fg(Color::LightCyan)
    } else if event.contains("refuse") || event.contains("aucun chemin") {
        Style::default().fg(Color::LightRed)
    } else {
        Style::default().fg(Color::Gray)
    }
}

fn sum_remaining(snapshot: &SimulationSnapshot, kind: ResourceKind) -> u32 {
    snapshot
        .resources
        .iter()
        .filter(|resource| resource.kind == kind)
        .map(|resource| resource.quantity)
        .sum()
}

fn percent(value: u32, total: u32) -> u32 {
    if total == 0 {
        100
    } else {
        ((value as f64 / total as f64) * 100.0).round() as u32
    }
}

fn resource_short_label(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Energy => "E",
        ResourceKind::Crystal => "C",
    }
}
