use std::time::{Duration, Instant};

use crossbeam_channel::{unbounded, Receiver, Sender};

pub const DEFAULT_TICK_HZ: u32 = 10;

/// Scheduler central : le thread principal tick à ~10 Hz,
/// les robots lisent le numéro via try_recv.
#[derive(Debug)]
pub struct TickClock {
    tick: u64,
    interval: Duration,
    last_tick: Instant,
    subscribers: Vec<Sender<u64>>,
}

impl TickClock {
    pub fn new(hz: u32) -> Self {
        let interval = Duration::from_secs_f64(1.0 / hz as f64);
        Self {
            tick: 0,
            interval,
            last_tick: Instant::now(),
            subscribers: Vec::new(),
        }
    }

    pub fn default_hz() -> Self {
        Self::new(DEFAULT_TICK_HZ)
    }

    pub fn current(&self) -> u64 {
        self.tick
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub fn subscribe(&mut self) -> Receiver<u64> {
        let (tx, rx) = unbounded();
        self.subscribers.push(tx);
        rx
    }

    pub fn tick(&mut self) -> Option<u64> {
        if self.last_tick.elapsed() < self.interval {
            return None;
        }
        self.tick += 1;
        self.last_tick = Instant::now();
        for tx in &self.subscribers {
            let _ = tx.try_send(self.tick);
        }
        Some(self.tick)
    }

    // pratique pour les tests
    pub fn force_tick(&mut self) -> u64 {
        self.tick += 1;
        for tx in &self.subscribers {
            let _ = tx.try_send(self.tick);
        }
        self.tick
    }

    pub fn sleep_until_next(&self) {
        let elapsed = self.last_tick.elapsed();
        if elapsed < self.interval {
            std::thread::sleep(self.interval - elapsed);
        }
    }
}

#[derive(Debug)]
pub struct RobotHandle {
    pub comm: super::hub::RobotComm,
    pub tick_rx: Receiver<u64>,
}

impl RobotHandle {
    pub fn poll_tick(&self) -> Option<u64> {
        self.tick_rx.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribers_get_tick_number() {
        let mut clock = TickClock::new(1000);
        let rx = clock.subscribe();

        let n = clock.force_tick();
        assert_eq!(n, 1);
        assert_eq!(rx.try_recv().unwrap(), 1);
    }

    #[test]
    fn no_double_tick_too_soon() {
        let mut clock = TickClock::new(1);
        clock.force_tick();
        assert!(clock.tick().is_none());
    }
}
