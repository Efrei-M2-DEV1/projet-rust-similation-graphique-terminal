//! Robot collecteur.
//!
//! Responsabilités :
//! - recevoir les ressources connues depuis le hub ;
//! - choisir une cible ;
//! - se déplacer avec A* ;
//! - demander au hub de collecter une unité ;
//! - revenir EXACTEMENT à la base pour déposer.
//!
//! Correction importante :
//! - Pour une ressource, le collector peut collecter à courte distance.
//! - Pour la base, il doit revenir sur la case exacte de la base.
//!
//! Cela corrige le blocage observé dans l'Event Log :
//! "aucun chemin vers (40, 14)".
//! Sur une carte 80x28, (40,14) est la base.

use std::collections::HashSet;
use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};

use crate::communication::{HubToRobot, KnownResource, RobotId, RobotToHub};
use crate::robots::common::{CarriedResource, LocalKnowledge};
use crate::utils::Position;
use crate::world::{Map, ResourceKind};

/// Capacité fixée à 1 pour respecter strictement l'énoncé :
/// le collector collecte une unité, puis retourne déposer à la base.
pub const DEFAULT_COLLECTOR_CAPACITY: u32 = 1;

/// Rayon de collecte autour d'une ressource.
///
/// Le collector doit s'approcher d'une ressource, mais n'a pas besoin
/// d'entrer exactement sur la case du gisement.
/// Cela réduit les blocages autour des ressources.
const COLLECTION_RANGE: u32 = 3;

/// Nombre maximal de cases temporairement évitées après des refus de déplacement.
const TEMPORARY_BLOCK_LIMIT: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectorState {
    Waiting,
    MovingToResource,
    Collecting,
    ReturningToBase,
}

impl CollectorState {
    #[allow(dead_code)]
    pub const fn label(self) -> &'static str {
        match self {
            CollectorState::Waiting => "waiting",
            CollectorState::MovingToResource => "to resource",
            CollectorState::Collecting => "collecting",
            CollectorState::ReturningToBase => "to base",
        }
    }
}

pub struct CollectorRobot {
    id: RobotId,
    position: Position,
    knowledge: LocalKnowledge,
    map: Arc<Map>,
    to_hub: Sender<RobotToHub>,
    from_hub: Receiver<HubToRobot>,
    target: Option<Position>,
    cargo: Option<CarriedResource>,
    capacity: u32,
    state: CollectorState,
    awaiting_move: bool,
    awaiting_collect: bool,
    temporary_blocked: HashSet<Position>,
}

impl CollectorRobot {
    pub fn new(
        id: RobotId,
        start: Position,
        map: Arc<Map>,
        to_hub: Sender<RobotToHub>,
        from_hub: Receiver<HubToRobot>,
    ) -> Self {
        Self {
            id,
            position: start,
            knowledge: LocalKnowledge::default(),
            map,
            to_hub,
            from_hub,
            target: None,
            cargo: None,
            capacity: DEFAULT_COLLECTOR_CAPACITY,
            state: CollectorState::Waiting,
            awaiting_move: false,
            awaiting_collect: false,
            temporary_blocked: HashSet::new(),
        }
    }

    /// Boucle principale du collecteur.
    ///
    /// Elle tourne dans son propre thread.
    pub fn run(mut self) {
        while let Ok(message) = self.from_hub.recv() {
            match message {
                HubToRobot::Tick(_tick) => self.on_tick(),

                HubToRobot::Knowledge {
                    resources,
                    obstacles,
                } => {
                    let previous_count = self.knowledge.resources().len();
                    self.knowledge.replace_with(&resources, &obstacles);
                    let new_count = self.knowledge.resources().len();

                    if previous_count == 0 && new_count > 0 {
                        self.log(format!("connait maintenant {} ressource(s)", new_count));
                    }
                }

                HubToRobot::MoveGranted { to } => {
                    self.position = to;
                    self.awaiting_move = false;

                    // Un mouvement réussi montre que la situation s'est débloquée.
                    self.temporary_blocked.clear();
                }

                HubToRobot::MoveDenied { attempted } => {
                    self.awaiting_move = false;

                    self.log(format!(
                        "deplacement refuse vers ({}, {})",
                        attempted.x, attempted.y
                    ));

                    self.temporary_blocked.insert(attempted);

                    if self.temporary_blocked.len() > TEMPORARY_BLOCK_LIMIT {
                        self.temporary_blocked.clear();
                    }
                }

                HubToRobot::CollectGranted {
                    position,
                    kind,
                    remaining,
                } => {
                    self.awaiting_collect = false;
                    self.add_cargo(kind);

                    self.log(format!(
                        "collecte 1 {} en ({}, {})",
                        resource_label(kind),
                        position.x,
                        position.y
                    ));

                    if remaining == 0 {
                        self.knowledge.remove_resource(position);
                        self.target = None;
                    } else {
                        self.knowledge.set_resource(KnownResource {
                            position,
                            kind,
                            quantity: remaining,
                        });
                    }

                    // Capacité = 1 : retour immédiat à la base.
                    if self.carried_amount() >= self.capacity {
                        self.state = CollectorState::ReturningToBase;
                        self.target = None;
                        self.temporary_blocked.clear();

                        self.log("retourne a la base avec sa cargaison".to_string());
                    }
                }

                HubToRobot::CollectDenied { position } => {
                    self.awaiting_collect = false;
                    self.knowledge.remove_resource(position);
                    self.target = None;
                    self.state = CollectorState::Waiting;

                    self.log(format!(
                        "collecte refusee en ({}, {})",
                        position.x, position.y
                    ));
                }

                HubToRobot::Shutdown => break,
            }
        }
    }

