//! Serializes a [`Duration`] as whole seconds, the format every Focus Lock
//! JSON consumer (UI, extension, database) expects. Use with
//! `#[serde(with = "focuslock_core::serde_secs")]`.

use std::time::Duration;

use serde::{Deserialize, Deserializer, Serializer};

/// # Errors
/// Propagates serializer errors.
#[allow(clippy::trivially_copy_pass_by_ref)] // signature required by serde
pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_u64(duration.as_secs())
}

/// # Errors
/// Fails if the value is not an unsigned integer.
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
    u64::deserialize(deserializer).map(Duration::from_secs)
}
