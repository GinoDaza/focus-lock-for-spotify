//! Shared state and the background ticker.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use chrono::{Local, Utc};
use tokio::time::{Instant, MissedTickBehavior};

use crate::engine::Engine;

pub const TICK_PERIOD: Duration = Duration::from_secs(5);

/// Handle shared by the ticker, both servers, the enforcers and the
/// Windows service control handler.
#[derive(Clone)]
pub struct App {
    engine: Arc<Mutex<Engine>>,
    last_heartbeat: Arc<Mutex<Option<Instant>>>,
    /// When the ticker last credited time.
    tick_baseline: Arc<Mutex<Instant>>,
}

impl App {
    #[must_use]
    pub fn new(engine: Engine) -> Self {
        Self {
            engine: Arc::new(Mutex::new(engine)),
            last_heartbeat: Arc::default(),
            tick_baseline: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Locks the engine. A panic in another holder must not take the
    /// service down, so poisoning is ignored.
    pub fn engine(&self) -> MutexGuard<'_, Engine> {
        lock(&self.engine)
    }

    pub fn record_heartbeat(&self) {
        *lock(&self.last_heartbeat) = Some(Instant::now());
    }

    /// When the Spotify extension last reported in, if ever.
    #[must_use]
    pub fn last_heartbeat(&self) -> Option<Instant> {
        *lock(&self.last_heartbeat)
    }

    #[must_use]
    pub fn is_locked(&self) -> bool {
        self.engine().is_locked()
    }

    /// Called when the machine resumes from sleep: credits the time asleep
    /// from the wall clock and restarts the ticker's monotonic baseline so
    /// that time is not counted twice.
    pub fn resume(&self) {
        let mut engine = self.engine();
        let event = engine.resume(Utc::now());
        *lock(&self.tick_baseline) = Instant::now();
        drop(engine);
        tracing::info!(?event, "resumed from sleep");
    }

    /// Advances the engine by the monotonic time since the last tick.
    fn tick(&self) {
        let mut engine = self.engine();
        let elapsed = {
            let mut baseline = lock(&self.tick_baseline);
            let now = Instant::now();
            let elapsed = now - *baseline;
            *baseline = now;
            elapsed
        };
        let events = engine.tick(elapsed, Utc::now(), Local::now().naive_local());
        drop(engine);
        for event in events {
            tracing::info!(?event, "session event");
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Advances the engine every [`TICK_PERIOD`]. Runs forever.
pub async fn run_ticker(app: App) {
    let mut interval = tokio::time::interval(TICK_PERIOD);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        app.tick();
    }
}