    fn on_tick(&mut self) {
        if self.awaiting_move || self.awaiting_collect {
            return;
        }

        let base = self.map.base();

        // CAS 1 : le collector est sur la base avec une cargaison.
        // Il peut déposer.
        if self.position == base && self.has_cargo() {
            self.deposit_at_base();
            return;
        }

        // CAS 2 : le collector transporte quelque chose.
        // Il doit revenir EXACTEMENT sur la base, pas juste à portée.
        if self.has_cargo() {
            self.state = CollectorState::ReturningToBase;
            self.request_move_to_base(base);
            return;
        }

        // CAS 3 : le collector a déjà une cible ressource.
        if let Some(target) = self.target {
            if self.is_in_collection_range(target) {
                self.request_collect(target);
            } else if self.target_is_known(target) {
                self.state = CollectorState::MovingToResource;
                self.request_move_towards_resource(target);
            } else {
                self.target = None;
                self.state = CollectorState::Waiting;
            }

            return;
        }

        // CAS 4 : le collector choisit une nouvelle ressource connue.
        if let Some(target) = self.choose_target() {
            self.target = Some(target);
            self.state = CollectorState::MovingToResource;

            self.log(format!(
                "vise une ressource en ({}, {})",
                target.x, target.y
            ));

            if self.is_in_collection_range(target) {
                self.request_collect(target);
            } else {
                self.request_move_towards_resource(target);
            }

            return;
        }

        self.state = CollectorState::Waiting;
    }

    /// Choisit une ressource.
    ///
    /// Les collectors pairs préfèrent l'énergie.
    /// Les collectors impairs préfèrent les cristaux.
    fn choose_target(&self) -> Option<Position> {
        let preferred_kind = self.preferred_kind();

        self.choose_target_for_kind(preferred_kind)
            .or_else(|| self.choose_any_target())
    }

    fn choose_target_for_kind(&self, kind: ResourceKind) -> Option<Position> {
        self.knowledge
            .resources()
            .values()
            .filter(|resource| resource.quantity > 0)
            .filter(|resource| resource.kind == kind)
            .filter_map(|resource| {
                self.path_len_to_collection_range(resource.position)
                    .map(|path_len| (resource.position, path_len))
            })
            .min_by_key(|(_position, path_len)| *path_len)
            .map(|(position, _)| position)
    }

    fn choose_any_target(&self) -> Option<Position> {
        self.knowledge
            .resources()
            .values()
            .filter(|resource| resource.quantity > 0)
            .filter_map(|resource| {
                self.path_len_to_collection_range(resource.position)
                    .map(|path_len| (resource.position, path_len))
            })
            .min_by_key(|(_position, path_len)| *path_len)
            .map(|(position, _)| position)
    }

    fn preferred_kind(&self) -> ResourceKind {
        if self.id.0 % 2 == 0 {
            ResourceKind::Energy
        } else {
            ResourceKind::Crystal
        }
    }

    /// Longueur du meilleur chemin vers une zone de collecte autour d'une ressource.
    fn path_len_to_collection_range(&self, target: Position) -> Option<usize> {
        self.collection_positions(target)
            .into_iter()
            .filter_map(|goal| {
                crate::pathfinding::find_path_avoiding(
                    &self.map,
                    self.position,
                    goal,
                    &self.temporary_blocked,
                )
                .map(|path| path.len())
            })
            .min()
    }

