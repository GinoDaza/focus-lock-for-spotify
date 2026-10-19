# ADR 0007: Installer and first-run setup

- Status: accepted
- Date: 2026-10-16

## Context

Focus Lock has two halves with different privileges:

- The service runs as `LocalSystem` and needs an elevated, per-machine install.
- Spotify, Spicetify and the extension live in the user's profile. Spicetify warns against running elevated, and an elevated installer may run as a different account than the one that uses Spotify.

The user should not need a terminal for either half.

## Decision

**Installer.** The Tauri NSIS bundler builds a per-machine installer (`C:\Program Files\FocusLock`). It ships the app, `focuslock-service.exe` and the `focuslock` CLI. Hooks in `installer/hooks.nsh` call the service's own commands:

- before installing (an update), `focuslock-service uninstall`, which stops the old service and waits for it;
- after installing, `focuslock-service install`;
- before uninstalling, `focuslock-service uninstall`.

When the user ticks "Delete the application data" in the uninstaller, the service's data in `C:\ProgramData\FocusLock` (allowlist, presets, schedules, log) is removed as well; updates never remove it.

`uninstall` refuses while a locked session runs. Updates and uninstalls therefore stop until the session ends, and the installer says so. `uninstall` succeeds when the service is not installed, so a repair or a first install over leftover files works. The installer doesn't touch Spotify or Spicetify.

**First-run setup in the app.** The `focuslock-setup` crate, used by the app as the logged-on user, checks three steps and fixes what is missing:

1. Spotify from spotify.com (`%APPDATA%\Spotify`, or Spicetify's `spotify_path`). The Microsoft Store build is detected and explained; the app links to the download page.
2. Spicetify. If it is missing, the app downloads the **latest** release from the GitHub API, checks the SHA-256 digest that GitHub publishes for the asset, and extracts it to `%LOCALAPPDATA%\spicetify`, like the official installer does. The latest release is used because Spicetify has to keep up with Spotify updates.
3. The extension. The app embeds `extension/dist/focuslock.js` at build time and writes it to `%APPDATA%\spicetify\Extensions`. Spicetify searches that folder first, and Spicetify updates don't overwrite it. The app enables it in the config if needed and applies (`backup apply` on a stock Spotify, `apply` otherwise). It is up to date when the copy applied in Spotify matches the embedded file byte for byte.

The app opens on the Setup page while something is missing and flags it in the navigation. Setup is refused during a locked session; the service keeps the setup in place then (ADR 0005). After the first run, the service re-applies Spicetify after Spotify updates as before.

**Builds.** `scripts/build-installer.sh` builds the extension, the service and CLI, then the installer with `tauri build --runner cargo-xwin`. CI is deferred until the project is published.

## Consequences

- A new app version that ships a new extension reports "Update" on the Setup page until the user applies it.
- Uninstalling Focus Lock leaves Spicetify and the extension in place. Without the service, the extension doesn't restrict anything once its cached state expires.
- The installer isn't signed yet (SignPath comes with the public release), so SmartScreen warns on first run.
