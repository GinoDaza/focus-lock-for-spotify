//! The Focus Lock service: owns the session state, advances time, serves the
//! control channel and the extension channel, and runs the enforcers.

pub mod app;
pub mod control;
pub mod enforcer;
pub mod engine;
pub mod extension;
pub mod sddl;
pub mod spicetify;
pub mod store;
#[cfg(windows)]
pub mod windows;

use std::future::Future;
use std::path::{Path, PathBuf};

use anyhow::Context;
use chrono::Utc;
use interprocess::local_socket::tokio::Listener;
use tokio::net::TcpListener;

use crate::app::App;
use crate::enforcer::{SpotifyEnforcer, SystemProcesses};
use crate::engine::Engine;
use crate::store::JsonFileStore;

pub struct Options {
    pub data_dir: PathBuf,
    pub socket: String,
    pub port: u16,
    /// Running as the Windows service: restrict the data directory to
    /// SYSTEM and Administrators and let interactive users reach the
    /// control channel.
    pub system: bool,
}

/// A service with its channels bound, ready to run.
pub struct Service {
    app: App,
    system: bool,
    control: Listener,
    extension: TcpListener,
}

impl Service {
    /// Loads the saved state and binds both channels.
    ///
    /// # Errors
    /// If the data directory or either channel cannot be opened.
    pub async fn bind(options: &Options) -> anyhow::Result<Self> {
        let store = JsonFileStore::open(&options.data_dir)?;
        if options.system {
            protect_data_dir(&options.data_dir)?;
        }
        let app = App::new(Engine::load(Box::new(store), Utc::now()));
        let control = control::bind(&options.socket, options.system)?;
        let extension = extension::bind(options.port).await.with_context(|| {
            format!("port {} is already in use by another program", options.port)
        })?;
        tracing::info!(
            data_dir = %options.data_dir.display(),
            socket = options.socket,
            extension = %extension.local_addr()?,
            "focuslock service running"
        );
        Ok(Self {
            app,
            system: options.system,
            control,
            extension,
        })
    }

    #[must_use]
    pub fn app(&self) -> App {
        self.app.clone()
    }

    /// Runs until `shutdown` completes. The state is saved on every change
    /// and tick, so stopping needs no extra work.
    ///
    /// # Errors
    /// If the extension server fails.
    pub async fn run(self, shutdown: impl Future<Output = ()>) -> anyhow::Result<()> {
        let Self {
            app,
            system,
            control,
            extension,
        } = self;
        let enforcers = enforcers(system);
        tokio::select! {
            () = app::run_ticker(app.clone()) => {}
            () = enforcer::run(app.clone(), enforcers) => {}
            () = control::serve(control, app.clone()) => {}
            result = extension::serve(extension, app) => result?,
            () = shutdown => tracing::info!("shutting down"),
        }
        Ok(())
    }
}

fn enforcers(system: bool) -> Vec<Box<dyn enforcer::Enforcer>> {
    let mut enforcers: Vec<Box<dyn enforcer::Enforcer>> =
        vec![Box::new(SpotifyEnforcer::new(SystemProcesses::new()))];
    // Protecting the user's files and applying Spicetify as the user need
    // the service's LocalSystem rights.
    #[cfg(windows)]
    if system {
        enforcers.push(Box::new(spicetify::SpicetifyEnforcer::new(
            windows::desktop::WindowsDesktop,
        )));
    }
    #[cfg(not(windows))]
    let _ = (system, &mut enforcers);
    enforcers
}

/// Runs the service in the console until Ctrl-C.
///
/// # Errors
/// If the service cannot start.
pub async fn run_console(options: &Options) -> anyhow::Result<()> {
    let service = Service::bind(options).await?;
    service
        .run(async {
            if let Err(error) = tokio::signal::ctrl_c().await {
                tracing::error!("cannot listen for Ctrl-C: {error}");
                std::future::pending::<()>().await;
            }
        })
        .await
}

#[cfg(windows)]
fn protect_data_dir(dir: &Path) -> anyhow::Result<()> {
    windows::acl::set_protected_dacl(dir, windows::acl::SYSTEM_AND_ADMINS_ONLY)
        .with_context(|| format!("restricting access to {}", dir.display()))
}

#[cfg(not(windows))]
#[allow(clippy::unnecessary_wraps)]
fn protect_data_dir(_dir: &Path) -> anyhow::Result<()> {
    Ok(())
}
