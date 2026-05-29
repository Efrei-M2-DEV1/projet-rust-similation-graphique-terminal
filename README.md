# projet-rust-similation-graphique-terminal

**Resource Collection Simulation** — Simulation graphique terminal (Ratatui) de robots autonomes qui explorent une carte procédurale et collectent des ressources.

> Projet universitaire EFREI M2 — équipe de 4 développeurs.

## Stack

- **Rust** (edition 2021)
- **Ratatui** + **Crossterm** — rendu terminal
- **noise** — génération procédurale (bruit de Perlin)
- **rand** — quantités / placements aléatoires
- **crossbeam-channel** — communication asynchrone inter-robots
- **pathfinding** — A\* pour collecteurs
- **anyhow** — gestion d'erreurs

## Architecture

```
src/
├── app/             # Boucle principale, état global
├── world/           # Carte, tuiles, ressources, génération Perlin
├── robots/          # Scout, Collector, traits communs
├── communication/   # Messages, channels, hub central
├── pathfinding/     # A* et utilitaires de navigation
├── ui/              # Rendu Ratatui
└── utils/           # Position, direction, helpers
```

## Légende visuelle

| Élément    | Symbole | Couleur         |
|------------|---------|-----------------|
| Obstacle   | `O`     | cyan clair      |
| Énergie    | `E`     | vert            |
| Cristal    | `C`     | magenta clair   |
| Base       | `#`     | vert clair      |
| Scout      | `x`     | rouge           |
| Collector  | `o`     | magenta         |

## Installation

```bash
git clone https://github.com/Efrei-M2-DEV1/projet-rust-similation-graphique-terminal.git
cd projet-rust-similation-graphique-terminal
cargo build --release
```

## Lancement

```bash
cargo run --release
```

Appuyer sur **n'importe quelle touche** pour quitter.

## Répartition de l'équipe

| Dev | Périmètre |
|-----|-----------|
| Dev 1 | Fondations projet, module `world` (carte Perlin, ressources), `utils` |
| Dev 2 | Robots (Scout / Collector), `pathfinding` |
| Dev 3 | `communication` (channels, messages), Base centrale |
| Dev 4 | UI Ratatui, intégration, boucle `app` |

## Workflow Git

- `main` — branche stable, protégée
- `feat/<scope>` — branches de feature (1 par tâche)
- Merge via **Pull Request** uniquement, avec relecture
- Convention de commits : **Conventional Commits** (`feat:`, `fix:`, `chore:`, `docs:`, `test:`, `refactor:`)

## Tests

```bash
cargo test
```

