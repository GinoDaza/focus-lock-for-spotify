//! Enforcers act on the system while a session is active. Today there is
//! one, for the Spotify desktop app.
//!
//! The Spotify extension does the real filtering. This side makes the
//! setup fail closed: during a session, a Spotify client that runs without
//! the extension reporting in gets closed.

use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use crate::app::App;

/// How often enforcers look at the system.
pub const CHECK_PERIOD: Duration = Duration::from_secs(2);

/// How long a newly seen Spotify client may run before the extension's
/// first heartbeat. Startup and loading the extension take a few seconds.
pub const STARTUP_GRACE: Duration = Duration::from_secs(20);

/// How long the extension may go quiet once it has reported in. The
/// extension beats every 5 s, but Chromium can throttle timers in a hidden
/// window to once a minute, so this has to be generous.
pub const STALE_AFTER: Duration = Duration::from_mins(2);

/// What enforcers know at each check.
#[derive(Debug, Clone, Copy)]
pub struct Context {
    pub active: bool,
    pub last_heartbeat: Option<Instant>,
    pub now: Instant,
}

pub trait Enforcer: Send {
    fn check(&mut self, context: &Context);
}

/// Runs the enforcers every [`CHECK_PERIOD`]. Runs forever.
pub async fn run(app: App, mut enforcers: Vec<Box<dyn Enforcer>>) {
    let mut interval = tokio::time::interval(CHECK_PERIOD);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        let context = Context {
            active: app.engine().is_active(),
            last_heartbeat: app.last_heartbeat().map(tokio::time::Instant::into_std),
            now: Instant::now(),
        };
        for enforcer in &mut enforcers {
            enforcer.check(&context);
        }
    }
}

/// Identifies one run of the Spotify client, so a restart counts as a new
/// client that needs a fresh heartbeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientId(pub u32);

/// Access to the Spotify client's processes.
pub trait SpotifyProcesses: Send {
    /// The running client, if any.
    fn find(&mut self) -> Option<ClientId>;
    /// Terminates every Spotify process. Returns how many were stopped.
    fn terminate(&mut self) -> usize;
}

/// Decides when a Spotify client is running without the extension.
#[derive(Debug, Default)]
pub struct Watchdog {
    /// The client being watched and when it was first seen.
    seen: Option<(ClientId, Instant)>,
}

impl Watchdog {
    /// Returns whether the client must be closed.
    pub fn should_close(&mut self, client: Option<ClientId>, context: &Context) -> bool {
        let Some(client) = client.filter(|_| context.active) else {
            self.seen = None;
            return false;
        };
        let first_seen = match self.seen {
            Some((id, since)) if id == client => since,
            _ => {
                self.seen = Some((client, context.now));
                context.now
            }
        };
        let close = match context.last_heartbeat.filter(|&beat| beat >= first_seen) {
            Some(beat) => context.now.duration_since(beat) > STALE_AFTER,
            None => context.now.duration_since(first_seen) > STARTUP_GRACE,
        };
        if close {
            self.seen = None;
        }
        close
    }
}

pub struct SpotifyEnforcer<P> {
    processes: P,
    watchdog: Watchdog,
}

impl<P: SpotifyProcesses> SpotifyEnforcer<P> {
    pub fn new(processes: P) -> Self {
        Self {
            processes,
            watchdog: Watchdog::default(),
        }
    }
}

impl<P: SpotifyProcesses> Enforcer for SpotifyEnforcer<P> {
    fn check(&mut self, context: &Context) {
        // Only look at processes during a session; idle checks are free.
        let client = if context.active {
            self.processes.find()
        } else {
            None
        };
        if self.watchdog.should_close(client, context) {
            let closed = self.processes.terminate();
            tracing::warn!(
                closed,
                "closed Spotify: the Tunelock extension did not report in"
            );
        }
    }
}

/// Finds Spotify through the OS process list.
pub struct SystemProcesses {
    system: System,
}

