//! First-run setup of the Spotify side of Focus Lock, run by the desktop app
//! as the logged-on user (Spicetify refuses to run elevated):
//!
//! 1. Spotify from spotify.com must be installed (the Microsoft Store build
//!    can't be patched).
//! 2. Spicetify is installed from its latest GitHub release if missing,
//!    into `%LOCALAPPDATA%\spicetify` like its official installer does.
//! 3. The bundled extension is written to `%APPDATA%\spicetify\Extensions`,
//!    which Spicetify searches first and its own updates never touch, then
//!    enabled and applied.
//!
//! After that the service keeps the setup in place (ADR 0005).

mod spicetify;

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, bail};
use serde::Serialize;

pub use spicetify::{install_latest, select_asset, verify_digest};

/// File name of the extension in Spicetify's folders and config.
pub const EXTENSION_NAME: &str = "focuslock.js";

/// Package folder of the Microsoft Store build of Spotify.
const STORE_PACKAGE: &str = "SpotifyAB.SpotifyMusic_zpdnekdrzrea0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpotifyState {
    Missing,
    /// Only the Microsoft Store build is installed.
    Store,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionState {
    Missing,
    /// Applied, but older than the one bundled with the app.
    Outdated,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SetupStatus {
    pub spotify: SpotifyState,
    pub spicetify: bool,
    pub extension: ExtensionState,
}

impl SetupStatus {
    #[must_use]
    pub fn ready(&self) -> bool {
        self.spotify == SpotifyState::Ready
            && self.spicetify
            && self.extension == ExtensionState::Ready
    }
}

/// The user's folders involved in the setup.
#[derive(Debug, Clone)]
pub struct Paths {
    /// `%APPDATA%`
    pub roaming: PathBuf,
    /// `%LOCALAPPDATA%`
    pub local: PathBuf,
}

impl Paths {
    /// # Errors
    /// If `APPDATA` or `LOCALAPPDATA` is not set.
    pub fn from_env() -> anyhow::Result<Self> {
        let var = |name| {
            std::env::var_os(name)
                .map(PathBuf::from)
                .with_context(|| format!("{name} is not set"))
        };
        Ok(Self {
            roaming: var("APPDATA")?,
            local: var("LOCALAPPDATA")?,
        })
    }

    #[must_use]
    pub fn spicetify_dir(&self) -> PathBuf {
        self.local.join("spicetify")
    }

    #[must_use]
    pub fn spicetify_exe(&self) -> PathBuf {
        self.spicetify_dir().join("spicetify.exe")
    }

    fn config_file(&self) -> PathBuf {
        self.roaming.join("spicetify").join("config-xpui.ini")
    }

    fn user_extension(&self) -> PathBuf {
        self.roaming
            .join("spicetify")
            .join("Extensions")
            .join(EXTENSION_NAME)
    }

    /// Where development builds used to copy the extension. It is shadowed
    /// by [`Paths::user_extension`] and removed to avoid confusion.
    fn legacy_extension(&self) -> PathBuf {
        self.spicetify_dir().join("Extensions").join(EXTENSION_NAME)
    }

    fn config(&self) -> Option<String> {
        fs::read_to_string(self.config_file()).ok()
    }

    /// Spotify's folder, following `spotify_path` in Spicetify's config.
    #[must_use]
    pub fn spotify_dir(&self) -> PathBuf {
        self.config()
            .as_deref()
            .and_then(|config| ini_value(config, "spotify_path"))
            .filter(|path| !path.is_empty())
            .map_or_else(|| self.roaming.join("Spotify"), PathBuf::from)
    }
}

/// Reads `key = value` from an INI file, ignoring sections.
fn ini_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|line| {
        let (k, v) = line.split_once('=')?;
        (k.trim() == key).then(|| v.trim())
    })
}

fn config_enables_extension(config: &str) -> bool {
    ini_value(config, "extensions")
        .is_some_and(|list| list.split('|').any(|name| name.trim() == EXTENSION_NAME))
}

/// Inspects the user's folders. Never fails: anything unreadable counts as
/// missing.
#[must_use]
pub fn status(paths: &Paths, bundled: &[u8]) -> SetupStatus {
    let spotify_dir = paths.spotify_dir();
    let spotify = if spotify_dir.join("Spotify.exe").is_file() {
        SpotifyState::Ready
    } else if paths.local.join("Packages").join(STORE_PACKAGE).is_dir() {
        SpotifyState::Store
    } else {
        SpotifyState::Missing
    };

    let apps = spotify_dir.join("Apps");
    let applied = fs::read(apps.join("xpui").join("extensions").join(EXTENSION_NAME)).ok();
    // A stock `xpui.spa` means Spotify updated and Spicetify is not applied.
    let extension = match applied {
        Some(bytes) if !apps.join("xpui.spa").exists() => {
            if bytes == bundled {
                ExtensionState::Ready
            } else {
                ExtensionState::Outdated
            }
        }
        _ => ExtensionState::Missing,
    };

    SetupStatus {
        spotify,
        spicetify: paths.spicetify_exe().is_file(),
        extension,
    }
}

