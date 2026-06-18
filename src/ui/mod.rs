//! Rendu Ratatui.
//!
//! Cette couche ne modifie aucun état.
//! Elle transforme un SimulationSnapshot en interface terminal.

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

pub fn render_loading(frame: &mut Frame<'_>) {
    let area = frame.area();

    let widget = Paragraph::new("Demarrage de la simulation concurrente...")
        .block(
            Block::default()
                .title(" Mars Resource Ops ")
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::LightGreen));

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
        .constraints([Constraint::Min(70), Constraint::Length(42)])
        .split(area);

    let map_area = root[0];
    let side_area = root[1];

    let side_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12),
            Constraint::Length(9),
            Constraint::Length(6),
            Constraint::Min(8),
        ])
        .split(side_area);

    let map_widget = Paragraph::new(Text::from(render_map_lines(snapshot))).block(
        Block::default()
            .title(" Mars Resource Map ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightCyan)),
    );

    let stats_widget = Paragraph::new(Text::from(stats_lines(snapshot))).block(
        Block::default()
            .title(" Mission Control ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightGreen)),
    );

    let legend_widget = Paragraph::new(Text::from(legend_lines())).block(
        Block::default()
            .title(" Legend ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray)),
    );

    let help_widget = Paragraph::new(Text::from(help_lines(snapshot))).block(
        Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    let event_widget = Paragraph::new(Text::from(event_lines(snapshot))).block(
        Block::default()
            .title(" Event Log ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightBlue)),
    );

    frame.render_widget(map_widget, map_area);
    frame.render_widget(stats_widget, side_split[0]);
    frame.render_widget(legend_widget, side_split[1]);
    frame.render_widget(help_widget, side_split[2]);
    frame.render_widget(event_widget, side_split[3]);
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

fn stats_lines(snapshot: &SimulationSnapshot) -> Vec<Line<'static>> {
    let spinner = ["|", "/", "-", "\\"][(snapshot.tick as usize) % 4];

    let remaining_energy = sum_remaining(snapshot, ResourceKind::Energy);
    let remaining_crystals = sum_remaining(snapshot, ResourceKind::Crystal);

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
            Span::raw(format!("Simulation concurrente {}", spinner)),
        ]),
        Line::from(vec![
            Span::styled("Tick: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(snapshot.tick.to_string()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Energie: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "{} collectees / {} restantes",
                snapshot.collected_energy, remaining_energy
            )),
        ]),
        Line::from(vec![
            Span::styled("Cristaux: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "{} collectes / {} restants",
                snapshot.collected_crystals, remaining_crystals
            )),
        ]),
        Line::from(vec![
            Span::styled("Robots: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{} scouts / {} collectors", scouts, collectors)),
        ]),
        Line::from(vec![
            Span::styled("Connues: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "{} ressources / {} obstacles",
                snapshot.known_resources, snapshot.known_obstacles
            )),
        ]),
    ]
}

fn sum_remaining(snapshot: &SimulationSnapshot, kind: ResourceKind) -> u32 {
    snapshot
        .resources
        .iter()
        .filter(|resource| resource.kind == kind)
        .map(|resource| resource.quantity)
        .sum()
}

fn legend_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled(
                "O",
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  obstacle"),
        ]),
        Line::from(vec![
            Span::styled(
                "E",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  energie"),
        ]),
        Line::from(vec![
            Span::styled(
                "C",
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  cristal"),
        ]),
        Line::from(vec![
            Span::styled(
                "#",
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  base"),
        ]),
        Line::from(vec![
            Span::styled(
                "x",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  scout"),
        ]),
        Line::from(vec![
            Span::styled(
                "o",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  collector"),
        ]),
    ]
}

fn help_lines(snapshot: &SimulationSnapshot) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled("Quitter: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("une touche"),
        ]),
        Line::from(vec![
            Span::styled("Carte: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{} x {}", snapshot.width, snapshot.height)),
        ]),
        Line::from(""),
        Line::from("Scouts: explorent et partagent."),
        Line::from("Collectors: collectent et deposent."),
    ]
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
            Line::from(vec![
                Span::styled("• ", Style::default().fg(Color::LightBlue)),
                Span::raw(event.clone()),
            ])
        })
        .collect()
}
