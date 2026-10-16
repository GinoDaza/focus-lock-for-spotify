// Pure allow/deny rules. Everything Spotify-specific lives in compat.ts.

import { ALWAYS_ALLOWED_PATHS, playlistPath } from "./compat";

const PLAYLIST_URI = /(?:^|:)playlist:([A-Za-z0-9]{22})$/;
const PLAYLIST_PATH = /^\/playlist\/([A-Za-z0-9]{22})\/?$/;

/** Extracts the playlist ID from `spotify:playlist:<id>` (or the legacy
 * `spotify:user:<name>:playlist:<id>`). */
export function playlistId(uri: string | undefined): string | null {
  return uri?.match(PLAYLIST_URI)?.[1] ?? null;
}

function allowedIds(allowedUris: string[]): Set<string> {
  return new Set(allowedUris.map(playlistId).filter((id): id is string => id !== null));
}

export function isAllowedContext(contextUri: string | undefined, allowedUris: string[]): boolean {
  const id = playlistId(contextUri);
  return id !== null && allowedIds(allowedUris).has(id);
}

export function isAllowedPath(pathname: string, allowedUris: string[]): boolean {
  if (ALWAYS_ALLOWED_PATHS.some((path) => pathname === path || pathname.startsWith(`${path}/`))) {
    return true;
  }
  const id = pathname.match(PLAYLIST_PATH)?.[1];
  return id !== undefined && allowedIds(allowedUris).has(id);
}

/** Where to send the user when they wander off. */
export function landingPath(allowedUris: string[]): string {
  const [first] = [...allowedIds(allowedUris)];
  return first ? playlistPath(first) : "/";
}
