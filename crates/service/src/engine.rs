//! Applies requests and the passage of time to the saved [`Data`].
//!
//! Time policy:
//! - While running, a tick credits `min(monotonic elapsed, wall-clock
//!   elapsed)`, so changing the system clock never shortens a session.
//! - Time spent shut down or asleep is credited from the wall clock since
//!   the last checkpoint ([`Engine::credit_downtime`]).

use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, Utc};
use tunelock_core::{Error as CoreError, LockMode, MAX_DURATION, Session, Transition};
use tunelock_protocol::{
    ErrorBody, ErrorCode, ExtensionState, PROTOCOL_VERSION, Request, Response, SessionInfo, Status,
};

use crate::store::{Data, Store};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Started,
    Ended,
}

pub struct Engine {
    data: Data,
    store: Box<dyn Store>,
}

type Reply = Result<Response, ErrorBody>;

impl Engine {
    /// Loads saved data (or starts fresh if there is none or it is corrupt)
    /// and credits the downtime since the last checkpoint.
    #[must_use]
    pub fn load(store: Box<dyn Store>, now: DateTime<Utc>) -> Self {
        let data = store.load().unwrap_or_else(|error| {
            tracing::error!("could not load saved data, starting fresh: {error:#}");
            None
        });
        let mut engine = Self {
            data: data.unwrap_or_default(),
            store,
        };
        engine.credit_downtime(now);
        engine.save();
        engine
    }

    /// Credits wall-clock time since the last checkpoint. Call on startup
    /// and when the machine resumes from sleep.
    pub fn credit_downtime(&mut self, now: DateTime<Utc>) -> Option<Event> {
        let gap = self.wall_elapsed(now).unwrap_or_default();
        self.data.checkpoint = Some(now);
        self.advance(gap)
    }

    /// Credits the time spent asleep and saves. Call when the machine
    /// resumes from sleep.
    pub fn resume(&mut self, now: DateTime<Utc>) -> Option<Event> {
        let event = self.credit_downtime(now);
        self.save();
        event
    }

    /// Regular heartbeat of the service, every few seconds.
    pub fn tick(
        &mut self,
        monotonic: Duration,
        now: DateTime<Utc>,
        local: NaiveDateTime,
    ) -> Vec<Event> {
        let credited = self
            .wall_elapsed(now)
            .map_or(monotonic, |wall| wall.min(monotonic));
        self.data.checkpoint = Some(now);

        let events: Vec<Event> = [self.advance(credited), self.apply_schedules(local)]
            .into_iter()
            .flatten()
            .collect();
        self.save();
        events
    }

    pub fn handle(&mut self, request: Request, local: NaiveDateTime) -> Response {
        let mutates = !matches!(
            request,
            Request::Status | Request::ListPresets | Request::GetAllowlist | Request::ListSchedules
        );
        let response = self.apply(request, local).unwrap_or_else(Response::Error);
        if mutates && !matches!(response, Response::Error(_)) && !self.save() {
            return Response::Error(ErrorBody {
                code: ErrorCode::Internal,
                message: "the change was applied but could not be saved".into(),
            });
        }
        response
    }

    #[must_use]
    pub fn extension_state(&self) -> ExtensionState {
        ExtensionState::from(&self.data.state)
    }

    #[must_use]
    pub fn status(&self) -> Status {
        Status {
            protocol_version: PROTOCOL_VERSION,
            service_version: env!("CARGO_PKG_VERSION").to_owned(),
            session: self.data.state.session().map(SessionInfo::from),
        }
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.data.state.is_active()
    }

    /// Whether a session that cannot be cancelled is running.
    #[must_use]
    pub fn is_locked(&self) -> bool {
        self.data
            .state
            .session()
            .is_some_and(|session| session.mode() == LockMode::Locked)
    }

