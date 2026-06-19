# projet-rust-similation-graphique-terminal

**Mars Resource Ops** — Simulation graphique terminal en Rust avec **Ratatui**, mettant en scène des robots autonomes qui explorent une carte procédurale, découvrent des ressources, communiquent via un hub central et collectent de l’énergie ainsi que des cristaux.

> Projet universitaire EFREI M2 — Resource Collection Simulation.

---

## Sommaire

- [Résumé du projet](#résumé-du-projet)
- [Choix techniques](#choix-techniques)
- [Structure du projet](#structure-du-projet)
- [Installation](#installation)
- [Lancement](#lancement)
- [Contrôles](#contrôles)
- [Légende visuelle](#légende-visuelle)
- [Fonctionnalités](#fonctionnalités)
- [Architecture générale](#architecture-générale)
- [Modules principaux](#modules-principaux)
- [Tests](#tests)
- [Workflow Git](#workflow-git)

---

## Résumé du projet

Le projet simule une mission de collecte de ressources sur une carte 2D générée procéduralement.

Des **scouts** explorent la carte et partagent leurs découvertes. Des **collectors** utilisent ces informations pour se déplacer vers les ressources, collecter une unité, puis revenir à la base pour déposer leur cargaison.

L’interface terminal affiche en temps réel :

- la carte ;
- les robots ;
- les obstacles ;
- les ressources ;
- les compteurs de mission ;
- l’état de la flotte ;
- les événements récents.

---

## Choix techniques

| Besoin                     | Choix                       |
| -------------------------- | --------------------------- |
| Langage                    | Rust 2021                   |
| Interface terminal         | Ratatui                     |
| Gestion terminal / clavier | Crossterm                   |
| Concurrence                | `std::thread`               |
| Communication              | `crossbeam-channel`         |
| Génération procédurale     | `noise`                     |
| Aléatoire déterministe     | `rand`                      |
| Chemins                    | A\* via `pathfinding_crate` |
| Gestion d’erreurs          | `anyhow`                    |

---

## Structure du projet

```txt
src/
├── app/             # Boucle UI Ratatui et gestion terminal
├── communication/   # Messages échangés entre robots, hub et simulation
├── pathfinding/     # A* et navigation sur la grille
├── robots/          # Scouts, collectors et types communs
├── simulation/      # Moteur concurrent, hub, snapshots, tests simulation
├── ui/              # Rendu Ratatui
├── utils/           # Position, direction, helpers géométriques
├── world/           # Carte, tuiles, ressources, génération, placement
└── main.rs          # Point d’entrée du projet
```

---

## Installation

### Prérequis

| Outil    | Version recommandée     |
| -------- | ----------------------- |
| Rust     | 1.75+                   |
| Cargo    | Inclus avec Rust        |
| Terminal | Support ANSI / couleurs |

Installation de Rust :

```bash
https://rustup.rs/
```

Installation du formateur Rust si nécessaire :

```bash
rustup component add rustfmt
```

Cloner le projet :

```bash
git clone https://github.com/Efrei-M2-DEV1/projet-rust-similation-graphique-terminal.git
cd projet-rust-similation-graphique-terminal
```

---

## Lancement

Build de développement :

```bash
cargo run
```

Build optimisé :

```bash
cargo run --release
```

Vérification rapide avant lancement :

```bash
cargo fmt
cargo check
cargo test
```

---

## Contrôles

La simulation démarre directement dans le terminal.

| Action                | Contrôle                            |
| --------------------- | ----------------------------------- |
| Quitter la simulation | Appuyer sur n’importe quelle touche |

Le terminal est restauré proprement à la sortie.

---

## Légende visuelle

| Élément   | Symbole | Couleur       |
| --------- | ------: | ------------- |
| Obstacle  |     `O` | Cyan clair    |
| Énergie   |     `E` | Vert          |
| Cristal   |     `C` | Magenta clair |
| Base      |     `#` | Vert clair    |
| Scout     |     `x` | Rouge         |
| Collector |     `o` | Magenta       |
| Case vide |     `.` | Gris          |

---

## Fonctionnalités

- Génération procédurale d’une carte 2D.
- Obstacles générés automatiquement.
- Ressources énergie et cristal avec quantités.
- Placement des ressources sur des zones atteignables depuis la base.
- Base centrale servant de point de dépôt.
- Scouts autonomes chargés de l’exploration.
- Collectors autonomes chargés de la collecte.
- Communication par messages via un hub central.
- Simulation concurrente avec threads.
- Interface Ratatui temps réel.
- Event Log dynamique.
- Mission Control avec compteurs et progression.
- Robot Fleet affichant l’état synthétique de la flotte.
- Tests unitaires et tests de simulation.

---

## Architecture générale

Le projet repose sur une architecture concurrente inspirée du modèle acteur.

```txt
┌──────────────────────────┐
│      UI Ratatui          │
│  Affiche les snapshots   │
└─────────────▲────────────┘
              │
              │ snapshots
              │
┌─────────────┴────────────┐
│        Hub / Simulation   │
│  Etat global officiel     │
│  Validation des actions   │
└───────▲──────────▲───────┘
        │          │
        │ messages │
        │          │
┌───────┴───┐  ┌───┴──────────┐
│ Scouts    │  │ Collectors    │
│ Threads   │  │ Threads       │
└───────────┘  └───────────────┘
```

Résumé :

- l’UI Ratatui tourne dans le thread principal ;
- le hub centralise l’état global de la simulation ;
- chaque robot tourne dans son propre thread ;
- les échanges passent par des channels ;
- l’UI affiche uniquement des snapshots.

---

## Modules principaux

### `world`

Contient la carte, les tuiles, les ressources, la génération procédurale et le placement des ressources.

Les ressources sont placées uniquement sur des cases accessibles depuis la base afin d’éviter des objectifs impossibles à atteindre.

### `robots`

Contient les comportements des scouts et des collectors.

Les scouts explorent et signalent les découvertes. Les collectors ciblent une ressource connue, collectent une unité, puis reviennent à la base.

### `communication`

Contient les messages typés échangés entre les robots et le hub.

En une phrase : les robots ne modifient pas directement l’état global, ils envoient des demandes au hub, qui valide et diffuse les informations utiles.

### `simulation`

Contient le moteur concurrent, le hub central, les threads robots, les snapshots envoyés à l’UI et les tests liés à la simulation.

### `pathfinding`

Contient la navigation A\* utilisée par les collectors pour atteindre les ressources ou revenir à la base.

### `ui`

Contient le rendu Ratatui : carte, Mission Control, Robot Fleet et Event Log.

### `app`

Gère le terminal, la boucle d’affichage Ratatui et l’arrêt propre de l’application.

---

## Architecture concurrente

La simulation utilise :

- un thread pour l’interface ;
- un thread pour le hub ;
- plusieurs threads pour les scouts ;
- plusieurs threads pour les collectors.

Le hub envoie des ticks aux robots. Les robots réagissent, envoient leurs messages, puis le hub met à jour l’état global et produit un snapshot pour l’interface.

Cette organisation permet de séparer clairement :

```txt
simulation
communication
robots
affichage
```

---

## Interface Ratatui

L’interface affiche :

- la carte de mission ;
- les obstacles ;
- les ressources restantes ;
- les robots ;
- la base ;
- les compteurs énergie / cristaux ;
- les états des robots ;
- les événements récents.

L’UI ne modifie pas l’état de la simulation : elle affiche le dernier snapshot reçu.

---

## Tests

Lancer tous les tests :

```bash
cargo test
```

Les tests couvrent notamment :

| Zone               | Vérifications                                     |
| ------------------ | ------------------------------------------------- |
| `utils`            | distance de Manhattan, voisins                    |
| `world::map`       | base au centre, bornes                            |
| `world::generator` | génération déterministe, obstacles, zone sûre     |
| `world::populate`  | ressources énergie/cristal, placement atteignable |
| `pathfinding`      | chemins A\*, obstacles, navigation                |
| `simulation`       | snapshots, ticks, robots, collecte et dépôt       |

Résultat attendu :

```txt
test result: ok
```

---

## Qualité du projet

Commandes recommandées avant un commit :

```bash
cargo fmt
cargo check
cargo test
```

Le projet doit compiler sans erreur et les tests doivent passer avant toute Pull Request.

---

## Workflow Git

Branches principales :

| Branche            | Rôle                    |
| ------------------ | ----------------------- |
| `main`             | Version stable          |
| `dev`              | Branche d’intégration   |
| `feat/<scope>`     | Nouvelle fonctionnalité |
| `fix/<scope>`      | Correction              |
| `refactor/<scope>` | Refactorisation         |

Convention de commits recommandée :

```txt
feat:
fix:
test:
docs:
refactor:
chore:
```

Exemples :

```bash
git commit -m "feat: improve ratatui mission dashboard"
git commit -m "fix: stabilize concurrent collection loop"
git commit -m "test: add concurrent simulation tests"
git commit -m "docs: update README"
```

---

## Résumé d’architecture

Le projet suit une architecture concurrente simple et lisible :

```txt
Scouts → découvrent
Collectors → collectent
Hub → valide, centralise, diffuse
UI → affiche des snapshots
```

Cette organisation permet de répondre aux objectifs du projet : simulation graphique terminal, robots autonomes, communication asynchrone, collecte de ressources, base centrale et affichage temps réel.