/// Brings everything to [`SetupStatus::ready`], reporting each step through
/// `progress`. Spotify restarts when Spicetify applies.
///
/// # Errors
/// If Spotify is missing, a download or Spicetify command fails, or the
/// extension is still not applied at the end.
pub fn run(paths: &Paths, bundled: &[u8], progress: impl Fn(&str)) -> anyhow::Result<()> {
    let before = status(paths, bundled);
    match before.spotify {
        SpotifyState::Ready => {}
        SpotifyState::Store => bail!(
            "the Microsoft Store version of Spotify can't be used; install Spotify from spotify.com"
        ),
        SpotifyState::Missing => bail!("Spotify isn't installed; install it from spotify.com"),
    }

    if !before.spicetify {
        progress("Downloading Spicetify…");
        install_latest(&paths.spicetify_dir())?;
    }

    progress("Installing the Focus Lock extension…");
    let target = paths.user_extension();
    if let Some(dir) = target.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&target, bundled).with_context(|| format!("writing {}", target.display()))?;
    let _ = fs::remove_file(paths.legacy_extension());

    if !paths.config().is_some_and(|c| config_enables_extension(&c)) {
        spicetify::run(paths, &["config", "extensions", EXTENSION_NAME])?;
    }

    progress("Applying to Spotify…");
    let apps = paths.spotify_dir().join("Apps");
    if apps.join("xpui.spa").exists() || !apps.join("xpui").is_dir() {
        spicetify::run(paths, &["-q", "backup", "apply"])?;
    } else {
        spicetify::run(paths, &["-q", "apply"])?;
    }

    if status(paths, bundled).extension != ExtensionState::Ready {
        bail!("Spicetify finished, but the extension isn't in Spotify yet; try again");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    const BUNDLED: &[u8] = b"// focuslock v2";

    fn paths(dir: &Path) -> Paths {
        Paths {
            roaming: dir.join("Roaming"),
            local: dir.join("Local"),
        }
    }

    fn touch(path: &Path, contents: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn ini_values() {
        let config = "[Setting]\nspotify_path = C:\\Spotify\n[AdditionalOptions]\nextensions = a.js|focuslock.js\n";
        assert_eq!(ini_value(config, "spotify_path"), Some("C:\\Spotify"));
        assert!(config_enables_extension(config));
        assert!(!config_enables_extension(
            "extensions = a.js|focuslock.js.bak"
        ));
        assert!(!config_enables_extension("extensions ="));
    }

    #[test]
    fn nothing_installed() {
        let dir = tempfile::tempdir().unwrap();
        let status = status(&paths(dir.path()), BUNDLED);
        assert_eq!(
            status,
            SetupStatus {
                spotify: SpotifyState::Missing,
                spicetify: false,
                extension: ExtensionState::Missing,
            }
        );
        assert!(!status.ready());
    }

    #[test]
    fn store_build_is_detected() {
        let dir = tempfile::tempdir().unwrap();
        let p = paths(dir.path());
        fs::create_dir_all(p.local.join("Packages").join(STORE_PACKAGE)).unwrap();
        assert_eq!(status(&p, BUNDLED).spotify, SpotifyState::Store);
    }

    #[test]
    fn extension_state_follows_the_applied_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = paths(dir.path());
        let spotify = p.roaming.join("Spotify");
        touch(&spotify.join("Spotify.exe"), b"");
        touch(&p.spicetify_exe(), b"");
        let applied = spotify.join("Apps/xpui/extensions").join(EXTENSION_NAME);

        touch(&applied, b"// focuslock v1");
        assert_eq!(status(&p, BUNDLED).extension, ExtensionState::Outdated);

        touch(&applied, BUNDLED);
        let ready = status(&p, BUNDLED);
        assert_eq!(ready.extension, ExtensionState::Ready);
        assert!(ready.ready());

        // A Spotify update brings back the stock bundle.
        touch(&spotify.join("Apps/xpui.spa"), b"");
        assert_eq!(status(&p, BUNDLED).extension, ExtensionState::Missing);
    }

    #[test]
    fn spotify_path_from_config() {
        let dir = tempfile::tempdir().unwrap();
        let p = paths(dir.path());
        let custom = dir.path().join("Custom Spotify");
        touch(&custom.join("Spotify.exe"), b"");
        touch(
            &p.config_file(),
            format!("[Setting]\nspotify_path = {}\n", custom.display()).as_bytes(),
        );
        assert_eq!(p.spotify_dir(), custom);
        assert_eq!(status(&p, BUNDLED).spotify, SpotifyState::Ready);
    }

    #[test]
    fn run_refuses_without_spotify() {
        let dir = tempfile::tempdir().unwrap();
        let error = run(&paths(dir.path()), BUNDLED, |_| {}).unwrap_err();
        assert!(error.to_string().contains("isn't installed"), "{error}");
    }
}