    fn apply(&mut self, request: Request, local: NaiveDateTime) -> Reply {
        let data = &mut self.data;
        match request {
            Request::Status => return Ok(Response::Status(self.status())),
            Request::StartSession {
                mode,
                duration,
                allowlist,
            } => {
                let session = Session::new(mode, duration, allowlist).map_err(core)?;
                data.state.start(session).map_err(core)?;
            }
            Request::StartPreset { name } => {
                let preset = data
                    .presets
                    .iter()
                    .find(|p| p.name() == name)
                    .ok_or_else(|| not_found(format!("no preset named {name:?}")))?;
                let allowlist = data
                    .allowlist
                    .clone()
                    .ok_or_else(|| not_found("no allowed playlists are saved yet".into()))?;
                let session = preset.to_session(allowlist).map_err(core)?;
                data.state.start(session).map_err(core)?;
            }
            Request::Cancel => {
                data.state.cancel().map_err(core)?;
                data.schedules_paused_until = data
                    .schedules
                    .iter()
                    .filter_map(|s| s.active_until(local))
                    .max();
            }
            Request::Extend { by } => data.state.extend(by).map_err(core)?,
            Request::Harden => data.state.harden().map_err(core)?,
            Request::ListPresets => {
                return Ok(Response::Presets {
                    presets: data.presets.clone(),
                });
            }
            Request::SavePreset { preset } => {
                data.presets.retain(|p| p.name() != preset.name());
                data.presets.push(preset);
            }
            Request::DeletePreset { name } => {
                let before = data.presets.len();
                data.presets.retain(|p| p.name() != name);
                if data.presets.len() == before {
                    return Err(not_found(format!("no preset named {name:?}")));
                }
            }
            Request::GetAllowlist => {
                return Ok(Response::Allowlist {
                    allowlist: data.allowlist.clone(),
                });
            }
            // A running session keeps its own copy, so this never affects it.
            Request::SetAllowlist { allowlist } => data.allowlist = Some(allowlist),
            Request::ListSchedules => {
                return Ok(Response::Schedules {
                    schedules: data.schedules.clone(),
                });
            }
            Request::SaveSchedule { schedule } => {
                let replaces = data.schedules.iter().any(|s| s.name() == schedule.name());
                if replaces {
                    ensure_not_locked(data)?;
                }
                data.schedules.retain(|s| s.name() != schedule.name());
                data.schedules.push(schedule);
            }
            Request::DeleteSchedule { name } => {
                if !data.schedules.iter().any(|s| s.name() == name) {
                    return Err(not_found(format!("no schedule named {name:?}")));
                }
                ensure_not_locked(data)?;
                data.schedules.retain(|s| s.name() != name);
            }
        }
        Ok(Response::Done)
    }

    /// Starts a session if a schedule window is open and nothing is running.
    fn apply_schedules(&mut self, local: NaiveDateTime) -> Option<Event> {
        let data = &mut self.data;
        if data.state.is_active() {
            return None;
        }
        if let Some(until) = data.schedules_paused_until {
            if local < until {
                return None;
            }
            data.schedules_paused_until = None;
        }
        let (schedule, end) = data
            .schedules
            .iter()
            .filter_map(|s| s.active_until(local).map(|end| (s, end)))
            .max_by_key(|(_, end)| *end)?;
        let Some(allowlist) = data.allowlist.clone() else {
            tracing::warn!(
                "schedule {:?} is due but no playlists are allowed yet",
                schedule.name()
            );
            return None;
        };
        let duration = (end - local).to_std().ok()?.min(MAX_DURATION);
        let session = Session::new(schedule.mode(), duration, allowlist).ok()?;
        data.state.start(session).ok()?;
        Some(Event::Started)
    }

    fn advance(&mut self, delta: Duration) -> Option<Event> {
        (self.data.state.advance(delta) == Transition::Expired).then_some(Event::Ended)
    }

    fn wall_elapsed(&self, now: DateTime<Utc>) -> Option<Duration> {
        let checkpoint = self.data.checkpoint?;
        // A clock moved backwards yields zero, never a negative credit.
        Some((now - checkpoint).to_std().unwrap_or_default())
    }

    /// Persists the data, logging failures. Returns whether it succeeded.
    fn save(&mut self) -> bool {
        match self.store.save(&self.data) {
            Ok(()) => true,
            Err(error) => {
                tracing::error!("could not save data: {error:#}");
                false
            }
        }
    }
}

