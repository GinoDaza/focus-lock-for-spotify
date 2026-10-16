# Plan: Tunelock, a focus lock for the Spotify desktop app (Windows)

> **Status (2026-10-16):** M0, M1 and M2 are done; M3 is in progress (ADR 0004). Two things changed during implementation:
> - Persistence is a JSON file instead of SQLite (ADR 0002).
> - The extension does not render its own overlay. It uses navigation and playback guards plus cosmetic CSS (ADR 0003).
>
> The current project status is in `AGENTS.md`.

## Context

Distracting music (phonk, etc.) breaks concentration while studying or coding. Research (see [`research.md`](research.md)) found **no** tool that reduces the Spotify desktop app to a few playlists with a Cold Turkey-style time lock. Existing blockers block whole apps or websites; none allow "keep the app, but only this content".

Goal: an **open-source (GPL-3.0), free project called Tunelock** with a solid architecture and light maintenance. Scope for v0.1 is **Windows and the Spotify desktop app**. The browser and the web player are out of scope.

## Stack decisions (the standard choice for each piece)

| Piece | Technology | Why |
|---|---|---|
| Core + service | **Rust**, `windows-service` crate, `tokio` | Small native binary, no runtime. `windows-service` is the standard crate for Win32 services. |
| Service ↔ UI IPC | Named pipe (`interprocess`) with an ACL + typed JSON protocol (`serde`) | The standard Windows mechanism between a service and a user app. The ACL controls who can send commands. |
| Persistence | ~~SQLite~~ JSON file in `%ProgramData%\Tunelock`, ACL for SYSTEM/Admin only | Sessions, presets, schedules. Atomic writes. See ADR 0002. |
| Desktop UI | **Tauri v2** + React + TypeScript + Vite, with a tray icon | Today's standard for Rust desktop apps. Reuses the protocol types. |
| Spotify extension | TypeScript bundled with **esbuild** into a single Spicetify `.js` | Spicetify extensions are a single JS file (official docs). |
| Installer | Tauri's NSIS bundler (`perMachine`) + `installer_hooks` (.nsh) | NSIS hooks install and remove the service. |
| CI/CD | GitHub Actions `windows-latest`. Signing through SignPath Foundation (free for OSS; requires a published release and an automated build) | Standard for OSS on Windows. |

## Architecture

```
tunelock/
├─ Cargo.toml                 (workspace)
├─ crates/
│  ├─ core/       pure domain: Lock, Preset, Schedule, Allowlist, state machine
│  ├─ protocol/   IPC message types (serde), versioned
│  ├─ service/    `tunelock-service` binary: Windows service, IPC, persistence, enforcers
│  └─ cli/        `tunelock` CLI for development and diagnostics
├─ apps/desktop/  Tauri v2 (src-tauri/ + src/ React)
├─ extension/     Spicetify: src/*.ts → dist/tunelock.js (esbuild)
├─ installer/     .nsh hooks
├─ docs/          original idea, research, ADRs
└─ .github/workflows/
```

### Core (`crates/core`)
- **Lock state machine:** `Idle → Active{Flexible|Locked} → Idle`. A `Locked` session rejects every cancel request until it ends.
- **Time that resists clock changes:** the service credits elapsed time itself (see ADR 0002), so changing the system time never brings the end closer. Sessions resume correctly after a reboot or sleep.
- **Presets** ("Study 10 h", "Exam week 5 days") and **schedules** (Mon–Fri 9:00–18:00), modelled in v0.1 even though the schedule UI arrives in v0.2.
- **Allowlist** of `spotify:playlist:<id>` URIs.
- Pure and without I/O, so it is fully testable.

### Service (`crates/service`)
- Runs as **LocalSystem** with automatic start and recovery configured (restart on failure).
- **IPC server** (named pipe): the UI and the CLI send commands (`StartSession`, `Status`, `ListPresets`…).
- **Read-only local endpoint** (HTTP on `127.0.0.1`): the extension reads the session state and the allowlist and sends a heartbeat. Because it is read-only, exposing it cannot unlock anything.
- **Enforcers** behind a trait (`on_lock_start`, `on_tick`, `on_lock_end`). Supporting other apps later means adding an enforcer.
  - `SpotifyEnforcer`:
    1. When a session starts, checks that Spicetify is applied with the Tunelock extension, and re-applies it in the user's context if not.
    2. **Fail-closed:** if `Spotify.exe` runs during a session and the extension sends no heartbeat within N seconds, it closes Spotify. That matches the user's choice: "I'd rather hear nothing."
    3. Protects the extension's files with ACLs for the duration of the session and restores permissions afterwards.
  - Detects Spotify updates (which remove Spicetify) and re-applies it.
