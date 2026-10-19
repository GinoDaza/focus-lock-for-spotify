// Focus Lock Spicetify extension: while a session is active, Spotify can only
// show and play the allowed playlists. It never changes a session; it only
// reads the state from the local service.

import {
  ensurePlaylistsFilter,
  history,
  isPlaying,
  isReady,
  playbackContextUri,
  sessionCss,
} from "./compat";
import { isAllowedContext, isAllowedPath, landingPath, playlistId } from "./guards";
import {
  type Enforcement,
  type Fetched,
  formatRemaining,
  parseState,
  resolve,
} from "./state";

const SERVICE = "http://127.0.0.1:39817";
const POLL_MS = 2_000;
const HEARTBEAT_MS = 5_000;
const FILTER_MS = 500;
const CACHE_KEY = "focuslock:enforcement";
const STYLE_ID = "focuslock-style";
const BADGE_ID = "focuslock-badge";

let current: Enforcement | null = null;
let stopListening: (() => void) | null = null;
let filterTimer: number | null = null;

async function fetchState(): Promise<Fetched> {
  try {
    const response = await fetch(`${SERVICE}/v1/state`, { cache: "no-store" });
    if (!response.ok) return "unreachable";
    return parseState(await response.json()) ?? "unreachable";
  } catch {
    return "unreachable";
  }
}

function loadCache(): Enforcement | null {
  try {
    const raw = localStorage.getItem(CACHE_KEY);
    return raw ? (JSON.parse(raw) as Enforcement) : null;
  } catch {
    return null;
  }
}

function saveCache(value: Enforcement | null): void {
  try {
    if (value) localStorage.setItem(CACHE_KEY, JSON.stringify(value));
    else localStorage.removeItem(CACHE_KEY);
  } catch {
    // Storage may be unavailable; the service remains the source of truth.
  }
}

function enforceRoute(pathname: string): void {
  if (current && !isAllowedPath(pathname, current.allowedUris)) {
    history()?.replace(landingPath(current.allowedUris));
  }
}

function enforcePlayback(): void {
  if (!current || !isPlaying()) return;
  if (!isAllowedContext(playbackContextUri(), current.allowedUris)) {
    Spicetify.Player.pause();
    Spicetify.showNotification("Focus Lock: only your focus playlists can play right now.");
  }
}

function element<T extends HTMLElement>(id: string, tag: string): T {
  let node = document.getElementById(id) as T | null;
  if (!node) {
    node = document.createElement(tag) as T;
    node.id = id;
    document.body.append(node);
  }
  return node;
}

function render(enforcement: Enforcement): void {
  const ids = enforcement.allowedUris.map(playlistId).filter((id): id is string => id !== null);
  element<HTMLStyleElement>(STYLE_ID, "style").textContent = sessionCss(ids);

  const badge = element<HTMLDivElement>(BADGE_ID, "div");
  Object.assign(badge.style, {
    position: "fixed",
    bottom: "96px",
    right: "16px",
    zIndex: "9999",
    padding: "6px 12px",
    borderRadius: "999px",
    background: "rgba(0, 0, 0, 0.75)",
    color: "#fff",
    font: "600 12px/1.4 sans-serif",
    pointerEvents: "none",
  });
  badge.textContent = `🔒 Focus Lock · ${formatRemaining(enforcement.endsAt - Date.now())}`;
}

function apply(next: Enforcement | null): void {
  const wasActive = current !== null;
  current = next;

  if (next) {
    if (!wasActive) {
      stopListening = history()?.listen((location) => enforceRoute(location.pathname)) ?? null;
      filterTimer = window.setInterval(ensurePlaylistsFilter, FILTER_MS);
    }
    render(next);
    enforceRoute(history()?.location.pathname ?? "/");
    enforcePlayback();
  } else if (wasActive) {
    stopListening?.();
    stopListening = null;
    if (filterTimer !== null) clearInterval(filterTimer);
    filterTimer = null;
    document.getElementById(STYLE_ID)?.remove();
    document.getElementById(BADGE_ID)?.remove();
  }
}

async function poll(): Promise<void> {
  const next = resolve(await fetchState(), loadCache(), Date.now());
  saveCache(next);
  apply(next);
}

function heartbeat(): void {
  fetch(`${SERVICE}/v1/heartbeat`, { method: "POST" }).catch(() => {});
}

async function main(): Promise<void> {
  while (!isReady()) {
    await new Promise((resolve) => setTimeout(resolve, 300));
  }
  Spicetify.Player.addEventListener("songchange", enforcePlayback);
  Spicetify.Player.addEventListener("onplaypause", enforcePlayback);

  heartbeat();
  setInterval(heartbeat, HEARTBEAT_MS);
  await poll();
  setInterval(() => void poll(), POLL_MS);
}

void main();
