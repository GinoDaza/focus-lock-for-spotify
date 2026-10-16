//! The Tunelock service: owns the session state, advances time, serves the
//! control channel and the extension channel.

pub mod app;
pub mod control;
pub mod engine;
pub mod extension;
pub mod store;

use std::path::Path;

use chrono::Utc;

use crate::app::App;
use crate::engine::Engine;
use crate::store::JsonFileStore;

/// Runs the service until Ctrl-C (console mode).
///
/// # Errors
/// If the data directory or either channel cannot be opened.
pub async fn run(data_dir: &Path, socket: &str, port: u16) -> anyhow::Result<()> {
    let store = JsonFileStore::open(data_dir)?;
    let app = App::new(Engine::load(Box::new(store), Utc::now()));

    let control = control::bind(socket)?;
    let extension = extension::bind(port).await?;
    tracing::info!(
        data_dir = %data_dir.display(),
        socket,
        extension = %extension.local_addr()?,
        "tunelock service running"
    );

    tokio::select! {
        () = app::run_ticker(app.clone()) => {}
        () = control::serve(control, app.clone()) => {}
        result = extension::serve(extension, app) => result?,
        result = tokio::signal::ctrl_c() => {
            result?;
            tracing::info!("shutting down");
        }
    }
    Ok(())
}
