# ADR 0004: Windows service and fail-closed watchdog

- Status: accepted
- Date: 2026-10-16

## Context

Milestone M3 turns `tunelock-service` into a real Windows service. A locked session must survive the user closing apps, rebooting and sleeping, and Spotify must not be usable without the extension during a session.

## Decision

### Service

- `tunelock-service install` registers the running executable as the `Tunelock` service: `LocalSystem`, automatic start, restart on every failure (the last recovery action repeats), and recovery also on non-crash exits. `service` is the hidden entry point the Service Control Manager uses; `run` stays the console mode for development.
- **Stopping is refused during a locked session.** The service reports the controls it accepts every second, and `STOP` is only included when no locked session runs. The control handler re-checks, in case a session became locked between two updates. `SHUTDOWN` is always accepted; the state is saved on every tick and change, so there is nothing to flush.
- **Uninstalling is refused during a locked session.** `uninstall` reads the saved state, credits the time since its checkpoint (without writing anything) and refuses if the session is still locked.
- **Sleep:** on `PBT_APMRESUMEAUTOMATIC` or `PBT_APMRESUMESUSPEND`, the service credits the time asleep from the wall clock (`Engine::resume`) and resets the ticker's monotonic baseline, as ADR 0002 planned.
- A service has no console, so it logs to `service.log` in the data directory, which starts over past 1 MiB.

### Access control

- **Data directory** (`%ProgramData%\Tunelock`): owned by Administrators, protected DACL with full control for SYSTEM and Administrators only, inherited by the files inside.
- **Control pipe** (`\\.\pipe\tunelock`): SYSTEM and Administrators get full access, interactive users get read and write. Remote clients are rejected. Authorization stays in the engine: every request is checked against the lock rules, whoever sends it.
- Win32 calls live in one module, `crates/service/src/windows/acl.rs`. The service crate denies `unsafe` instead of forbidding it so that module can opt in; every other crate still forbids it.

### Fail-closed watchdog (`SpotifyEnforcer`)

Every 2 s during an active session, the service looks for `Spotify.exe`. The oldest process of the tree identifies one run of the client.

- A newly seen client has 20 s to send its first extension heartbeat (startup and loading the extension take a few seconds).
- After that, the extension may go quiet for at most 2 minutes. It beats every 5 s, but Chromium can throttle timers in a hidden window to once a minute, so a tighter limit would close Spotify while it sits minimized.
- If either limit passes, the service terminates every `Spotify.exe` process. A restarted client counts as new and needs its own heartbeat.

The decision logic (`Watchdog`) is pure and unit-tested; process access sits behind the `SpotifyProcesses` trait (`sysinfo` in production).

## Consequences

- The service and CLI are cross-compiled from WSL with `cargo xwin`; Windows test binaries run through WSL interop.
- Spotify updates or a missing Spicetify setup now close Spotify during a session instead of letting it play unfiltered. Re-applying Spicetify automatically and protecting the extension's files come next in M3.
