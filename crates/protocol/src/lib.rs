//! Wire protocol between `tunelock-service` and its clients.
//!
//! Two channels exist:
//! - **Control** (named pipe [`PIPE_NAME`]): the desktop app and the CLI
//!   send [`Request`]s and receive [`Response`]s, one JSON object per line.
//! - **Extension** (read-only, loopback only): the Spotify extension polls
//!   [`ExtensionState`]. Nothing on this channel can weaken a session.
//!
//! Durations are always whole seconds in JSON.

use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tunelock_core::{
    Allowlist, LockMode, LockState, PlaylistId, Preset, Schedule, Session, serde_secs,
};

/// Bumped on any breaking change to the types in this crate.
pub const PROTOCOL_VERSION: u32 = 1;

/// Named pipe of the control channel.
pub const PIPE_NAME: &str = r"\\.\pipe\tunelock";

/// Loopback port of the extension channel.
pub const EXTENSION_PORT: u16 = 39_817;

/// Upper bound for one line on the control channel.
pub const MAX_LINE_LEN: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Status,
    /// `duration` is validated when the service builds the session.
    StartSession {
        mode: LockMode,
        #[serde(with = "serde_secs")]
        duration: Duration,
        allowlist: Allowlist,
    },
    /// Starts a session from a saved preset and the saved allowlist.
    StartPreset {
        name: String,
    },
    Cancel,
    Extend {
        #[serde(with = "serde_secs")]
        by: Duration,
    },
    /// Turns the running flexible session into a locked one.
    Harden,
    ListPresets,
    SavePreset {
        preset: Preset,
    },
    DeletePreset {
        name: String,
    },
    GetAllowlist,
    SetAllowlist {
        allowlist: Allowlist,
    },
    ListSchedules,
    SaveSchedule {
        schedule: Schedule,
    },
    DeleteSchedule {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Status(Status),
    Presets { presets: Vec<Preset> },
    Allowlist { allowlist: Option<Allowlist> },
    Schedules { schedules: Vec<Schedule> },
    Done,
    Error(ErrorBody),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    pub protocol_version: u32,
    pub service_version: String,
    pub session: Option<SessionInfo>,
}

/// A running session as clients see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub mode: LockMode,
    #[serde(with = "serde_secs")]
    pub duration: Duration,
    #[serde(with = "serde_secs")]
    pub remaining: Duration,
    pub allowlist: Allowlist,
}

