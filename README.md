# Tunelock

**Focus lock for the Spotify desktop app on Windows.** Pick how long you want to focus, and Spotify is reduced to the playlists you allowed (for example, a binaural-beats playlist and an instrumental one) until the timer runs out.

> Status: early development. Nothing to install yet.

## Why

Website and app blockers can block Spotify entirely, but none of them let you keep Spotify open with *only* your focus music. Tunelock fills that gap.

## Planned features (v0.1)

- Timed focus sessions measured in hours or days.
- **Locked mode:** once started, the session cannot be cancelled until it ends.
- **Flexible mode:** start and stop with one click.
- Spotify shows only your allowed playlists. Search, home, library and recommendations are hidden.
- Presets such as "Study 10 h" or "Exam week 5 days". Recurring schedules come in v0.2.
- Closing the app does not end the session, because it runs in a background Windows service.

## How it works

- `tunelock-service` is a Windows service written in Rust. It owns the session state and the timer.
- A [Spicetify](https://spicetify.app) extension restricts the Spotify desktop client while a session is active.
- A desktop app built with Tauri provides the UI and a tray icon.

See [`docs/plan.md`](docs/plan.md) and [`docs/adr/`](docs/adr/) for the design.

## Limitations

- Windows only, with the Spotify desktop app from spotify.com. The Microsoft Store version is not supported by Spicetify.
- Tunelock adds strong friction, but it is not meant to be impossible to remove for an administrator of their own machine.
- Modifying the Spotify client is against Spotify's Terms of Use. Use it at your own risk.

## Project status and support

Tunelock is a personal project maintained in spare time. It comes without warranty and without support guarantees. Issues and PRs are welcome, but responses may be slow.

## License

[GPL-3.0](LICENSE). Tunelock is not affiliated with Spotify or Cold Turkey.
