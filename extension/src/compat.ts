// Every assumption about Spotify's internals lives here, so a client update
// that breaks something should only need changes in this file.
// Verified against Spotify 1.3.5.3058.

/** Routes that never lead to other music. */
export const ALWAYS_ALLOWED_PATHS = ["/queue", "/lyrics"];

export const playlistPath = (id: string): string => `/playlist/${id}`;

/** Elements that only exist to find other music. Hidden during a session. */
export const HIDDEN_SELECTORS = [
  '[data-testid="search-input"]',
  '[data-testid="home-button"]',
  '[data-testid="browse-button"]',
];

/** Rows of the "Your Library" sidebar. */
const LIBRARY_ROW = '#Desktop_LeftSidebar_Id [role="row"]';

/** A library row is labelled by an element whose id contains its URI. */
const libraryRowFor = (id: string): string => `[id="listrow-title-spotify:playlist:${id}"]`;

/** CSS that hides navigation and every library row except allowed playlists. */
export function sessionCss(allowedIds: string[]): string {
  const keep = allowedIds.map(libraryRowFor).join(", ");
  const rows = keep ? `${LIBRARY_ROW}:not(:has(${keep}))` : LIBRARY_ROW;
  return `${[...HIDDEN_SELECTORS, rows].join(",\n")} { display: none !important; }`;
}

export function history(): Spicetify.History | undefined {
  return Spicetify.Platform?.History;
}

export function playbackContextUri(): string | undefined {
  return Spicetify.Player.data?.context?.uri;
}

export function isPlaying(): boolean {
  return Spicetify.Player.data?.isPaused === false;
}

/** Spicetify loads after extensions start, so every part may still be missing. */
type Loading = {
  Spicetify?: {
    showNotification?: unknown;
    Player?: { addEventListener?: unknown };
    Platform?: { History?: unknown };
  };
};

export function isReady(): boolean {
  const s = (globalThis as Loading).Spicetify;
  return Boolean(s?.showNotification && s.Player?.addEventListener && s.Platform?.History);
}