    /// Positions acceptables pour collecter une ressource.
    ///
    /// Cette règle ne s'applique PAS à la base.
    fn collection_positions(&self, target: Position) -> Vec<Position> {
        let mut positions = Vec::new();
        let radius = COLLECTION_RANGE as i32;

        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let position = Position::new(target.x + dx, target.y + dy);

                if !self.map.in_bounds(position) {
                    continue;
                }

                if position.manhattan(target) > COLLECTION_RANGE {
                    continue;
                }

                if self.map.is_walkable(position) {
                    positions.push(position);
                }
            }
        }

        positions
    }

    fn target_is_known(&self, target: Position) -> bool {
        self.knowledge
            .resources()
            .get(&target)
            .map(|resource| resource.quantity > 0)
            .unwrap_or(false)
    }

    fn is_in_collection_range(&self, target: Position) -> bool {
        self.position.manhattan(target) <= COLLECTION_RANGE
    }

    /// Mouvement vers une ressource.
    ///
    /// Ici, on va vers une position à portée de collecte.
    fn request_move_towards_resource(&mut self, target: Position) {
        let Some(goal) = self.best_collection_goal(target) else {
            self.log(format!(
                "aucun point de collecte vers ({}, {})",
                target.x, target.y
            ));
            self.target = None;
            self.state = CollectorState::Waiting;
            self.temporary_blocked.clear();
            return;
        };

        if goal == self.position {
            self.request_collect(target);
            return;
        }

        let Some(next) = self.next_step_to(goal) else {
            self.log(format!(
                "aucun chemin vers ressource ({}, {})",
                target.x, target.y
            ));
            self.target = None;
            self.state = CollectorState::Waiting;
            self.temporary_blocked.clear();
            return;
        };

        self.request_move(next);
    }

    /// Mouvement vers la base.
    ///
    /// Ici, contrairement aux ressources, on doit atteindre la case exacte
    /// de la base pour déposer.
    fn request_move_to_base(&mut self, base: Position) {
        if self.position == base {
            self.deposit_at_base();
            return;
        }

        let Some(next) = self.next_step_to_exact(base) else {
            self.log(format!(
                "chemin exact vers base introuvable ({}, {})",
                base.x, base.y
            ));

            // Sécurité : si les blocages temporaires empêchent le retour,
            // on les oublie. La base doit toujours rester prioritaire.
            self.temporary_blocked.clear();
            return;
        };

        self.request_move(next);
    }

    fn request_move(&mut self, next: Position) {
        self.awaiting_move = true;

        let _ = self.to_hub.send(RobotToHub::MoveRequested {
            robot_id: self.id,
            from: self.position,
            to: next,
        });
    }

    fn best_collection_goal(&self, target: Position) -> Option<Position> {
        self.collection_positions(target)
            .into_iter()
            .filter_map(|goal| {
                crate::pathfinding::find_path_avoiding(
                    &self.map,
                    self.position,
                    goal,
                    &self.temporary_blocked,
                )
                .map(|path| (goal, path.len()))
            })
            .min_by_key(|(_goal, path_len)| *path_len)
            .map(|(goal, _)| goal)
    }

    /// Prochain pas vers un objectif quelconque en utilisant les blocages temporaires.
    fn next_step_to(&mut self, goal: Position) -> Option<Position> {
        if self.position == goal {
            return None;
        }

        if let Some(next) = crate::pathfinding::next_step_avoiding(
            &self.map,
            self.position,
            goal,
            &self.temporary_blocked,
        ) {
            return Some(next);
        }

        // Si les blocages temporaires empêchent le chemin,
        // on les nettoie et on retente sans eux.
        if !self.temporary_blocked.is_empty() {
            self.temporary_blocked.clear();

            let empty_blocked = HashSet::new();

            return crate::pathfinding::next_step_avoiding(
                &self.map,
                self.position,
                goal,
                &empty_blocked,
            );
        }

        None
    }

    /// Prochain pas vers une case exacte.
    ///
    /// Utilisé surtout pour revenir à la base.
    fn next_step_to_exact(&mut self, goal: Position) -> Option<Position> {
        self.next_step_to(goal)
    }

    fn request_collect(&mut self, position: Position) {
        self.awaiting_collect = true;
        self.state = CollectorState::Collecting;

        self.log(format!(
            "demande collecte en ({}, {})",
            position.x, position.y
        ));

        let _ = self.to_hub.send(RobotToHub::CollectRequested {
            robot_id: self.id,
            position,
        });
    }

    fn deposit_at_base(&mut self) {
        if let Some(cargo) = self.cargo.take() {
            self.log(format!(
                "depose {} {} a la base",
                cargo.amount,
                resource_label(cargo.kind)
            ));

            let _ = self.to_hub.send(RobotToHub::Deposit {
                robot_id: self.id,
                kind: cargo.kind,
                amount: cargo.amount,
            });
        }

        self.state = CollectorState::Waiting;
        self.temporary_blocked.clear();
    }

    fn add_cargo(&mut self, kind: ResourceKind) {
        match &mut self.cargo {
            Some(cargo) if cargo.kind == kind => {
                cargo.amount += 1;
            }
            Some(cargo) => {
                cargo.kind = kind;
                cargo.amount = 1;
            }
            None => {
                self.cargo = Some(CarriedResource { kind, amount: 1 });
            }
        }
    }

    fn has_cargo(&self) -> bool {
        self.carried_amount() > 0
    }

    fn carried_amount(&self) -> u32 {
        self.cargo.map(|cargo| cargo.amount).unwrap_or(0)
    }

    fn log(&self, text: String) {
        let _ = self.to_hub.send(RobotToHub::Log {
            robot_id: self.id,
            text,
        });
    }
}

fn resource_label(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Energy => "energie",
        ResourceKind::Crystal => "cristal",
    }
}
