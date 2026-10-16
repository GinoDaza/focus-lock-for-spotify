//! [`Desktop`] for Windows: the console user's folders, folder protection
//! through ACLs and running Spicetify in the user's session.

use std::io;
use std::path::Path;
use std::time::Duration;

use crate::sddl;
use crate::spicetify::{Desktop, PatchState, UserPaths};
use crate::windows::{acl, user};

/// Applying Spicetify extracts and patches Spotify's UI; it takes a few
/// seconds, so anything past this is stuck.
const APPLY_TIMEOUT: Duration = Duration::from_mins(2);

pub struct WindowsDesktop;

impl Desktop for WindowsDesktop {
    fn user_paths(&mut self) -> Option<UserPaths> {
        let token = user::console_user()?;
        let folders = user::roaming_app_data(&token).and_then(|roaming| {
            user::local_app_data(&token).map(|local| UserPaths::new(&roaming, &local))
        });
        folders
            .inspect_err(|error| tracing::warn!("cannot find the user's folders: {error}"))
            .ok()
    }

    fn patch_state(&self, paths: &UserPaths) -> PatchState {
        paths.patch_state()
    }

    fn set_protected(&mut self, path: &Path, protected: bool) -> io::Result<()> {
        if !path.exists() {
            return Ok(());
        }
        let current = acl::read_dacl(path)?;
        let wanted = if protected {
            sddl::protect(&current)
        } else {
            sddl::unprotect(&current)
        };
        if wanted == current {
            return Ok(());
        }
        acl::write_dacl(path, &wanted)
    }

    fn apply_spicetify(&mut self, paths: &UserPaths) -> io::Result<()> {
        let token = user::console_user()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "nobody is logged on"))?;
        let exe = paths.spicetify_exe();
        if !exe.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("{} not found", exe.display()),
            ));
        }
        // `backup` takes a fresh backup of the updated Spotify, `apply`
        // patches it. Quiet, and without restarting Spotify: if it runs
        // unpatched, the watchdog closes it.
        let code = user::run_as(
            &token,
            &exe,
            &["-q", "-n", "backup", "apply"],
            APPLY_TIMEOUT,
        )?;
        if code == 0 {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "spicetify exited with code {code}"
            )))
        }
    }
}
