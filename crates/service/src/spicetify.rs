//! Keeps the Spicetify setup that carries the Focus Lock extension in place.
//!
//! - During a session, if Spotify was updated (which removes Spicetify),
//!   Spicetify is applied again in the logged-on user's session.
//! - During a locked session, the folders that make up the setup are
//!   protected so the user cannot change them.
//!
//! The decisions live here; the operating system work sits behind
//! [`Desktop`] so it can be tested anywhere.

use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::enforcer::{Context, Enforcer};

/// Name of the extension file inside Spotify's patched UI.
pub const EXTENSION_FILE: &str = "focuslock.js";

/// Minimum time between two attempts to re-apply Spicetify.
pub const REAPPLY_BACKOFF: Duration = Duration::from_mins(5);

/// Where the logged-on user's Spotify and Spicetify live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPaths {
    /// `%APPDATA%\Spotify`, or the `spotify_path` set in Spicetify's config.
    pub spotify: PathBuf,
    /// `%LOCALAPPDATA%\spicetify`: the CLI and bundled extensions.
    pub spicetify_install: PathBuf,
    /// `%APPDATA%\spicetify`: config, backups and user extensions.
    pub spicetify_config: PathBuf,
}

impl UserPaths {
    /// Builds the paths from the user's roaming and local app data folders,
    /// honouring a custom Spotify location in Spicetify's config.
    #[must_use]
    pub fn new(roaming: &Path, local: &Path) -> Self {
        let spicetify_config = roaming.join("spicetify");
        let spotify = std::fs::read_to_string(spicetify_config.join("config-xpui.ini"))
            .ok()
            .and_then(|config| config_value(&config, "spotify_path").map(PathBuf::from))
            .unwrap_or_else(|| roaming.join("Spotify"));
        Self {
            spotify,
            spicetify_install: local.join("spicetify"),
            spicetify_config,
        }
    }

    #[must_use]
    pub fn spicetify_exe(&self) -> PathBuf {
        self.spicetify_install.join("spicetify.exe")
    }

    /// The folders protected during a locked session: Spotify's UI (which
    /// holds the applied extension) and both Spicetify folders.
    #[must_use]
    pub fn protected(&self) -> [PathBuf; 3] {
        [
            self.spotify.join("Apps"),
            self.spicetify_install.clone(),
            self.spicetify_config.clone(),
        ]
    }

    /// Reads the state of Spotify's UI from disk.
    #[must_use]
    pub fn patch_state(&self) -> PatchState {
        let apps = self.spotify.join("Apps");
        if !apps.is_dir() {
            PatchState::NotInstalled
        } else if !apps.join("xpui.spa").exists()
            && apps.join("xpui/extensions").join(EXTENSION_FILE).is_file()
        {
            PatchState::Patched
        } else {
            PatchState::Unpatched
        }
    }
}

