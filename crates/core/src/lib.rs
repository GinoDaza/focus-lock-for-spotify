//! Domain model of Focus Lock.
//!
//! This crate is pure: no I/O, no system clock. The service feeds it elapsed
//! time and the current local time, which keeps every rule testable.

mod error;
mod playlist;
mod preset;
mod schedule;
pub mod serde_secs;
mod session;

pub use error::{Error, Result};
pub use playlist::{Allowlist, PlaylistId};
pub use preset::Preset;
pub use schedule::Schedule;
pub use session::{LockMode, LockState, MAX_DURATION, Session, Transition};

use std::time::Duration;

/// Formats a duration for humans, e.g. `2d 3h 5m` or `45s`.
#[must_use]
pub fn format_duration(duration: &Duration) -> String {
    let total = duration.as_secs();
    let (days, hours, minutes, seconds) = (
        total / 86_400,
        total % 86_400 / 3_600,
        total % 3_600 / 60,
        total % 60,
    );
    let parts: Vec<String> = [(days, "d"), (hours, "h"), (minutes, "m")]
        .into_iter()
        .filter(|(value, _)| *value > 0)
        .map(|(value, unit)| format!("{value}{unit}"))
        .collect();
    if parts.is_empty() {
        format!("{seconds}s")
    } else {
        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_durations() {
        assert_eq!(format_duration(&Duration::from_secs(45)), "45s");
        assert_eq!(format_duration(&Duration::from_secs(5 * 60 + 7)), "5m");
        assert_eq!(
            format_duration(&Duration::from_mins(2 * 24 * 60 + 3 * 60 + 5)),
            "2d 3h 5m"
        );
        assert_eq!(format_duration(&Duration::from_hours(1)), "1h");
    }
}
