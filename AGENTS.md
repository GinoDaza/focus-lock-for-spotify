# AGENTS.md

Guide for AI agents and contributors working on Tunelock. Read this first, then `docs/plan.md` and `docs/adr/`.

## What this is

Tunelock is a Cold Turkey-style focus lock for the **Spotify desktop app on Windows**. During a session, Spotify can only show and play an allowlist of playlists. A **locked** session cannot be cancelled until it ends. The project is open source (GPL-3.0), and v0.1 targets Windows and Spotify only.

## Layout

| Path | What |
|---|---|
| `crates/core` | Pure domain model: `PlaylistId`/`Allowlist`, `Session`/`LockState` state machine, `Preset`, `Schedule`. No I/O. Time advances only via `LockState::advance`. |
| `crates/protocol` | Wire types. The control channel is line-delimited JSON over the local socket `tunelock` (named pipe on Windows). The extension channel is `ExtensionState` over loopback HTTP on port 39817. |
| `crates/service` | `tunelock-service`: `engine.rs` (requests and time policy), `store.rs` (atomic JSON file), `control.rs` (socket server), `extension.rs` (axum: `GET /v1/state`, `POST /v1/heartbeat`), `app.rs` (shared state and 5 s ticker), `enforcer.rs` (fail-closed Spotify watchdog), `spicetify.rs` (re-apply Spicetify, protect its folders during locked sessions), `sddl.rs` (pure DACL edits), `windows/` (SCM service, ACLs, console user token/folders/processes; `acl.rs` and `user.rs` are the only `unsafe` code). |
| `crates/client` | Blocking control-channel client shared by the CLI and the desktop app. |
| `crates/setup` | First-run setup of the Spotify side, run by the app as the user: detects Spotify (and the Store build), installs the latest Spicetify from GitHub (SHA-256 checked), writes the embedded extension to `%APPDATA%\spicetify\Extensions` and applies it (ADR 0007). |
| `crates/cli` | `tunelock` CLI (status/start/cancel/extend/harden/allowlist/preset/schedule). |
| `apps/desktop` | Tauri v2 app (`tunelock-app.exe`): a stateless React frontend. `src-tauri` exposes one command, `request`, that forwards a protocol `Request` to the service. `src/protocol.ts` mirrors the Rust types by hand. No tray (ADR 0006). It embeds `extension/dist/tunelock.js`, so build the extension first. |
| `installer/` | NSIS hooks: the installer registers and removes the service through `tunelock-service install/uninstall` (ADR 0007). |
| `scripts/build-installer.sh` | Builds everything and the installer from WSL. |
| `extension/` | Spicetify extension in TypeScript, bundled by esbuild to `dist/tunelock.js`. **All Spotify-specific selectors and routes live in `src/compat.ts`.** |
| `spikes/` | Throwaway experiments (`tunelock-spike.js` proved extension → localhost works). |
| `docs/` | `plan.md` (approved plan), ADRs, `idea.md` (original note) and `research.md` (prior art). |

## Commands

```sh
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test   # Rust (clippy pedantic)
cd extension && pnpm typecheck && pnpm test && pnpm build             # extension
SPICETIFY_EXTENSIONS_DIR=<dir> pnpm build                               # build + copy into Spicetify
cd apps/desktop && pnpm typecheck && pnpm test && pnpm build           # desktop frontend (build before the Rust crate)
cargo xwin build -p tunelock-desktop --release --features tauri/custom-protocol --target x86_64-pc-windows-msvc   # desktop app; without the feature it loads the dev server
./scripts/build-installer.sh                                            # installer -> target/x86_64-pc-windows-msvc/release/bundle/nsis/ (needs llvm, clang, lld, nsis)
cargo test -p tunelock-setup -- --ignored                               # downloads the real latest Spicetify
cargo run -p tunelock-service -- run --data-dir <dir>                   # service in foreground
cargo run -p tunelock-cli -- status                                     # CLI
cargo xwin build -p tunelock-service -p tunelock-cli --target x86_64-pc-windows-msvc   # Windows build
cargo xwin clippy -p tunelock-service --all-targets --target x86_64-pc-windows-msvc -- -D warnings
```

Windows `.exe` files (including test binaries in `target/x86_64-pc-windows-msvc/debug/deps/`) run directly from WSL through interop. Installing the service needs an elevated prompt on Windows, so the user does it.

The workspace lints forbid `unsafe` (`unsafe_code = "forbid"`). The service crate only denies it, so `src/windows/acl.rs` and `src/windows/user.rs` opt in with `#![allow(unsafe_code)]`; keep Win32 FFI in those modules and the logic in pure, tested ones.

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

**M3 (Windows integration), in progress** (see `docs/plan.md`, ADR 0004 and ADR 0005):
- Installed and verified on the user's machine: Windows service (LocalSystem, auto-start, recovery), pipe access for interactive users, locked sessions with the extension reporting in, and folder protection (writes denied during the lock, Spotify keeps playing, ACEs removed when it ends).
- Implemented, verified only when Spotify next updates: re-applying Spicetify (`spicetify -q -n backup apply` as the console user). Check `C:\ProgramData\Tunelock\service.log`.
- To update the installed service: stop it outside a session, replace `C:\Program Files\Tunelock\tunelock-service.exe`, start it (elevated prompt; the user does it).

**M4 (Tauri UI), done:** `apps/desktop`, verified on the user's machine against the installed service (ADR 0006).

**M5 (installer and first-run setup), in progress** (ADR 0007): the NSIS installer and the app's Setup page are built and tested; the install on the user's machine is still to be verified. The repo is public (github.com/GinoDaza/tunelock). CI and the release workflow are in `.github/workflows` (ADR 0009) but haven't run on GitHub yet. The first release and SignPath are still to do. Naming is in ADR 0008.