const SPOTIFY_EXE: &str = "spotify.exe";

impl SystemProcesses {
    #[must_use]
    pub fn new() -> Self {
        Self {
            system: System::new(),
        }
    }

    fn spotify(&mut self) -> impl Iterator<Item = &sysinfo::Process> {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing(),
        );
        self.system
            .processes()
            .values()
            .filter(|p| p.name().eq_ignore_ascii_case(SPOTIFY_EXE))
    }
}

impl Default for SystemProcesses {
    fn default() -> Self {
        Self::new()
    }
}

impl SpotifyProcesses for SystemProcesses {
    fn find(&mut self) -> Option<ClientId> {
        // The client is a tree of `Spotify.exe` processes; the oldest one
        // is the main process and identifies this run.
        self.spotify()
            .min_by_key(|p| (p.start_time(), p.pid()))
            .map(|p| ClientId(Pid::as_u32(p.pid())))
    }

    fn terminate(&mut self) -> usize {
        self.spotify().filter(|p| p.kill()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPOTIFY: Option<ClientId> = Some(ClientId(42));

    fn at(start: Instant, secs: u64, heartbeat: Option<u64>) -> Context {
        Context {
            active: true,
            last_heartbeat: heartbeat.map(|s| start + Duration::from_secs(s)),
            now: start + Duration::from_secs(secs),
        }
    }

    #[test]
    fn closes_a_client_that_never_reports_in() {
        let start = Instant::now();
        let mut dog = Watchdog::default();
        assert!(!dog.should_close(SPOTIFY, &at(start, 0, None)));
        assert!(!dog.should_close(SPOTIFY, &at(start, 20, None)));
        assert!(dog.should_close(SPOTIFY, &at(start, 21, None)));
    }

    #[test]
    fn keeps_a_client_whose_extension_reports_in() {
        let start = Instant::now();
        let mut dog = Watchdog::default();
        assert!(!dog.should_close(SPOTIFY, &at(start, 0, None)));
        assert!(!dog.should_close(SPOTIFY, &at(start, 60, Some(5))));
        assert!(!dog.should_close(SPOTIFY, &at(start, 125, Some(5))));
        assert!(dog.should_close(SPOTIFY, &at(start, 126, Some(5))));
    }

    #[test]
    fn a_restarted_client_needs_a_new_heartbeat() {
        let start = Instant::now();
        let mut dog = Watchdog::default();
        assert!(!dog.should_close(SPOTIFY, &at(start, 0, Some(0))));
        let restarted = Some(ClientId(7));
        assert!(!dog.should_close(restarted, &at(start, 10, Some(9))));
        assert!(dog.should_close(restarted, &at(start, 31, Some(9))));
    }

    #[test]
    fn does_nothing_without_a_session() {
        let start = Instant::now();
        let mut dog = Watchdog::default();
        let idle = Context {
            active: false,
            ..at(start, 1_000, None)
        };
        assert!(!dog.should_close(SPOTIFY, &idle));
        // A session that starts later gives the client a fresh grace period.
        assert!(!dog.should_close(SPOTIFY, &at(start, 1_001, None)));
        assert!(!dog.should_close(SPOTIFY, &at(start, 1_020, None)));
    }

    struct FakeProcesses {
        running: bool,
        terminated: usize,
    }

    impl SpotifyProcesses for FakeProcesses {
        fn find(&mut self) -> Option<ClientId> {
            self.running.then_some(ClientId(1))
        }

        fn terminate(&mut self) -> usize {
            self.running = false;
            self.terminated += 1;
            1
        }
    }

    #[test]
    fn enforcer_terminates_spotify() {
        let start = Instant::now();
        let mut enforcer = SpotifyEnforcer::new(FakeProcesses {
            running: true,
            terminated: 0,
        });
        enforcer.check(&at(start, 0, None));
        enforcer.check(&at(start, 30, None));
        assert_eq!(enforcer.processes.terminated, 1);
        assert!(!enforcer.processes.running);
    }
}
