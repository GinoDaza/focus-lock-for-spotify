use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::session::validate_duration;
use crate::{Allowlist, Error, LockMode, Result, Session};

/// A saved session template, e.g. "Study 10 h, locked".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preset {
    name: String,
    mode: LockMode,
    duration: Duration,
}

impl Preset {
    /// # Errors
    /// [`Error::EmptyName`] or a duration error.
    pub fn new(name: &str, mode: LockMode, duration: Duration) -> Result<Self> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::EmptyName);
        }
        validate_duration(duration)?;
        Ok(Self {
            name: name.to_owned(),
            mode,
            duration,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn mode(&self) -> LockMode {
        self.mode
    }

    #[must_use]
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// # Errors
    /// Only if the preset was built without validation (e.g. edited on disk).
    pub fn to_session(&self, allowlist: Allowlist) -> Result<Session> {
        Session::new(self.mode, self.duration, allowlist)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PlaylistId;

    #[test]
    fn validates_and_trims_name() {
        assert_eq!(
            Preset::new("   ", LockMode::Locked, Duration::from_mins(1)),
            Err(Error::EmptyName)
        );
        let preset = Preset::new(" Study ", LockMode::Locked, Duration::from_mins(1)).unwrap();
        assert_eq!(preset.name(), "Study");
    }

    #[test]
    fn builds_a_session() {
        let preset =
            Preset::new("Exam week", LockMode::Locked, Duration::from_hours(5 * 24)).unwrap();
        let allowlist =
            Allowlist::new([PlaylistId::parse("37i9dQZF1DX8NTLI2TtZa6").unwrap()]).unwrap();
        let session = preset.to_session(allowlist).unwrap();
        assert_eq!(session.mode(), LockMode::Locked);
        assert_eq!(session.remaining(), Duration::from_hours(5 * 24));
    }
}
