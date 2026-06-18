//! Rendu Ratatui de la simulation.
//!
//! Ce module ne modifie pas l'état: il lit `App` et le transforme en
//! interface terminal.

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::utils::Position;
use crate::world::{ResourceKind, Tile};

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let root = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(34)])
        .split(frame.area());

    let map_area = root[0];
    let side_area = root[1];

    let side_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Length(10), Constraint::Min(5)])
        .split(side_area);

    let map_widget = Paragraph::new(Text::from(render_map_lines(app))).block(
        Block::default()
            .title("Carte")
            .borders(Borders::ALL),
    );

    let stats_widget = Paragraph::new(Text::from(stats_lines(app))).block(
        Block::default()
            .title("Etat")
            .borders(Borders::ALL),
    );

    let legend_widget = Paragraph::new(Text::from(legend_lines())).block(
        Block::default()
            .title("Legende")
            .borders(Borders::ALL),
    );

    let help_widget = Paragraph::new(Text::from(help_lines(app))).block(
        Block::default()
            .title("Aide")
            .borders(Borders::ALL),
    );

    frame.render_widget(map_widget, map_area);
    frame.render_widget(stats_widget, side_split[0]);
    frame.render_widget(legend_widget, side_split[1]);
    frame.render_widget(help_widget, side_split[2]);
}

fn render_map_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(app.map.height());

    for y in 0..app.map.height() as i32 {
        let mut spans = Vec::with_capacity(app.map.width());

        for x in 0..app.map.width() as i32 {
            let pos = Position::new(x, y);

            let span = if let Some(robot) = robot_span(pos, app) {
                robot
            } else {
                let tile = app.map.get(pos).expect("position in bounds");
                tile_span(tile)
            };

            spans.push(span);
        }

        lines.push(Line::from(spans));
    }

    lines
}

fn robot_span(pos: Position, app: &App) -> Option<Span<'static>> {
    if app.collectors.contains(&pos) {
        Some(Span::styled(
            "o",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ))
    } else if app.scouts.contains(&pos) {
        Some(Span::styled(
            "x",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ))
    } else {
        None
    }
}

fn tile_span(tile: &Tile) -> Span<'static> {
    match tile {
        Tile::Empty => Span::styled(".", Style::default().fg(Color::DarkGray)),
        Tile::Obstacle => Span::styled(
            "O",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ),
        Tile::Base => Span::styled(
            "#",
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        ),
        Tile::Resource(resource) => match resource.kind {
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
        },
    }
}

fn stats_lines(app: &App) -> Vec<Line<'static>> {
    let spinner = ["|", "/", "-", "\\"][(app.tick() as usize) % 4];

    vec![
        Line::from(vec![
            Span::styled("Etat: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("Simulation en cours {}", spinner)),
        ]),
        Line::from(vec![
            Span::styled("Tick: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(app.tick().to_string()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("E collecte: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(app.stats.collected_energy.to_string()),
        ]),
        Line::from(vec![
            Span::styled("C collecte: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(app.stats.collected_crystals.to_string()),
        ]),
        Line::from(vec![
            Span::styled("E restantes: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(app.remaining_energy().to_string()),
        ]),
        Line::from(vec![
            Span::styled("C restantes: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(app.remaining_crystals().to_string()),
        ]),
        Line::from(vec![
            Span::styled("Robots: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "{} eclaireurs / {} collecteurs",
                app.scouts.len(),
                app.collectors.len()
            )),
        ]),
        Line::from(vec![
            Span::styled("Connues: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "{} ressources / {} obstacles",
                app.known_resources(),
                app.known_obstacles()
            )),
        ]),
    ]
}

fn legend_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled("O", Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)),
            Span::raw(" obstacle"),
        ]),
        Line::from(vec![
            Span::styled("E", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" energie"),
        ]),
        Line::from(vec![
            Span::styled("C", Style::default().fg(Color::LightMagenta).add_modifier(Modifier::BOLD)),
            Span::raw(" cristal"),
        ]),
        Line::from(vec![
            Span::styled("#", Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)),
            Span::raw(" base"),
        ]),
        Line::from(vec![
            Span::styled("x", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" eclaireur"),
        ]),
        Line::from(vec![
            Span::styled("o", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Span::raw(" collecteur"),
        ]),
    ]
}

fn help_lines(app: &App) -> Vec<Line<'static>> {
    vec![
        Line::from("Appuyer sur n'importe quelle touche"),
        Line::from("pour quitter"),
        Line::from(""),
        Line::from(format!(
            "Carte: {} x {}",
            app.map.width(),
            app.map.height()
        )),
    ]
}
