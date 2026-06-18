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
    let area = frame.area();

    // Sécurité UX :
    // Ratatui dessine dans la taille actuelle du terminal.
    // Si la fenêtre est trop petite, la carte et le panneau latéral deviennent illisibles.
    // On affiche donc un message clair au lieu d'un rendu cassé.
    if area.width < 100 || area.height < 28 {
        render_terminal_too_small(frame, app);
        return;
    }

    // Layout principal :
    // - grande zone à gauche : carte de simulation
    // - colonne à droite : état, légende, aide
    let root = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(60),     // carte
            Constraint::Length(38),  // panneau latéral
        ])
        .split(area);

    let map_area = root[0];
    let side_area = root[1];

    // Découpage du panneau latéral.
    // On donne plus de hauteur à l'état car il contient les compteurs.
    let side_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Mission Control
            Constraint::Length(10), // Légende
            Constraint::Min(6),     // Aide
        ])
        .split(side_area);

    let map_widget = Paragraph::new(Text::from(render_map_lines(app))).block(
        Block::default()
            .title(" Mars Resource Map ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightCyan)),
    );

    let stats_widget = Paragraph::new(Text::from(stats_lines(app))).block(
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

    let help_widget = Paragraph::new(Text::from(help_lines(app))).block(
        Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(map_widget, map_area);
    frame.render_widget(stats_widget, side_split[0]);
    frame.render_widget(legend_widget, side_split[1]);
    frame.render_widget(help_widget, side_split[2]);
}

fn render_terminal_too_small(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();

    let warning = Paragraph::new(Text::from(vec![
        Line::from(vec![
            Span::styled(
                "Terminal trop petit",
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from("Agrandis la fenêtre pour afficher correctement la simulation."),
        Line::from(""),
        Line::from(format!(
            "Taille actuelle : {} colonnes x {} lignes",
            area.width, area.height
        )),
        Line::from("Taille recommandée : au moins 100 colonnes x 28 lignes"),
        Line::from(""),
        Line::from(format!(
            "Carte du projet : {} x {}",
            app.map.width(),
            app.map.height()
        )),
    ]))
    .block(
        Block::default()
            .title(" Mars Resource Ops ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightRed)),
    );

    frame.render_widget(warning, area);
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
    // Petit spinner visuel pour montrer que la simulation tourne.
    // On change de symbole selon le tick courant.
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

        // Ici on affiche les unités collectées ET les unités restantes.
        // C'est plus précis que seulement compter le nombre de gisements.
        Line::from(vec![
            Span::styled("Energie: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "{} u collectees / {} u restantes",
                app.stats.collected_energy,
                app.remaining_energy_units()
            )),
        ]),

        Line::from(vec![
            Span::styled("Cristaux: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "{} u collectees / {} u restantes",
                app.stats.collected_crystals,
                app.remaining_crystal_units()
            )),
        ]),

        // Ici on garde aussi le nombre de gisements encore présents.
        // Cela permet de distinguer :
        // - combien de zones de ressources restent ;
        // - combien d'unités restent au total.
        Line::from(vec![
            Span::styled("Gisements: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "{} E / {} C",
                app.remaining_energy(),
                app.remaining_crystals()
            )),
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
            Span::raw("  obstacle infranchissable"),
        ]),
        Line::from(vec![
            Span::styled("E", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("  source d'energie"),
        ]),
        Line::from(vec![
            Span::styled("C", Style::default().fg(Color::LightMagenta).add_modifier(Modifier::BOLD)),
            Span::raw("  depot de cristal"),
        ]),
        Line::from(vec![
            Span::styled("#", Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)),
            Span::raw("  base centrale"),
        ]),
        Line::from(vec![
            Span::styled("x", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw("  scout"),
        ]),
        Line::from(vec![
            Span::styled("o", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Span::raw("  collector"),
        ]),
    ]
}

fn help_lines(app: &App) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled("Quitter: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("appuyer sur une touche"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Carte: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{} x {}", app.map.width(), app.map.height())),
        ]),
        Line::from(""),
        Line::from("Les scouts explorent et partagent."),
        Line::from("Les collectors collectent puis rentrent."),
    ]
}