#[allow(clippy::needless_pass_by_value)] // used as `map_err(core)`
fn core(error: CoreError) -> ErrorBody {
    ErrorBody::from(&error)
}

fn not_found(message: String) -> ErrorBody {
    ErrorBody {
        code: ErrorCode::NotFound,
        message,
    }
}

fn ensure_not_locked(data: &Data) -> Result<(), ErrorBody> {
    match data.state.session() {
        Some(session) if session.mode() == LockMode::Locked => Err(ErrorBody {
            code: ErrorCode::Locked,
            message: "schedules cannot be changed or removed during a locked session".into(),
        }),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime, TimeDelta, Weekday};
    use tunelock_core::{Allowlist, PlaylistId, Preset, Schedule};

    use super::*;
    use crate::store::MemoryStore;

    const HOUR: Duration = Duration::from_hours(1);

    fn allowlist() -> Allowlist {
        Allowlist::new([PlaylistId::parse("37i9dQZF1DX8NTLI2TtZa6").unwrap()]).unwrap()
    }

    fn t0() -> DateTime<Utc> {
        "2026-10-16T12:00:00Z".parse().unwrap()
    }

    /// Friday 2026-10-16 at the given local time.
    fn local(time: &str) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 16)
            .unwrap()
            .and_time(NaiveTime::parse_from_str(time, "%H:%M").unwrap())
    }

    fn engine() -> Engine {
        Engine::load(Box::new(MemoryStore::default()), t0())
    }

    fn start(engine: &mut Engine, mode: LockMode, duration: Duration) {
        let response = engine.handle(
            Request::StartSession {
                mode,
                duration,
                allowlist: allowlist(),
            },
            local("12:00"),
        );
        assert_eq!(response, Response::Done);
    }

    fn remaining(engine: &Engine) -> Option<Duration> {
        engine.status().session.map(|s| s.remaining)
    }

    fn error_code(response: &Response) -> Option<ErrorCode> {
        match response {
            Response::Error(body) => Some(body.code),
            _ => None,
        }
    }

    #[test]
    fn tick_counts_monotonic_time() {
        let mut engine = engine();
        start(&mut engine, LockMode::Locked, 2 * HOUR);
        engine.tick(HOUR, t0() + TimeDelta::hours(1), local("13:00"));
        assert_eq!(remaining(&engine), Some(HOUR));
    }

    #[test]
    fn moving_the_clock_forward_does_not_shorten_a_session() {
        let mut engine = engine();
        start(&mut engine, LockMode::Locked, 2 * HOUR);
        engine.tick(
            Duration::from_secs(5),
            t0() + TimeDelta::days(3),
            local("12:00"),
        );
        assert_eq!(remaining(&engine), Some(Duration::from_secs(2 * 3_600 - 5)));
    }

    #[test]
    fn moving_the_clock_backward_does_not_crash_or_credit() {
        let mut engine = engine();
        start(&mut engine, LockMode::Locked, 2 * HOUR);
        engine.tick(
            Duration::from_secs(5),
            t0() - TimeDelta::hours(5),
            local("12:00"),
        );
        assert_eq!(remaining(&engine), Some(2 * HOUR));
    }

    #[test]
    fn downtime_counts_and_can_end_the_session() {
        let mut store = MemoryStore::default();
        {
            let mut engine = Engine::load(Box::new(MemoryStore::default()), t0());
            start(&mut engine, LockMode::Locked, 10 * HOUR);
            store.save(&engine.data).unwrap();
        }
        let engine = Engine::load(Box::new(store), t0() + TimeDelta::hours(3));
        assert_eq!(remaining(&engine), Some(7 * HOUR));

        let mut engine = engine;
        assert_eq!(
            engine.credit_downtime(t0() + TimeDelta::hours(20)),
            Some(Event::Ended)
        );
        assert!(!engine.is_active());
    }

    #[test]
    fn resume_credits_sleep_and_saves() {
        let mut engine = engine();
        start(&mut engine, LockMode::Locked, 2 * HOUR);
        assert_eq!(engine.resume(t0() + TimeDelta::hours(1)), None);
        assert_eq!(remaining(&engine), Some(HOUR));
        assert!(engine.is_locked());
        assert_eq!(
            engine.resume(t0() + TimeDelta::hours(3)),
            Some(Event::Ended)
        );
        assert!(!engine.is_locked());
    }

    #[test]
    fn locked_session_cannot_be_cancelled() {
        let mut engine = engine();
        start(&mut engine, LockMode::Locked, HOUR);
        let response = engine.handle(Request::Cancel, local("12:00"));
        assert_eq!(error_code(&response), Some(ErrorCode::Locked));
        assert!(engine.is_active());
    }

    #[test]
    fn start_preset_needs_preset_and_allowlist() {
        let mut engine = engine();
        let start = || Request::StartPreset {
            name: "Study".into(),
        };
        assert_eq!(
            error_code(&engine.handle(start(), local("12:00"))),
            Some(ErrorCode::NotFound)
        );

        let preset = Preset::new("Study", LockMode::Locked, 2 * HOUR).unwrap();
        engine.handle(Request::SavePreset { preset }, local("12:00"));
        assert_eq!(
            error_code(&engine.handle(start(), local("12:00"))),
            Some(ErrorCode::NotFound)
        );

        engine.handle(
            Request::SetAllowlist {
                allowlist: allowlist(),
            },
            local("12:00"),
        );
        assert_eq!(engine.handle(start(), local("12:00")), Response::Done);
        assert_eq!(remaining(&engine), Some(2 * HOUR));
    }

    fn workday_schedule(mode: LockMode) -> Schedule {
        Schedule::new(
            "Work",
            [Weekday::Fri],
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
            mode,
        )
        .unwrap()
    }

    #[test]
    fn schedule_starts_a_session_until_the_window_ends() {
        let mut engine = engine();
        engine.handle(
            Request::SetAllowlist {
                allowlist: allowlist(),
            },
            local("08:00"),
        );
        engine.handle(
            Request::SaveSchedule {
                schedule: workday_schedule(LockMode::Locked),
            },
            local("08:00"),
        );

        assert!(engine.tick(Duration::ZERO, t0(), local("08:59")).is_empty());
        assert_eq!(
            engine.tick(Duration::ZERO, t0(), local("10:00")),
            vec![Event::Started]
        );
        assert_eq!(remaining(&engine), Some(8 * HOUR));
    }

    #[test]
    fn cancelled_schedule_does_not_restart_in_the_same_window() {
        let mut engine = engine();
        engine.handle(
            Request::SetAllowlist {
                allowlist: allowlist(),
            },
            local("08:00"),
        );
        engine.handle(
            Request::SaveSchedule {
                schedule: workday_schedule(LockMode::Flexible),
            },
            local("08:00"),
        );
        engine.tick(Duration::ZERO, t0(), local("10:00"));
        assert_eq!(
            engine.handle(Request::Cancel, local("10:05")),
            Response::Done
        );
        assert!(engine.tick(Duration::ZERO, t0(), local("10:10")).is_empty());
        assert!(!engine.is_active());
    }

    #[test]
    fn schedules_cannot_be_removed_during_a_locked_session() {
        let mut engine = engine();
        engine.handle(
            Request::SaveSchedule {
                schedule: workday_schedule(LockMode::Locked),
            },
            local("08:00"),
        );
        start(&mut engine, LockMode::Locked, HOUR);
        let delete = engine.handle(
            Request::DeleteSchedule {
                name: "Work".into(),
            },
            local("08:00"),
        );
        assert_eq!(error_code(&delete), Some(ErrorCode::Locked));

        let replace = engine.handle(
            Request::SaveSchedule {
                schedule: workday_schedule(LockMode::Flexible),
            },
            local("08:00"),
        );
        assert_eq!(error_code(&replace), Some(ErrorCode::Locked));
    }

    #[test]
    fn extension_state_reflects_the_session() {
        let mut engine = engine();
        assert!(!engine.extension_state().active);
        start(&mut engine, LockMode::Locked, HOUR);
        let state = engine.extension_state();
        assert!(state.active);
        assert_eq!(
            state.allowed_uris,
            ["spotify:playlist:37i9dQZF1DX8NTLI2TtZa6"]
        );
    }
}
