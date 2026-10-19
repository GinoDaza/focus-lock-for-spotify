# Research: restricted focus mode for Spotify (Cold Turkey style)

> Prior-art research done before starting Focus Lock (October 2026). It was produced with an AI research agent; links were checked to exist but details were not all verified.

## 1. Verdict

**Does not exist** as a complete solution for the Spotify desktop client. It exists **partially** if you combine web URL blockers with the browser player, but that loses the native desktop app and leaves gaps in navigation.

> **In one line:** no software or mod currently enforces a strict playback allowlist (only certain playlists) inside the official Spotify desktop client together with a tamper-resistant time lock.

## 2. Tools evaluated

| Name | Link | What it does | What it lacks | Open source | Windows | Hard to undo |
|---|---|---|---|---|---|---|
| **Spicetify + extensions** (`focus-mode-spicetify`, `spotify-control-toolkit`) | [spicetify/cli](https://github.com/spicetify/cli), [focus-mode-spicetify](https://github.com/fatihkaan22/focus-mode-spicetify), [spotify-control-toolkit](https://github.com/SDavid33/spotify-control-toolkit) | Injects JS/CSS into the official desktop client. Can hide navigation and skip banned songs or artists. | No exclusive playlist allowlist, no timers, schedules or lock mode, no tamper resistance. | Yes (LGPL-2.1 / MIT) | Yes | Low |
| **Cold Turkey Blocker** | [getcoldturkey.com](https://getcoldturkey.com/) | Blocker with timers, schedules and a strict tamper-resistant lock. Blocks whole executables and URLs. | Cannot allowlist content inside `Spotify.exe`; does not modify Spotify's UI. Closed source. | No | Yes | High |
| **Axorith** | [axorithlabs/axorith](https://github.com/axorithlabs/axorith) | Modern Cold Turkey alternative in .NET with a client–service architecture (`Axorith.Host`), presets and blocks. | Blocks whole executables or domains; cannot restrict screens inside Spotify. | Source-available (BSL 1.1) | Yes | Medium |
| **FocusMe** | [focusme.com](https://focusme.com/) | Distraction blocker with a "Forced Mode" lock, schedules, app and site control. | Paid and closed. Only blocks `Spotify.exe` as a whole or web URLs. | No | Yes | High |
| **LeechBlock NG** | [proginosko/LeechBlockNG](https://github.com/proginosko/LeechBlockNG) | Browser extension with URL allow/block lists, schedules and a strict "Lockdown" mode. | Browser only; no control over the Spotify desktop client. | Yes (MPL-2.0) | Via browser | Medium |
| **Cold Turkey Micromanager** | [getcoldturkey.com/micromanager](https://getcoldturkey.com/micromanager/) | Restricts Windows to an allowlist of applications. | Works per window or executable; inside Spotify everything stays reachable. Closed source. | No | Yes | High |
| **Freedom** | [freedom.to](https://freedom.to/) | Commercial cross-platform blocker with "Locked Mode" and scheduled sessions. | Blocks whole processes or web traffic; no allowlist inside Spotify. | No | Yes | Medium/High |
| **Spotube** | [KRTirtho/spotube](https://github.com/KRTirtho/spotube) | Lightweight open-source Spotify desktop client (Flutter). | No kiosk mode, allowlists, timers or locks. | Yes (GPL-3.0 / BSD-4-Clause) | Yes | Low |
| **Spotify Community ideas** | [Idea Submissions](https://community.spotify.com/t5/Idea-Submissions/) | Users requesting "Kiosk Mode", "playlist allowlist" and "Study Mode". | Not implemented by Spotify. | — | — | — |

## 3. Best candidates to build on

1. **Spicetify CLI:** the only mature open-source way to modify the official desktop client on Windows without patching binaries. Through `Spicetify.Platform.History` and `Spicetify.Player`, an extension can suppress navigation and keep playback within the allowed playlist URIs. It has no lock of its own, so it needs a system-level companion.
2. **Axorith:** the closest open-source reference for the lock side: a background service decoupled from the UI, with session timers, recurring schedules and headless operation. Its license (BSL 1.1) must be checked before reusing code.
3. **LeechBlock NG:** proven logic for timers, day/hour schedules and an irrevocable "Lockdown" mode. Its JavaScript time-window model is a useful reference.

## 4. Proposed architecture: combining open-source pieces

No tool covers both the restricted Spotify UI and a tamper-resistant time lock, so the solution combines two components:

```
 ┌─────────────────────────────────────────────────────────────┐
 │                          WINDOWS                            │
 │  ┌───────────────────────────────────────────────────────┐  │
 │  │        LOCAL GUARDIAN SERVICE (background)            │  │
 │  │        - Owns time and the lock mode                  │  │
 │  │        - Protects its own files during a lock         │  │
 │  └──────────────────────────┬────────────────────────────┘  │
 │                             │ lock state                    │
 │                             ▼                               │
 │  ┌───────────────────────────────────────────────────────┐  │
 │  │             SPOTIFY DESKTOP (official client)         │  │
 │  │  ┌─────────────────────────────────────────────────┐  │  │
 │  │  │      SPICETIFY EXTENSION (UI logic)             │  │  │
 │  │  │  - CSS: hide search, home, library, feed        │  │  │
 │  │  │  - JS: allow only the playlist routes           │  │  │
 │  │  │  - JS: pause anything outside the allowlist     │  │  │
 │  │  └─────────────────────────────────────────────────┘  │  │
 │  └───────────────────────────────────────────────────────┘  │
 └─────────────────────────────────────────────────────────────┘
```

**Component A, the UI "jail" (Spicetify extension):**
- Injected CSS hides search, home, browse, the library and recommendations, leaving the cover, playback controls and the allowed playlists.
- A navigation guard on `Spicetify.Platform.History.listen()` redirects any route outside the allowed playlist URIs.
- A playback guard on `Spicetify.Player`'s `songchange` event pauses anything whose context is not an allowed playlist.

**Component B, the lock (local Windows service):**
- A background service that stores the session end time and offers two modes. **Flexible** sessions can be cancelled at any time. **Locked** sessions ignore every cancel request until they end.
- During a lock it protects the extension and its configuration files.
- When Spotify starts, the extension reads the service's state and applies the restricted UI automatically. When the time runs out, the service lifts the restrictions and Spotify returns to normal.
