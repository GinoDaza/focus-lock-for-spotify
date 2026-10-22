// Every assumption about Spotify's internals lives here, so a client update
// that breaks something should only need changes in this file.
// Verified against Spotify 1.3.5.3058 (Windows).

/** Routes that never lead to other music. */
export const ALWAYS_ALLOWED_PATHS = ["/queue", "/lyrics"];

export const playlistPath = (id: string): string => `/playlist/${id}`;

/**
 * Elements that only exist to find other music. Hidden during a session.
 *
 * Production builds strip `data-testid`, so these rely on attributes that
 * survive: the top-bar search input carries `data-top-bar-search`, and
 * Spicetify maps the nav classes to stable `main-globalNav-*` names. The
 * first selector hides every ancestor of the search box that does not also
 * hold the history buttons or the right-hand controls, which is the whole
 * home + search + browse section. Spicetify can lag behind a Spotify update
 * and lose those class names (2.45.3 on 1.3.6), so the climb also stops below
 * anything holding the sidebar or the main view; otherwise it would hide the
 * whole window.
 */
export const HIDDEN_SELECTORS = [
  ":is(div, form):has([data-top-bar-search])" +
    ":not(:has(.main-globalNav-historyButtonsWrapper))" +
    ":not(:has(.main-globalNav-contentRight))" +
    ":not(:has(#Desktop_LeftSidebar_Id))" +
    ":not(:has(.main-view-container))",
  "[data-top-bar-search]",
  ".main-globalNav-navLink",
  // Notifications (what's new) and friend activity.
  ".main-actionButtons-button",
];

/** Rows of the "Your Library" sidebar. */
const LIBRARY_ROW = '#Desktop_LeftSidebar_Id [role="row"]';

/** A library row is labelled by an element whose id contains its URI. */
const libraryRowFor = (id: string): string => `[id="listrow-title-spotify:playlist:${id}"]`;

/**
 * Filter chips of the library sidebar (Playlists, Podcasts, ...) are Encore
 * chips with `role="checkbox"`; the selected one has `aria-checked="true"`.
 * The unfiltered list loads lazily and can leave allowed playlists out, so a
 * session keeps a filter selected. `chipClear` is the button that clears it.
 */
const LIBRARY_CHIP = '#Desktop_LeftSidebar_Id [data-encore-id="chip"]';
const LIBRARY_CHIP_CLEAR = '#Desktop_LeftSidebar_Id [data-encore-id="chipClear"]';

/**
 * Selects the first chip, which is "Playlists" in every locale, when no chip
 * is selected.
 */
export function ensurePlaylistsFilter(): void {
  const chips = document.querySelectorAll<HTMLElement>(LIBRARY_CHIP);
  if (chips.length === 0) return;
  if (![...chips].some((chip) => chip.getAttribute("aria-checked") === "true")) {
    chips[0]?.click();
  }
}

/** CSS that hides navigation and every library row except allowed playlists. */
export function sessionCss(allowedIds: string[]): string {
  const keep = allowedIds.map(libraryRowFor).join(", ");
  const rows = keep ? `${LIBRARY_ROW}:not(:has(${keep}))` : LIBRARY_ROW;
  return `${[...HIDDEN_SELECTORS, rows, LIBRARY_CHIP_CLEAR].join(",\n")} { display: none !important; }`;
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
