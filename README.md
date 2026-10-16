# Tunelock — focus lock for Spotify

**Focus lock for the Spotify desktop app on Windows.** Pick how long you want to focus, and Spotify is reduced to the playlists you allowed (for example, a binaural-beats playlist and an instrumental one) until the timer runs out.

> Status: early development (v0.1). The installer works, but it isn't signed yet, so Windows SmartScreen warns on first run.

## Why

Website and app blockers can block Spotify entirely, but none of them let you keep Spotify open with *only* your focus music. Tunelock fills that gap.

## Features (v0.1)

- Timed focus sessions measured in hours or days.
- **Locked mode:** once started, the session cannot be cancelled until it ends.
- **Flexible mode:** start and stop with one click.
- Spotify shows only your allowed playlists. Search, home, library and recommendations are hidden.
- Presets such as "Study 10 h" or "Exam week 5 days". Recurring schedules come in v0.2.
- Closing the app does not end the session, because it runs in a background Windows service.

## Install

1. Install the Spotify desktop app from [spotify.com](https://www.spotify.com/download/). The Microsoft Store version is not supported.
2. Download `Tunelock_<version>_x64-setup.exe` from the [Releases](../../releases) page and run it. It asks for administrator rights because it installs a Windows service.
3. Open Tunelock. The Setup page installs Spicetify and the Tunelock extension if they are missing.
4. Pick your allowed playlists and start a session.

## How it works

- `tunelock-service` is a Windows service written in Rust. It owns the session state and the timer.
- A [Spicetify](https://spicetify.app) extension restricts the Spotify desktop client while a session is active.
- A desktop app built with Tauri provides the UI. Closing it does not end a session.

See [`docs/plan.md`](docs/plan.md) and [`docs/adr/`](docs/adr/) for the design.

## Limitations

- Windows only, with the Spotify desktop app from spotify.com. The Microsoft Store version is not supported by Spicetify.
- Tunelock adds strong friction, but it is not meant to be impossible to remove for an administrator of their own machine.
- Modifying the Spotify client is against Spotify's Terms of Use. Use it at your own risk.

## Project status and support

Tunelock is a personal project maintained in spare time. It comes without warranty and without support guarantees. Issues and PRs are welcome, but responses may be slow.

## License

[GPL-3.0](LICENSE).

Tunelock is an independent project. It is not affiliated with, endorsed by or sponsored by Spotify AB, Spicetify or Cold Turkey. Spotify is a trademark of Spotify AB, and Cold Turkey is a trademark of its owner. They are named here only to say what Tunelock works with.
