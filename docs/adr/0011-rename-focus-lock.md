# ADR 0011: Rename to Focus Lock for Spotify

- Status: accepted
- Date: 2026-10-18
- Supersedes: ADR 0008

## Context

"Tunelock" says nothing about what the project does, so people searching for a way to lock Spotify to a set of playlists would not find it. Searches for "spotify focus lock" found no existing product or repository with that name.

## Decision

- The product is named **Focus Lock**, and the repository **focus-lock-for-spotify**. "for Spotify" is a descriptor, as in ADR 0008. The README title is "Focus Lock for Spotify".
- The short identifier is `focuslock`: crates (`focuslock-service`, `focuslock-cli`, ...), the CLI, the control socket, the extension file (`focuslock.js`) and the `FOCUSLOCK_DATA_DIR` variable.
- Installed names use `FocusLock`: the Windows service, `C:\Program Files\FocusLock`, `C:\ProgramData\FocusLock` and the installer file.
- The disclaimer of non-affiliation with Spotify AB, Spicetify and Cold Turkey and the rule about our own icon from ADR 0008 still apply.

## Consequences

- An installation made under the old name is not migrated: install the new version and remove the old one with its own uninstaller.
- GitHub redirects the old repository URL after the repository is renamed.
