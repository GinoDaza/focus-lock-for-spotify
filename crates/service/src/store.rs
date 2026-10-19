//! Persistence of everything the service must remember across restarts.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use chrono::{DateTime, NaiveDateTime, Utc};
use focuslock_core::{Allowlist, LockState, Preset, Schedule};
use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Data {
    pub schema: u32,
    pub state: LockState,
    pub allowlist: Option<Allowlist>,
    pub presets: Vec<Preset>,
    pub schedules: Vec<Schedule>,
    /// Last moment the service was known to be running. Used to credit
    /// time spent shut down or asleep.
    pub checkpoint: Option<DateTime<Utc>>,
    /// Set when a scheduled session is cancelled, so the same window does
    /// not restart it right away.
    pub schedules_paused_until: Option<NaiveDateTime>,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            schema: SCHEMA_VERSION,
            state: LockState::Idle,
            allowlist: None,
            presets: Vec::new(),
            schedules: Vec::new(),
            checkpoint: None,
            schedules_paused_until: None,
        }
    }
}

pub trait Store: Send {
    /// Returns `None` when nothing has been saved yet.
    ///
    /// # Errors
    /// If existing data cannot be read or parsed.
    fn load(&self) -> anyhow::Result<Option<Data>>;

    /// # Errors
    /// If the data cannot be written.
    fn save(&mut self, data: &Data) -> anyhow::Result<()>;
}

/// Stores [`Data`] as one JSON file, replaced atomically on every save.
pub struct JsonFileStore {
    path: PathBuf,
}

impl JsonFileStore {
    /// # Errors
    /// If the directory cannot be created.
    pub fn open(dir: &Path) -> anyhow::Result<Self> {
        fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        Ok(Self {
            path: dir.join("state.json"),
        })
    }
}

impl Store for JsonFileStore {
    fn load(&self) -> anyhow::Result<Option<Data>> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e).with_context(|| format!("reading {}", self.path.display())),
        };
        let data = serde_json::from_str(&text)
            .with_context(|| format!("parsing {}", self.path.display()))?;
        Ok(Some(data))
    }

    fn save(&mut self, data: &Data) -> anyhow::Result<()> {
        let tmp = self.path.with_extension("json.tmp");
        let mut file =
            fs::File::create(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
        file.write_all(&serde_json::to_vec_pretty(data)?)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, &self.path)
            .with_context(|| format!("replacing {}", self.path.display()))?;
        Ok(())
    }
}

/// Keeps data in memory only. For tests.
#[derive(Default)]
pub struct MemoryStore(pub Option<Data>);

impl Store for MemoryStore {
    fn load(&self) -> anyhow::Result<Option<Data>> {
        Ok(self.0.clone())
    }

    fn save(&mut self, data: &Data) -> anyhow::Result<()> {
        self.0 = Some(data.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_store_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = JsonFileStore::open(dir.path()).unwrap();
        assert_eq!(store.load().unwrap(), None);

        let data = Data {
            checkpoint: Some(Utc::now()),
            ..Data::default()
        };
        store.save(&data).unwrap();
        assert_eq!(store.load().unwrap(), Some(data));
        assert!(!dir.path().join("state.json.tmp").exists());
    }

    #[test]
    fn corrupt_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("state.json"), "{ nope").unwrap();
        assert!(JsonFileStore::open(dir.path()).unwrap().load().is_err());
    }
}
