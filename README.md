# projet-rust-similation-graphique-terminal

**Resource Collection Simulation** — Simulation graphique terminal (Ratatui) de robots autonomes qui explorent une carte procédurale et collectent des ressources.

> Projet universitaire EFREI M2 — équipe de 4 développeurs.

## Prérequis

| Outil | Version minimale |
|-------|-----------------|
| [Rust](https://rustup.rs/) | 1.75+ (edition 2021) |
| Cargo | inclus avec Rust |
| Terminal | 80×24 colonnes minimum, support des couleurs ANSI |

> **Windows** : utilisez Windows Terminal ou un émulateur ANSI compatible (pas l'ancien `cmd.exe`).  
> **Linux/macOS** : n'importe quel terminal moderne.

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

Appuyer sur **n'importe quelle touche** pour quitter proprement (le terminal est restauré).

> Pour un build de développement (plus rapide à compiler, moins optimisé) :
> ```bash
> cargo run
> ```

## Tests

```bash
# Lancer tous les tests unitaires
cargo test

# Résultat attendu : 39 tests, 0 échec
```

Les modules couverts par les TU :

| Module | Ce qui est testé |
|--------|-----------------|
| `utils` | `Position::manhattan`, voisinage 4-connexe |
| `world::map` | base au centre, bornes |
| `world::generator` | déterminisme par seed, zone safe, présence d'obstacles |
| `world::populate` | placement déterministe, types Energy/Crystal |
| `communication` | scheduler, hub crossbeam, base, messages |
| `pathfinding` | chemin A*, cases évitées, cas limites |
| `robots::common` | `LocalKnowledge` : obstacle, ressource, épuisement, observation |
| `robots::scout` | pas de mouvement sans tick, déplacement, évitement obstacles |
| `robots::collector` | état `Waiting` sans ressource, `MovingToResource` après message |

## Stack

- **Rust** (edition 2021)
- **Ratatui** + **Crossterm** — rendu terminal
- **noise** — génération procédurale (bruit de Perlin)
- **rand** — quantités / placements aléatoires
- **crossbeam-channel** — communication asynchrone inter-robots
- **pathfinding_crate** — A\* pour collecteurs (alias de la crate `pathfinding`)
- **anyhow** — gestion d'erreurs

## Architecture

```
src/
├── app/             # Boucle principale, état global
├── world/           # Carte, tuiles, ressources, génération Perlin
├── robots/          # Scout, Collector, traits communs
├── communication/   # Messages, channels, hub central, base
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

## Répartition de l'équipe

| Dev | Périmètre |
|-----|-----------|
| Dev 1 | Fondations projet, module `world` (carte Perlin, ressources), `utils` |
| Dev 2 | Robots (Scout / Collector), `pathfinding` |
| Dev 3 | `communication` (channels, messages), Base centrale |
| Dev 4 | UI Ratatui, intégration, boucle `app` |

## Workflow Git

- `main` — branche stable, protégée
- `dev` — branche d'intégration
- `feat/<scope>` — branches de feature (1 par tâche), merge dans `dev` via PR
- Convention de commits : **Conventional Commits** (`feat:`, `fix:`, `chore:`, `docs:`, `test:`, `refactor:`)

