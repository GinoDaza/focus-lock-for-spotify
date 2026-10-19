use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{Allowlist, Error, Result};

/// Upper bound for a session, so a typo cannot lock someone out for a year.
pub const MAX_DURATION: Duration = Duration::from_hours(30 * 24);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockMode {
    /// Can be cancelled at any time.
    Flexible,
    /// Cannot be cancelled or shortened until it ends.
    Locked,
}

/// A focus session. Time advances only through [`LockState::advance`], so
/// the service decides which clock to trust.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SessionRepr")]
pub struct Session {
    mode: LockMode,
    #[serde(serialize_with = "crate::serde_secs::serialize")]
    duration: Duration,
    #[serde(serialize_with = "crate::serde_secs::serialize")]
    elapsed: Duration,
    allowlist: Allowlist,
}

/// Unvalidated wire form of [`Session`]; deserialization goes through it.
#[derive(Deserialize)]
struct SessionRepr {
    mode: LockMode,
    #[serde(with = "crate::serde_secs")]
    duration: Duration,
    #[serde(with = "crate::serde_secs")]
    elapsed: Duration,
    allowlist: Allowlist,
}

impl TryFrom<SessionRepr> for Session {
    type Error = Error;

    fn try_from(repr: SessionRepr) -> Result<Self> {
        let mut session = Self::new(repr.mode, repr.duration, repr.allowlist)?;
        session.elapsed = repr.elapsed.min(session.duration);
        Ok(session)
    }
}

impl Session {
    /// # Errors
    /// [`Error::ZeroDuration`] or [`Error::DurationTooLong`].
    pub fn new(mode: LockMode, duration: Duration, allowlist: Allowlist) -> Result<Self> {
        validate_duration(duration)?;
        Ok(Self {
            mode,
            duration,
            elapsed: Duration::ZERO,
            allowlist,
        })
    }

    #[must_use]
    pub fn mode(&self) -> LockMode {
        self.mode
    }

    #[must_use]
    pub fn duration(&self) -> Duration {
        self.duration
    }

    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    #[must_use]
    pub fn remaining(&self) -> Duration {
        self.duration.saturating_sub(self.elapsed)
    }

    #[must_use]
    pub fn allowlist(&self) -> &Allowlist {
        &self.allowlist
    }
}

pub(crate) fn validate_duration(duration: Duration) -> Result<()> {
    if duration.is_zero() {
        Err(Error::ZeroDuration)
    } else if duration > MAX_DURATION {
        Err(Error::DurationTooLong {
            max_days: MAX_DURATION.as_secs() / 86_400,
        })
    } else {
        Ok(())
    }
}

/// What happened as a result of [`LockState::advance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    None,
    Expired,
}

/// The single source of truth for whether Focus Lock is restricting anything.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum LockState {
    #[default]
    Idle,
    Active(Session),
}

