//! Extension channel: a tiny read-only HTTP API on the loopback interface
//! for the Spotify extension. Nothing here can change a session; the only
//! write is the timestamp of the last heartbeat.

use std::io;
use std::net::{Ipv4Addr, SocketAddr};

use axum::extract::State;
use axum::http::{Method, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use focuslock_protocol::ExtensionState;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use crate::app::App;

/// Binds the extension API on 127.0.0.1. Port 0 picks a free port.
///
/// # Errors
/// If the port is in use.
pub async fn bind(port: u16) -> io::Result<TcpListener> {
    TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port))).await
}

pub fn router(app: App) -> Router {
    // The Spotify client is a web page with its own origin, so CORS is
    // required. No route can change a session, so any origin may call them.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST]);
    Router::new()
        .route("/v1/state", get(state))
        .route("/v1/heartbeat", post(heartbeat))
        .layer(cors)
        .with_state(app)
}

/// Serves until the process exits.
///
/// # Errors
/// If the server fails.
pub async fn serve(listener: TcpListener, app: App) -> io::Result<()> {
    axum::serve(listener, router(app)).await
}

async fn state(State(app): State<App>) -> Json<ExtensionState> {
    Json(app.engine().extension_state())
}

async fn heartbeat(State(app): State<App>) -> StatusCode {
    tracing::debug!("extension heartbeat");
    app.record_heartbeat();
    StatusCode::NO_CONTENT
}
