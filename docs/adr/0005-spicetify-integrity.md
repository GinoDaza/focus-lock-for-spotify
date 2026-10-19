# ADR 0005: Keeping the Spicetify setup in place

- Status: accepted
- Date: 2026-10-16

## Context

The Focus Lock extension only runs inside Spotify while Spicetify is applied. Spotify updates replace the patched UI with a stock one, and every piece of the setup lives in folders the user owns. The fail-closed watchdog (ADR 0004) already closes a Spotify client that runs without the extension. This ADR covers the other half: putting the setup back and keeping it unchanged during a locked session.

Spotify and Spicetify are installed per user, but the service runs as `LocalSystem`. The service acts on the user logged on at the console (`WTSGetActiveConsoleSessionId` + `WTSQueryUserToken`) and resolves their folders with `SHGetKnownFolderPath` and that user's token.

## Decision

`SpicetifyEnforcer` runs with the other enforcers every 2 s, only in the Windows service.

### Re-applying after updates

During any session, if `%APPDATA%\Spotify\Apps` holds a stock UI (`xpui.spa` present, or no `xpui\extensions\focuslock.js`), the service runs `spicetify -q -n backup apply` as the logged-on user (`CreateProcessAsUserW` with their token and environment block, no window, 2 minute timeout). It does not restart Spotify; a client still running unpatched is closed by the watchdog and the next launch loads the extension. Attempts are at least 5 minutes apart. The Spotify path follows `spotify_path` in Spicetify's `config-xpui.ini` when set.

### Protecting the setup during locked sessions

While a locked session runs, these folders get two explicit ACEs:

- `%APPDATA%\Spotify\Apps` (the patched UI, which holds the applied extension),
- `%LOCALAPPDATA%\spicetify` (the CLI and bundled extensions),
- `%APPDATA%\spicetify` (config and backups).

The ACEs are a deny for interactive users covering every right that changes files (write, append, attributes, delete, delete child, change permissions, take ownership), inherited by everything inside, and an OWNER RIGHTS entry that limits the owner to reading the permissions. The rest of the DACL is kept as it was. When the session ends, or when the service starts with no locked session, exactly those two explicit ACEs are removed.

Before re-applying Spicetify, the service lifts the protection, since Spicetify runs as the user and must write to those folders. The next check restores it.

The DACL edits work on SDDL strings in the pure `sddl` module. Only reading and writing the strings is Win32 (`windows/acl.rs`), together with the token, folder and process calls in `windows/user.rs`. These are the only `unsafe` modules.

## Consequences

- Spotify cannot update itself during a locked session. It updates after the session ends, and the service re-applies Spicetify at the next session.
- Flexible sessions do not protect files: they can be cancelled anyway.
- Only the console user is handled. Fast user switching and remote sessions are out of scope for v0.1.
