//! Tunelock desktop app.
//!
//! A thin frontend over the service's control channel: it keeps no state of
//! its own, so closing it never affects a session. Every rule, including
//! "a locked session cannot be weakened", is enforced by the service.
//!
//! It also runs the first-run setup of the Spotify side (`tunelock-setup`),
//! which has to happen as the user rather than in the elevated installer.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::bail;
use tauri::Manager;
use tauri::ipc::Channel;
use tauri_plugin_window_state::StateFlags;
use tunelock_client::Client;
use tunelock_core::LockMode;
use tunelock_protocol::{CONTROL_SOCKET, Request, Response};
use tunelock_setup::{Paths, SetupStatus};

/// The Spicetify extension shipped with this version of the app.
const EXTENSION: &[u8] = include_bytes!("../../../../extension/dist/tunelock.js");

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
    blocking(|| Ok(tunelock_setup::status(&Paths::from_env()?, EXTENSION))).await
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
        tunelock_setup::run(&paths, EXTENSION, |step| {
            let _ = on_progress.send(step.to_owned());
        })?;
        Ok(tunelock_setup::status(&paths, EXTENSION))
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
        // Opens centered the first time, then where the user left it. The
        // window starts hidden and is shown once it's in place, so it doesn't
        // jump from the center to its saved position.
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() - StateFlags::VISIBLE)
                .build(),
        )
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
                window.set_focus()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            request,
            setup_status,
            run_setup,
            open_spotify_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Tunelock app");
}