- Stopping or uninstalling the service is refused while a `Locked` session is active (the NSIS hook asks the service).

### Extension (`extension/`)
- **Navigation guard** (`Spicetify.Platform.History.listen`): any route outside the allowlist redirects to an allowed playlist.
- **Playback guard** (`Spicetify.Player` events): if the playing context is not in the allowlist, playback is paused.
- **Cosmetic CSS:** hides search, home, browse and the library rows that are not allowed. (The plan originally proposed a full-screen overlay; ADR 0003 replaced it with these layers.)
- Spotify's internal selectors and routes live in one file, `compat.ts`, so adapting to an update touches a single place.
- Without an active session, the extension does nothing and Spotify works normally.

### UI (`apps/desktop`)
- Tray icon with the status and a countdown.
- Screens: Home (start a flexible or locked session, in hours or days), Presets, Allowed playlists (paste the "Copy link" URL and extract the ID), Settings.
- A strong confirmation before a `Locked` session ("you won't be able to cancel it until…").
- Closing the UI does not end the session (it lives in the service).

## Known risks (resolved with spikes in phase 0)
1. ~~Can the extension reach `127.0.0.1` from inside the Spotify client?~~ **Yes** (ADR 0003).
2. **Spotify is installed per user** (`%APPDATA%`), but the service runs as SYSTEM. Applying Spicetify requires launching processes in the active user's session (`WTSQueryUserToken` + `CreateProcessAsUser`).
3. **The Microsoft Store version of Spotify is not compatible with Spicetify.** The installer must detect it and ask for the spotify.com version.
4. **No lock is absolute against a local administrator.** The goal is high friction (like Cold Turkey), not invulnerability. The README says so without explaining how to get around it.
5. **Spotify's terms of service:** modifying the client goes against them. The README warns about it.
6. **Environment:** development happens in WSL, but the service and Tauri are Windows software. Build with `cargo-xwin` or on Windows; CI uses `windows-latest`. Real tests run on Windows.

## Milestones

- **M0, repo and spikes:** `git init`, `.gitignore`, GPL-3.0 `LICENSE`, `README.md`, `.editorconfig`, docs moved to `docs/`, ADR for the stack, spikes for risks 1 and 2.
- **M1, core + service + CLI:** state machine, persistence, IPC, and `tunelock start 2h --locked` working end to end.
- **M2, extension:** guards, cosmetic CSS, connection to the service.
- **M3, Windows integration:** real Windows service, pipe ACL, protected data directory, resume-from-sleep handling, `SpotifyEnforcer` (re-apply Spicetify, fail-closed watchdog, ACLs, update detection).
- **M4, Tauri UI:** tray, start a session, presets, allowlist.
- **M5, release v0.1:** NSIS installer with service hooks, CI (fmt, clippy, tests, build), GitHub release. Then apply to SignPath.
- **v0.2:** schedule UI, session history, multiple profiles.

## Repository conventions
- Conventional Commits; protect `main` later.
- CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `pnpm typecheck`, `pnpm test`, and builds of Tauri and the extension.
- README: what it is, screenshots, installation, the Spotify terms warning, "personal project without support guarantees", and the code-signing policy (required by SignPath).

## Verification
- **Unit (core):** state transitions, refusing to cancel in `Locked` mode, resuming after a restart, clock changes having no effect, schedule windows.
- **Integration (service):** a test IPC client against the service in console mode.
- **Extension:** allowlist and guard logic with Vitest.
- **Manual end-to-end on Windows:** install → set 2 playlists → 5-minute `Locked` session → check that only those playlists are visible and playable, that closing the UI does not end it and that a reboot keeps it → when it expires, Spotify returns to normal.