impl LockState {
    #[must_use]
    pub fn session(&self) -> Option<&Session> {
        match self {
            Self::Idle => None,
            Self::Active(session) => Some(session),
        }
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active(_))
    }

    /// # Errors
    /// [`Error::AlreadyActive`] if a session is running.
    pub fn start(&mut self, session: Session) -> Result<()> {
        if self.is_active() {
            return Err(Error::AlreadyActive);
        }
        *self = Self::Active(session);
        Ok(())
    }

    /// Ends a flexible session early.
    ///
    /// # Errors
    /// [`Error::NotActive`], or [`Error::Locked`] for a locked session.
    pub fn cancel(&mut self) -> Result<()> {
        match self {
            Self::Idle => Err(Error::NotActive),
            Self::Active(session) if session.mode == LockMode::Locked => Err(Error::Locked {
                remaining: session.remaining(),
            }),
            Self::Active(_) => {
                *self = Self::Idle;
                Ok(())
            }
        }
    }

    /// Adds time to the running session. Allowed in both modes: making a
    /// session longer never weakens it.
    ///
    /// # Errors
    /// [`Error::NotActive`], or a duration error if the new remaining time
    /// would be zero or exceed [`MAX_DURATION`].
    pub fn extend(&mut self, extra: Duration) -> Result<()> {
        let Self::Active(session) = self else {
            return Err(Error::NotActive);
        };
        validate_duration(extra)?;
        validate_duration(session.remaining().saturating_add(extra))?;
        session.duration = session.duration.saturating_add(extra);
        Ok(())
    }

    /// Turns a flexible session into a locked one. There is no way back.
    ///
    /// # Errors
    /// [`Error::NotActive`].
    pub fn harden(&mut self) -> Result<()> {
        let Self::Active(session) = self else {
            return Err(Error::NotActive);
        };
        session.mode = LockMode::Locked;
        Ok(())
    }

    /// Moves the session clock forward and ends it once its time is used up.
    pub fn advance(&mut self, delta: Duration) -> Transition {
        let Self::Active(session) = self else {
            return Transition::None;
        };
        session.elapsed = session.elapsed.saturating_add(delta).min(session.duration);
        if session.remaining().is_zero() {
            *self = Self::Idle;
            Transition::Expired
        } else {
            Transition::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PlaylistId;

    const HOUR: Duration = Duration::from_hours(1);

    fn allowlist() -> Allowlist {
        Allowlist::new([PlaylistId::parse("37i9dQZF1DX8NTLI2TtZa6").unwrap()]).unwrap()
    }

    fn active(mode: LockMode, duration: Duration) -> LockState {
        let mut state = LockState::Idle;
        state
            .start(Session::new(mode, duration, allowlist()).unwrap())
            .unwrap();
        state
    }

    #[test]
    fn rejects_invalid_durations() {
        assert_eq!(
            Session::new(LockMode::Locked, Duration::ZERO, allowlist()),
            Err(Error::ZeroDuration)
        );
        assert_eq!(
            Session::new(LockMode::Locked, MAX_DURATION + HOUR, allowlist()),
            Err(Error::DurationTooLong { max_days: 30 })
        );
        assert!(Session::new(LockMode::Locked, MAX_DURATION, allowlist()).is_ok());
    }

    #[test]
    fn cannot_start_twice() {
        let mut state = active(LockMode::Flexible, HOUR);
        let second = Session::new(LockMode::Flexible, HOUR, allowlist()).unwrap();
        assert_eq!(state.start(second), Err(Error::AlreadyActive));
    }

    #[test]
    fn flexible_session_can_be_cancelled() {
        let mut state = active(LockMode::Flexible, HOUR);
        assert_eq!(state.cancel(), Ok(()));
        assert_eq!(state, LockState::Idle);
        assert_eq!(state.cancel(), Err(Error::NotActive));
    }

    #[test]
    fn locked_session_refuses_cancel_until_it_expires() {
        let mut state = active(LockMode::Locked, 2 * HOUR);
        state.advance(HOUR / 2);
        assert_eq!(
            state.cancel(),
            Err(Error::Locked {
                remaining: HOUR + HOUR / 2
            })
        );
        assert!(state.is_active());

        assert_eq!(state.advance(HOUR + HOUR / 2), Transition::Expired);
        assert_eq!(state, LockState::Idle);
    }

    #[test]
    fn advance_is_cumulative_and_saturates() {
        let mut state = active(LockMode::Locked, HOUR);
        for _ in 0..59 {
            assert_eq!(state.advance(Duration::from_mins(1)), Transition::None);
        }
        assert_eq!(state.session().unwrap().remaining(), Duration::from_mins(1));
        assert_eq!(state.advance(10 * HOUR), Transition::Expired);
        assert_eq!(state.advance(HOUR), Transition::None);
    }

    #[test]
    fn extend_adds_time_in_both_modes() {
        for mode in [LockMode::Flexible, LockMode::Locked] {
            let mut state = active(mode, HOUR);
            state.advance(HOUR / 2);
            state.extend(HOUR).unwrap();
            assert_eq!(state.session().unwrap().remaining(), HOUR + HOUR / 2);
        }
    }

    #[test]
    fn extend_respects_limits() {
        let mut state = active(LockMode::Locked, MAX_DURATION);
        assert_eq!(state.extend(Duration::ZERO), Err(Error::ZeroDuration));
        assert_eq!(
            state.extend(HOUR),
            Err(Error::DurationTooLong { max_days: 30 })
        );
        state.advance(HOUR);
        assert_eq!(state.extend(HOUR), Ok(()));
        assert_eq!(LockState::Idle.extend(HOUR), Err(Error::NotActive));
    }

    #[test]
    fn harden_is_one_way() {
        let mut state = active(LockMode::Flexible, HOUR);
        state.harden().unwrap();
        assert_eq!(state.session().unwrap().mode(), LockMode::Locked);
        assert!(matches!(state.cancel(), Err(Error::Locked { .. })));
    }

    #[test]
    fn state_survives_a_serde_round_trip() {
        let mut state = active(LockMode::Locked, 10 * HOUR);
        state.advance(3 * HOUR);
        let json = serde_json::to_string(&state).unwrap();
        let restored: LockState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, state);
        assert_eq!(restored.session().unwrap().remaining(), 7 * HOUR);
    }

    #[test]
    fn json_shape_is_stable() {
        let json = serde_json::to_string(&active(LockMode::Locked, HOUR)).unwrap();
        assert_eq!(
            json,
            r#"{"state":"active","mode":"locked","duration":3600,"elapsed":0,"allowlist":["37i9dQZF1DX8NTLI2TtZa6"]}"#
        );
        assert_eq!(
            serde_json::to_string(&LockState::Idle).unwrap(),
            r#"{"state":"idle"}"#
        );
    }

    #[test]
    fn deserialize_validates_and_clamps_elapsed() {
        let zero = r#"{"state":"active","mode":"locked","duration":0,"elapsed":0,"allowlist":["37i9dQZF1DX8NTLI2TtZa6"]}"#;
        assert!(serde_json::from_str::<LockState>(zero).is_err());

        let overrun = r#"{"state":"active","mode":"locked","duration":60,"elapsed":999,"allowlist":["37i9dQZF1DX8NTLI2TtZa6"]}"#;
        let state: LockState = serde_json::from_str(overrun).unwrap();
        assert_eq!(state.session().unwrap().remaining(), Duration::ZERO);
    }
}
