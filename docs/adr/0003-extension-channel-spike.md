# ADR 0003: Extension ↔ service channel (spike result)

- Status: accepted
- Date: 2026-10-16

## Question

Can a Spicetify extension running inside the Spotify desktop client make HTTP requests to the Focus Lock service on `127.0.0.1`? The plan's fallback was to embed the state in the extension and re-apply Spicetify on every change.

## Spike

`spikes/focuslock-spike.js` was loaded into Spotify 1.3.5.3058 on Windows with Spicetify. The service ran in WSL2 (mirrored networking) on port 39817.

## Result

- `fetch("http://127.0.0.1:39817/v1/state")` succeeded, and the service received `POST /v1/heartbeat`.
- `Spicetify.Platform.History.listen` is available, so the navigation guard is feasible.

## Decision

Keep the loopback HTTP channel (polling plus heartbeat) and drop the fallback.

The extension enforces in layers, so that a Spotify UI change degrades only the cosmetic layer:
1. A **navigation guard** that redirects any route outside the allowlist.
2. A **playback guard** that stops any context outside the allowlist.
3. **CSS** that hides search, home, browse and the library sidebar. This layer is cosmetic only.