impl From<&Session> for SessionInfo {
    fn from(session: &Session) -> Self {
        Self {
            mode: session.mode(),
            duration: session.duration(),
            remaining: session.remaining(),
            allowlist: session.allowlist().clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidInput,
    AlreadyActive,
    NotActive,
    Locked,
    NotFound,
    Internal,
}

impl From<&tunelock_core::Error> for ErrorBody {
    fn from(error: &tunelock_core::Error) -> Self {
        use tunelock_core::Error as E;
        let code = match error {
            E::AlreadyActive => ErrorCode::AlreadyActive,
            E::NotActive => ErrorCode::NotActive,
            E::Locked { .. } => ErrorCode::Locked,
            E::InvalidPlaylist(_)
            | E::EmptyAllowlist
            | E::ZeroDuration
            | E::DurationTooLong { .. }
            | E::EmptyName
            | E::NoDays
            | E::EmptyWindow => ErrorCode::InvalidInput,
        };
        Self {
            code,
            message: error.to_string(),
        }
    }
}

impl From<tunelock_core::Error> for Response {
    fn from(error: tunelock_core::Error) -> Self {
        Self::Error(ErrorBody::from(&error))
    }
}

/// What the Spotify extension needs to know. Playlists are given as
/// `spotify:playlist:<id>` URIs, ready to compare with the player context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionState {
    pub protocol_version: u32,
    pub active: bool,
    #[serde(with = "serde_secs")]
    pub remaining: Duration,
    pub allowed_uris: Vec<String>,
}

impl From<&LockState> for ExtensionState {
    fn from(state: &LockState) -> Self {
        let session = state.session();
        Self {
            protocol_version: PROTOCOL_VERSION,
            active: session.is_some(),
            remaining: session.map_or(Duration::ZERO, Session::remaining),
            allowed_uris: session
                .map(|s| s.allowlist().iter().map(PlaylistId::uri).collect())
                .unwrap_or_default(),
        }
    }
}

/// Encodes a message as one line of JSON, newline included.
///
/// # Errors
/// Only if `T`'s `Serialize` impl fails, which the types here never do.
pub fn encode_line<T: Serialize>(message: &T) -> serde_json::Result<String> {
    let mut line = serde_json::to_string(message)?;
    line.push('\n');
    Ok(line)
}

/// Decodes one line produced by [`encode_line`].
///
/// # Errors
/// If the line is not valid JSON for `T`, including failed validation.
pub fn decode_line<T: DeserializeOwned>(line: &str) -> serde_json::Result<T> {
    serde_json::from_str(line.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "37i9dQZF1DX8NTLI2TtZa6";

    fn allowlist() -> Allowlist {
        Allowlist::new([PlaylistId::parse(ID).unwrap()]).unwrap()
    }

    #[test]
    fn requests_round_trip() {
        let requests = [
            Request::Status,
            Request::StartSession {
                mode: LockMode::Locked,
                duration: Duration::from_hours(2),
                allowlist: allowlist(),
            },
            Request::StartPreset {
                name: "Study".into(),
            },
            Request::Cancel,
            Request::Extend {
                by: Duration::from_mins(30),
            },
            Request::Harden,
            Request::SetAllowlist {
                allowlist: allowlist(),
            },
        ];
        for request in requests {
            let line = encode_line(&request).unwrap();
            assert!(line.ends_with('\n') && !line[..line.len() - 1].contains('\n'));
            assert_eq!(decode_line::<Request>(&line).unwrap(), request);
        }
    }

    #[test]
    fn request_json_shape_is_stable() {
        let request = Request::StartSession {
            mode: LockMode::Locked,
            duration: Duration::from_hours(2),
            allowlist: allowlist(),
        };
        assert_eq!(
            serde_json::to_string(&request).unwrap(),
            format!(
                r#"{{"type":"start_session","mode":"locked","duration":7200,"allowlist":["{ID}"]}}"#
            )
        );
    }

    #[test]
    fn invalid_requests_are_rejected() {
        for line in [
            r#"{"type":"start_session","mode":"locked","duration":60,"allowlist":[]}"#,
            r#"{"type":"set_allowlist","allowlist":["not-a-playlist"]}"#,
            r#"{"type":"unknown"}"#,
        ] {
            assert!(decode_line::<Request>(line).is_err(), "{line}");
        }
    }

    #[test]
    fn errors_map_to_codes() {
        let response = Response::from(tunelock_core::Error::Locked {
            remaining: Duration::from_hours(1),
        });
        assert_eq!(
            serde_json::to_string(&response).unwrap(),
            r#"{"type":"error","code":"locked","message":"the session is locked; 1h remaining"}"#
        );
    }

    #[test]
    fn extension_state_from_lock_state() {
        assert_eq!(
            ExtensionState::from(&LockState::Idle),
            ExtensionState {
                protocol_version: PROTOCOL_VERSION,
                active: false,
                remaining: Duration::ZERO,
                allowed_uris: vec![],
            }
        );

        let mut state = LockState::Idle;
        state
            .start(Session::new(LockMode::Locked, Duration::from_hours(1), allowlist()).unwrap())
            .unwrap();
        let extension = ExtensionState::from(&state);
        assert!(extension.active);
        assert_eq!(extension.remaining, Duration::from_hours(1));
        assert_eq!(
            extension.allowed_uris,
            vec![format!("spotify:playlist:{ID}")]
        );
    }
}