/// Reads `key = value` from Spicetify's ini config. Empty values count as
/// missing.
fn config_value<'a>(config: &'a str, key: &str) -> Option<&'a str> {
    config.lines().find_map(|line| {
        let (k, v) = line.split_once('=')?;
        let v = v.trim();
        (k.trim() == key && !v.is_empty()).then_some(v)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchState {
    /// Spotify is not installed where expected.
    NotInstalled,
    /// Spicetify is applied with the Focus Lock extension.
    Patched,
    /// Stock Spotify UI, e.g. right after an update.
    Unpatched,
}

/// Operating system access needed by [`SpicetifyEnforcer`].
pub trait Desktop: Send {
    /// The logged-on user's paths, or `None` when nobody is logged on.
    fn user_paths(&mut self) -> Option<UserPaths>;
    fn patch_state(&self, paths: &UserPaths) -> PatchState;
    /// Whether Spotify's own updater is running right now.
    fn updating(&mut self, paths: &UserPaths) -> bool;
    /// Adds or removes the protection on one folder. A missing folder is
    /// not an error.
    ///
    /// # Errors
    /// If the folder's permissions cannot be read or written.
    fn set_protected(&mut self, path: &Path, protected: bool) -> io::Result<()>;
    /// Runs `spicetify backup apply` as the logged-on user and waits for it.
    ///
    /// # Errors
    /// If it cannot be started or does not succeed.
    fn apply_spicetify(&mut self, paths: &UserPaths) -> io::Result<()>;
}

pub struct SpicetifyEnforcer<D> {
    desktop: D,
    /// Whether the folders are protected; `None` until the first change,
    /// so a restarted service always brings them in line.
    protected: Option<bool>,
    last_apply: Option<Instant>,
}

impl<D: Desktop> SpicetifyEnforcer<D> {
    pub fn new(desktop: D) -> Self {
        Self {
            desktop,
            protected: None,
            last_apply: None,
        }
    }

    fn set_protected(&mut self, paths: &UserPaths, protected: bool) {
        for path in paths.protected() {
            if let Err(error) = self.desktop.set_protected(&path, protected) {
                tracing::warn!(path = %path.display(), protected, "cannot change protection: {error}");
            }
        }
        // Not retried on failure: one warning per change is enough, and
        // the next session tries again.
        self.protected = Some(protected);
        tracing::info!(protected, "Spicetify folders protection updated");
    }

    fn maybe_apply(&mut self, paths: &UserPaths, now: Instant) {
        if self.desktop.patch_state(paths) != PatchState::Unpatched {
            return;
        }
        if self
            .last_apply
            .is_some_and(|last| now.duration_since(last) < REAPPLY_BACKOFF)
        {
            return;
        }
        self.last_apply = Some(now);
        // Spicetify has to write into the protected folders.
        if self.protected != Some(false) {
            self.set_protected(paths, false);
        }
        tracing::info!("Spotify's UI is not patched; applying Spicetify");
        match self.desktop.apply_spicetify(paths) {
            Ok(()) => tracing::info!("Spicetify applied"),
            Err(error) => tracing::warn!("cannot apply Spicetify: {error}"),
        }
    }
}

impl<D: Desktop> Enforcer for SpicetifyEnforcer<D> {
    fn check(&mut self, context: &Context) {
        if !context.active && self.protected == Some(false) {
            return;
        }
        let Some(paths) = self.desktop.user_paths() else {
            return;
        };
        // Spotify's updater runs as the user and replaces the UI folder, so
        // it gets a free hand until it exits. The check after that
        // re-applies Spicetify and restores the protection.
        if context.active && self.desktop.updating(&paths) {
            if self.protected != Some(false) {
                tracing::info!("Spotify is updating; lifting the protection until it finishes");
                self.set_protected(&paths, false);
            }
            return;
        }
        if context.active {
            self.maybe_apply(&paths, context.now);
        }
        if self.protected != Some(context.locked) {
            self.set_protected(&paths, context.locked);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeDesktop {
        unpatched: bool,
        updating: bool,
        protected: bool,
        log: Vec<String>,
    }

    impl Desktop for FakeDesktop {
        fn user_paths(&mut self) -> Option<UserPaths> {
            Some(UserPaths {
                spotify: "S".into(),
                spicetify_install: "I".into(),
                spicetify_config: "C".into(),
            })
        }

        fn patch_state(&self, _: &UserPaths) -> PatchState {
            if self.unpatched {
                PatchState::Unpatched
            } else {
                PatchState::Patched
            }
        }

        fn updating(&mut self, _: &UserPaths) -> bool {
            self.updating
        }

        fn set_protected(&mut self, path: &Path, protected: bool) -> io::Result<()> {
            self.protected = protected;
            self.log.push(format!("{}={protected}", path.display()));
            Ok(())
        }

        fn apply_spicetify(&mut self, _: &UserPaths) -> io::Result<()> {
            assert!(!self.protected, "applied while protected");
            self.unpatched = false;
            self.log.push("apply".into());
            Ok(())
        }
    }

    fn context(active: bool, locked: bool, now: Instant) -> Context {
        Context {
            active,
            locked,
            last_heartbeat: None,
            now,
        }
    }

    fn enforcer() -> SpicetifyEnforcer<FakeDesktop> {
        SpicetifyEnforcer::new(FakeDesktop::default())
    }

    #[test]
    fn startup_unprotects_once_then_idles() {
        let now = Instant::now();
        let mut enforcer = enforcer();
        enforcer.check(&context(false, false, now));
        enforcer.check(&context(false, false, now));
        assert_eq!(
            enforcer.desktop.log,
            ["S/Apps=false", "I=false", "C=false"]
                .map(|s| s.replace('/', std::path::MAIN_SEPARATOR_STR))
        );
    }

    #[test]
    fn locked_sessions_protect_and_unprotect_at_the_end() {
        let now = Instant::now();
        let mut enforcer = enforcer();
        enforcer.check(&context(true, true, now));
        assert!(enforcer.desktop.protected);
        enforcer.check(&context(true, true, now));
        assert_eq!(enforcer.desktop.log.len(), 3);
        enforcer.check(&context(false, false, now));
        assert!(!enforcer.desktop.protected);
    }

    #[test]
    fn flexible_sessions_do_not_protect() {
        let now = Instant::now();
        let mut enforcer = enforcer();
        enforcer.check(&context(true, false, now));
        assert!(!enforcer.desktop.protected);
    }

    #[test]
    fn reapplies_after_an_update_and_protects_again() {
        let now = Instant::now();
        let mut enforcer = enforcer();
        enforcer.check(&context(true, true, now));
        enforcer.desktop.unpatched = true;
        enforcer.desktop.log.clear();
        enforcer.check(&context(true, true, now));
        assert_eq!(enforcer.desktop.log[3], "apply");
        assert!(enforcer.desktop.protected);
    }

    #[test]
    fn spotify_updates_run_unprotected_then_restore() {
        let now = Instant::now();
        let mut enforcer = enforcer();
        enforcer.check(&context(true, true, now));
        assert!(enforcer.desktop.protected);
        enforcer.desktop.updating = true;
        enforcer.check(&context(true, true, now));
        assert!(!enforcer.desktop.protected);
        enforcer.check(&context(true, true, now));
        enforcer.desktop.updating = false;
        enforcer.desktop.unpatched = true;
        enforcer.check(&context(true, true, now));
        assert!(enforcer.desktop.log.last().unwrap().ends_with("=true"));
        assert!(enforcer.desktop.protected);
        assert!(!enforcer.desktop.unpatched);
    }

    #[test]
    fn failed_applies_back_off() {
        struct Failing(FakeDesktop, usize);
        impl Desktop for Failing {
            fn user_paths(&mut self) -> Option<UserPaths> {
                self.0.user_paths()
            }
            fn patch_state(&self, _: &UserPaths) -> PatchState {
                PatchState::Unpatched
            }
            fn updating(&mut self, _: &UserPaths) -> bool {
                false
            }
            fn set_protected(&mut self, path: &Path, protected: bool) -> io::Result<()> {
                self.0.set_protected(path, protected)
            }
            fn apply_spicetify(&mut self, _: &UserPaths) -> io::Result<()> {
                self.1 += 1;
                Err(io::Error::other("nope"))
            }
        }
        let now = Instant::now();
        let mut enforcer = SpicetifyEnforcer::new(Failing(FakeDesktop::default(), 0));
        enforcer.check(&context(true, false, now));
        enforcer.check(&context(true, false, now + Duration::from_mins(1)));
        assert_eq!(enforcer.desktop.1, 1);
        enforcer.check(&context(true, false, now + Duration::from_mins(6)));
        assert_eq!(enforcer.desktop.1, 2);
    }

    #[test]
    fn reads_the_spotify_path_from_the_config() {
        let config = "[Setting]\nspotify_path           = D:\\Spotify\nprefs_path = \n";
        assert_eq!(config_value(config, "spotify_path"), Some("D:\\Spotify"));
        assert_eq!(config_value(config, "prefs_path"), None);
    }

    #[test]
    fn patch_state_reads_the_ui_folder() {
        let dir = tempfile::tempdir().unwrap();
        let paths = UserPaths::new(dir.path(), dir.path());
        assert_eq!(paths.patch_state(), PatchState::NotInstalled);

        let apps = paths.spotify.join("Apps");
        std::fs::create_dir_all(apps.join("xpui/extensions")).unwrap();
        std::fs::write(apps.join("xpui.spa"), "").unwrap();
        assert_eq!(paths.patch_state(), PatchState::Unpatched);

        std::fs::remove_file(apps.join("xpui.spa")).unwrap();
        std::fs::write(apps.join("xpui/extensions").join(EXTENSION_FILE), "").unwrap();
        assert_eq!(paths.patch_state(), PatchState::Patched);
    }
}
