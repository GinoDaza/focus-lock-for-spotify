# ADR 0006: The desktop app is a thin, stateless frontend

- Status: accepted
- Date: 2026-10-16

## Context

The plan (ADR 0001) called for a Tauri v2 app with a tray icon and a countdown. Before building it we checked what it costs. The service uses about 13 MB of RAM and Spotify about 650 MB. Measured on Windows, the app window uses about 145 MB of private memory across its WebView2 processes while it is open. A tray icon would keep the app resident all the time.

Sessions live in the service. The app only reads the state and sends requests, so it does not need to run for a lock to hold.

## Decision

- **Tauri v2 + React + TypeScript + Vite** in `apps/desktop`. The Rust side exposes a single command, `request`, that forwards one `focuslock_protocol::Request` to the control socket and returns the `Response`. Service errors become rejected promises. `src/protocol.ts` mirrors the Rust types by hand.
- **No tray icon in v0.1.** The app is a normal window. Closing it exits the process and leaves the session running; the footer says so. Nothing is resident besides the service.
- **No state in the app.** It reads `status` when it opens, every 5 s while open, and when the window becomes visible. The countdown ticks locally between reads.
- **The service stays the only authority.** The UI never decides what is allowed. It still adds friction where the CLI does: locking a session (starting one, starting a locked preset, or hardening) asks the user to type `lock`, and a locked session only offers adding time.
- **Windows 11 look.** The CSS follows the WinUI theme resources: Segoe UI Variable and its type ramp, Segoe Fluent Icons, 4px/8px corners, Settings-style cards, a NavigationView-style pane, InfoBars and a ContentDialog. It uses the default Windows accent because WebView2 can't read the user's.
- **Playlist names and covers** come from Spotify's public oEmbed endpoint (`open.spotify.com/oembed`, no account or token). They are cached in `localStorage`. On failure the app shows the ID. The CSP allows only that host and Spotify's image CDN.
- The blocking control client moved from the CLI into `crates/client` (`focuslock-client`), shared by the CLI and the app.
- The app's crate is a workspace member but not a default member, so `cargo test` and `cargo clippy` on Linux do not need a WebView toolchain. Windows builds name it with `-p focuslock-desktop`.

## Consequences

- No memory cost when the window is closed. While it is open, the app costs about what a browser tab does.
- Users don't get a constant "Focus Lock is running" indicator. A tray icon can come later as part of the frontend without touching the service.
- Opening the app sends the allowlisted playlist IDs to Spotify's oEmbed endpoint. Spotify already knows what the user plays, so this reveals nothing new.
- Cross-compiling the app from WSL needs `llvm-rc` (from the `llvm` package) for the Windows resources.
