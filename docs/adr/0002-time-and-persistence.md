# ADR 0002: Session time and persistence

- Status: accepted
- Date: 2026-10-16

## Time

A session's duration is wall time: a 10 h session started at 08:00 ends at 18:00, even if the machine was off or asleep in between. Cold Turkey works the same way, and it is what users expect.

- **While the service runs**, every tick credits `min(monotonic elapsed, wall-clock elapsed)`. Changing the system clock therefore never shortens a running session.
- **Downtime** (shutdown, sleep) is credited from the wall clock, using the gap since the last checkpoint. Every tick saves a checkpoint. A negative gap counts as zero.
- On Windows, the service will call `credit_downtime` when it receives a resume-from-sleep power event, and will reset the tick baseline then so sleep time is not counted twice.

## Persistence

All state lives in a single JSON file (`state.json`) in the data directory. Each save writes a temporary file, syncs it and renames it over the original.

The plan proposed SQLite. The data is tiny (one session, a few presets, schedules and playlists), so one JSON file is simpler, needs no native dependency and is readable when debugging. We will move to SQLite only if we add session history or anything else that grows over time.

On Windows the data directory is `%ProgramData%\FocusLock`, restricted to SYSTEM and Administrators (milestone M3).

If the file is corrupt, the service logs the error and starts fresh instead of refusing to run.
