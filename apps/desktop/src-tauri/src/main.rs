//! Focus Lock desktop app.
//!
//! A thin frontend over the service's control channel: it keeps no state of
//! its own, so closing it never affects a session. Every rule, including
//! "a locked session cannot be weakened", is enforced by the service.
//!
//! It also runs the first-run setup of the Spotify side (`focuslock-setup`),
//! which has to happen as the user rather than in the elevated installer.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::bail;
use focuslock_client::Client;
use focuslock_core::LockMode;
use focuslock_protocol::{CONTROL_SOCKET, Request, Response};
use focuslock_setup::{Paths, SetupStatus};
use tauri::Manager;
use tauri::ipc::Channel;

/// The Spicetify extension shipped with this version of the app.
const EXTENSION: &[u8] = include_bytes!("../../../../extension/dist/focuslock.js");

const SPOTIFY_DOWNLOAD: &str = "https://www.spotify.com/download/windows/";

/// Runs blocking work off the main thread. Errors reach the frontend as a
/// rejected promise carrying a readable message.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| format!("{error:#}"))
}

/// Forwards one request to the service.
#[tauri::command]
async fn request(request: Request) -> Result<Response, String> {
    blocking(move || send(&request)).await
}

/// One connection per request: the UI sends a handful per minute, and a
/// fresh connection recovers on its own after a service restart.
fn send(request: &Request) -> anyhow::Result<Response> {
    let mut client = Client::connect(CONTROL_SOCKET)?;
    match request {
        // `status` also checks that both sides speak the same protocol.
        Request::Status => client.status().map(Response::Status),
        _ => client.send(request),
    }
}

#[tauri::command]
async fn setup_status() -> Result<SetupStatus, String> {
    blocking(|| Ok(focuslock_setup::status(&Paths::from_env()?, EXTENSION))).await
}

#[tauri::command]
async fn run_setup(on_progress: Channel<String>) -> Result<SetupStatus, String> {
    blocking(move || {
        // The service write-protects these folders during a locked session
        // and keeps the setup in place by itself.
        let status = Client::connect(CONTROL_SOCKET)?.status()?;
        if status.session.is_some_and(|s| s.mode == LockMode::Locked) {
            bail!("setup is unavailable during a locked session");
        }
        let paths = Paths::from_env()?;
        focuslock_setup::run(&paths, EXTENSION, |step| {
            let _ = on_progress.send(step.to_owned());
        })?;
        Ok(focuslock_setup::status(&paths, EXTENSION))
    })
    .await
}

/// Opens Spotify's download page in the default browser. The URL is fixed:
/// the frontend can't open arbitrary links.
#[tauri::command]
fn open_spotify_download() -> Result<(), String> {
    #[cfg(windows)]
    let opener = "explorer";
    #[cfg(not(windows))]
    let opener = "xdg-open";
    std::process::Command::new(opener)
        .arg(SPOTIFY_DOWNLOAD)
        .spawn()
        .map(drop)
        .map_err(|error| error.to_string())
}

fn main() {
    tauri::Builder::default()
        // A second launch brings the open window to the front instead.
        // Registered first, as the plugin requires.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .invoke_handler(tauri::generate_handler![
            request,
            setup_status,
            run_setup,
            open_spotify_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Focus Lock app");
}
