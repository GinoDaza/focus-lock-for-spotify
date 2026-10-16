# AGENTS.md

Guide for AI agents and contributors working on Tunelock. Read this first, then `docs/plan.md` and `docs/adr/`.

## What this is

Tunelock is a Cold Turkey-style focus lock for the **Spotify desktop app on Windows**. During a session, Spotify can only show and play an allowlist of playlists. A **locked** session cannot be cancelled until it ends. The project is open source (GPL-3.0), and v0.1 targets Windows and Spotify only.

## Layout

| Path | What |
|---|---|
| `crates/core` | Pure domain model: `PlaylistId`/`Allowlist`, `Session`/`LockState` state machine, `Preset`, `Schedule`. No I/O. Time advances only via `LockState::advance`. |
| `crates/protocol` | Wire types. The control channel is line-delimited JSON over the local socket `tunelock` (named pipe on Windows). The extension channel is `ExtensionState` over loopback HTTP on port 39817. |
| `crates/service` | `tunelock-service`: `engine.rs` (requests and time policy), `store.rs` (atomic JSON file), `control.rs` (socket server), `extension.rs` (axum: `GET /v1/state`, `POST /v1/heartbeat`), `app.rs` (shared state and 5 s ticker). |
| `crates/cli` | `tunelock` CLI (status/start/cancel/extend/harden/allowlist/preset/schedule). |
| `extension/` | Spicetify extension in TypeScript, bundled by esbuild to `dist/tunelock.js`. **All Spotify-specific selectors and routes live in `src/compat.ts`.** |
| `spikes/` | Throwaway experiments (`tunelock-spike.js` proved extension → localhost works). |
| `docs/` | `plan.md` (approved plan), ADRs, `idea.md` (original note) and `research.md` (prior art). |

## Commands

```sh
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test   # Rust (clippy pedantic)
cd extension && pnpm typecheck && pnpm test && pnpm build             # extension
SPICETIFY_EXTENSIONS_DIR=<dir> pnpm build                               # build + copy into Spicetify
cargo run -p tunelock-service -- run --data-dir <dir>                   # service in foreground
cargo run -p tunelock-cli -- status                                     # CLI
```

The workspace lints forbid `unsafe` (`unsafe_code = "forbid"`). Windows FFI code will need a scoped exception in the service crate.

## Rules

- **Never document or explain ways to bypass or undo a lock.** That includes commands, workarounds and weaknesses spelled out step by step, in docs, comments, READMEs or chat. Describe protections, not how to defeat them. The user explicitly asked for this.
- A locked session must never become weaker. Only adding time and hardening are allowed. Any new request or setting must preserve that.
- Keep Spotify internals in `extension/src/compat.ts`. Production Spotify strips `data-testid`, so use surviving attributes (`data-top-bar-search`, `aria-*`, ids) or Spicetify's stable `main-*` classes. Verify a selector against the installed `xpui` bundle before using it.
- Conventional Commits. Everything in the repo is in English (code, comments, docs, commits). The user writes in Spanish; translate when writing to the repo.
- New architectural decisions get an ADR in `docs/adr/`.

## Status (2026-10-16)

Done and verified end-to-end on the user's machine (Spotify 1.3.5.3058, Windows, service running in WSL2 with mirrored networking):
- **M0:** repo and spikes.
- **M1:** core, protocol, service and CLI.
- **M2:** extension. It redirects routes, pauses other contexts, hides search, home, browse, the bell and friend activity, and filters the library sidebar.

**Next, M3 (Windows integration)**, see `docs/plan.md`:
1. Build `tunelock-service` for Windows (cross-compile from WSL, e.g. `cargo-xwin`, or natively) and add `install`/`uninstall`/`service` subcommands using the `windows-service` crate. The service runs as LocalSystem with auto-start and recovery.
2. Named pipe security descriptor (interprocess `ListenerOptionsExt::security_descriptor`) so interactive users can connect. Authorization stays in the engine.
3. Data directory `%ProgramData%\Tunelock`, restricted to SYSTEM and Administrators.
4. Resume-from-sleep power event: call `Engine::credit_downtime` and reset the ticker baseline (ADR 0002).
5. `SpotifyEnforcer`:
   - Fail-closed watchdog: during a session, if `Spotify.exe` runs and no extension heartbeat arrives within N seconds, terminate Spotify.
   - Protect the Spicetify and extension files during locked sessions.
   - Re-apply Spicetify in the user's session after Spotify updates (`WTSQueryUserToken` + `CreateProcessAsUser`).
6. Refuse service stop and uninstall during a locked session.

After M3: M4 (Tauri UI), M5 (NSIS installer, CI, release, SignPath).
