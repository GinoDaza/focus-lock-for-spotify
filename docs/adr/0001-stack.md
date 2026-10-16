# ADR 0001: Technology stack

- Status: accepted
- Date: 2026-10-16

## Context

Tunelock needs these parts:
- A lock that survives closing the UI, reboots and system clock changes.
- A way to restrict the Spotify desktop client to an allowlist of playlists.
- A small desktop UI.

The first target is Windows only. The project is open source (GPL-3.0) and maintained by a single person, so low maintenance matters more than breadth.

## Decision

- **Core and service: Rust.** The service uses the `windows-service` crate and `tokio`. It produces a native binary with no runtime dependency.
- **UI ↔ service IPC:** a named pipe (`interprocess` crate) with a restrictive ACL. Messages are serde/JSON types defined in a shared `protocol` crate.
- **Persistence:** SQLite (`rusqlite`) in `%ProgramData%\Tunelock`.
- **Desktop UI:** Tauri v2 with React, TypeScript and Vite, plus a tray icon.
- **Spotify integration:** a Spicetify extension written in TypeScript and bundled with esbuild into a single `.js` file. It renders its own focus overlay instead of hiding many Spotify selectors. All Spotify-specific selectors and routes live in one compat module.
- **Installer:** the Tauri NSIS bundler in per-machine mode, with installer hooks that register the service.
- **CI:** GitHub Actions on `windows-latest`. Code signing will be requested from SignPath Foundation after the first release.

## Consequences

- One language (Rust) covers the service, the CLI and the Tauri backend, so the protocol types are shared without code generation.
- Development happens in WSL, but the service and the UI are built and tested on Windows, either with `cargo-xwin` or natively, and in CI.
- Spotify updates can break the extension. Keeping Spotify-specific code in one module limits the fix to one place.
