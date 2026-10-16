//! Shared state and the background ticker.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use chrono::{Local, Utc};
use tokio::time::{Instant, MissedTickBehavior};

use crate::engine::Engine;

pub const TICK_PERIOD: Duration = Duration::from_secs(5);

/// Handle shared by the ticker and both servers.
#[derive(Clone)]
pub struct App {
    engine: Arc<Mutex<Engine>>,
    last_heartbeat: Arc<Mutex<Option<Instant>>>,
}

impl App {
    #[must_use]
    pub fn new(engine: Engine) -> Self {
        Self {
            engine: Arc::new(Mutex::new(engine)),
            last_heartbeat: Arc::default(),
        }
    }

    /// Locks the engine. A panic in another holder must not take the
    /// service down, so poisoning is ignored.
    pub fn engine(&self) -> MutexGuard<'_, Engine> {
        self.engine.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn record_heartbeat(&self) {
        *self
            .last_heartbeat
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(Instant::now());
    }

    /// When the Spotify extension last reported in, if ever.
    #[must_use]
    pub fn last_heartbeat(&self) -> Option<Instant> {
        *self
            .last_heartbeat
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// Advances the engine every [`TICK_PERIOD`]. Runs forever.
pub async fn run_ticker(app: App) {
    let mut interval = tokio::time::interval(TICK_PERIOD);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut last = Instant::now();
    loop {
        interval.tick().await;
        let now = Instant::now();
        let elapsed = now - last;
        last = now;
        let events = app
            .engine()
            .tick(elapsed, Utc::now(), Local::now().naive_local());
        for event in events {
            tracing::info!(?event, "session event");
        }
    }
}
