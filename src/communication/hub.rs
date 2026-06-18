use std::sync::{Arc, Mutex};

use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};

use crate::communication::base::Base;
use crate::communication::message::{Envelope, Message, RobotId};
use crate::utils::Position;

#[derive(Debug)]
pub struct RobotComm {
    id: RobotId,
    to_hub: Sender<Envelope>,
    from_hub: Receiver<Envelope>,
}

impl RobotComm {
    pub fn id(&self) -> RobotId {
        self.id
    }

    pub fn send(&self, payload: Message) -> bool {
        self.to_hub
            .try_send(Envelope::new(self.id, payload))
            .is_ok()
    }

    pub fn drain_inbox(&self) -> Vec<Envelope> {
        self.from_hub.try_iter().collect()
    }

    pub fn try_recv(&self) -> Option<Envelope> {
        match self.from_hub.try_recv() {
            Ok(env) => Some(env),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }
}

#[derive(Debug)]
pub struct CommHub {
    base: Arc<Mutex<Base>>,
    robot_txs: Vec<Sender<Envelope>>,
    hub_rx: Receiver<Envelope>,
    hub_tx: Sender<Envelope>,
    next_id: usize,
}

impl CommHub {
    pub fn new(base_position: Position) -> Self {
        let (hub_tx, hub_rx) = unbounded();
        Self {
            base: Arc::new(Mutex::new(Base::new(base_position))),
            robot_txs: Vec::new(),
            hub_rx,
            hub_tx,
            next_id: 0,
        }
    }

    pub fn base(&self) -> Arc<Mutex<Base>> {
        Arc::clone(&self.base)
    }

    pub fn register_robot(&mut self) -> RobotComm {
        let (tx, rx) = unbounded();
        let id = RobotId(self.next_id);
        self.next_id += 1;
        self.robot_txs.push(tx);

        RobotComm {
            id,
            to_hub: self.hub_tx.clone(),
            from_hub: rx,
        }
    }

    pub fn poll(&self) -> usize {
        let mut n = 0;
        for envelope in self.hub_rx.try_iter() {
            if let Ok(mut base) = self.base.lock() {
                base.apply(&envelope.payload);
            }
            self.broadcast(&envelope);
            n += 1;
        }
        n
    }

    fn broadcast(&self, envelope: &Envelope) {
        for (i, tx) in self.robot_txs.iter().enumerate() {
            if RobotId(i) == envelope.sender {
                continue;
            }
            let _ = tx.try_send(envelope.clone());
        }
    }

    pub fn robot_count(&self) -> usize {
        self.robot_txs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::ResourceKind;

    #[test]
    fn scout_message_reaches_collector() {
        let mut hub = CommHub::new(Position::new(5, 5));
        let scout = hub.register_robot();
        let collector = hub.register_robot();

        let res_pos = Position::new(12, 8);
        scout.send(Message::ResourceFound {
            position: res_pos,
            kind: ResourceKind::Energy,
            quantity: 100,
        });
        assert_eq!(hub.poll(), 1);

        let base_ref = hub.base();
        let base = base_ref.lock().unwrap();
        assert_eq!(base.known_resource_count(), 1);
        drop(base);

        let msgs = collector.drain_inbox();
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0].payload, Message::ResourceFound { .. }));
    }

    #[test]
    fn deposit_updates_counters() {
        let mut hub = CommHub::new(Position::new(0, 0));
        let collector = hub.register_robot();

        collector.send(Message::Deposit {
            kind: ResourceKind::Crystal,
            amount: 42,
        });
        hub.poll();

        let base_ref = hub.base();
        let base = base_ref.lock().unwrap();
        assert_eq!(base.stored_crystals(), 42);
    }

    #[test]
    fn emitter_does_not_get_own_broadcast() {
        let mut hub = CommHub::new(Position::new(0, 0));
        let scout = hub.register_robot();

        scout.send(Message::ObstacleFound {
            position: Position::new(1, 1),
        });
        hub.poll();

        assert!(scout.drain_inbox().is_empty());
    }
}
