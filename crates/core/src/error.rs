use std::time::Duration;

use crate::format_duration;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("not a valid Spotify playlist link or ID: {0:?}")]
    InvalidPlaylist(String),
    #[error("the allowlist must contain at least one playlist")]
    EmptyAllowlist,
    #[error("duration must be greater than zero")]
    ZeroDuration,
    #[error("duration exceeds the maximum of {max_days} days")]
    DurationTooLong { max_days: u64 },
    #[error("a focus session is already active")]
    AlreadyActive,
    #[error("no focus session is active")]
    NotActive,
    #[error("the session is locked; {} remaining", format_duration(.remaining))]
    Locked { remaining: Duration },
    #[error("name must not be empty")]
    EmptyName,
    #[error("a schedule needs at least one day")]
    NoDays,
    #[error("schedule start and end times must differ")]
    EmptyWindow,
}
