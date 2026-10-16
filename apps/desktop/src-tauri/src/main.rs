//! Tunelock desktop app.
//!
//! A thin frontend over the service's control channel: it keeps no state of
//! its own, so closing it never affects a session. Every rule, including
//! "a locked session cannot be weakened", is enforced by the service.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tunelock_client::Client;
use tunelock_protocol::{CONTROL_SOCKET, Request, Response};

/// Forwards one request to the service. Errors reach the frontend as a
/// rejected promise carrying a readable message.
#[tauri::command]
async fn request(request: Request) -> Result<Response, String> {
    tauri::async_runtime::spawn_blocking(move || send(&request))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| format!("{error:#}"))
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

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![request])
        .run(tauri::generate_context!())
        .expect("error while running the Tunelock app");
}
