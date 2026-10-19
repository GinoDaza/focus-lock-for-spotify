// Playlist names and covers from Spotify's public oEmbed endpoint (no account
// or token needed). Purely cosmetic: on any failure the UI shows the ID.

export interface PlaylistMeta {
  title: string;
  thumbnail: string | null;
}

const CACHE_KEY = "focuslock.playlists.v1";
const memory = new Map<string, Promise<PlaylistMeta | null>>();

function readCache(): Record<string, PlaylistMeta> {
  try {
    return JSON.parse(localStorage.getItem(CACHE_KEY) ?? "{}") as Record<string, PlaylistMeta>;
  } catch {
    return {};
  }
}

function writeCache(id: string, meta: PlaylistMeta): void {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify({ ...readCache(), [id]: meta }));
  } catch {
    // Storage is optional.
  }
}

export function cachedMeta(id: string): PlaylistMeta | null {
  return readCache()[id] ?? null;
}

export function fetchMeta(id: string): Promise<PlaylistMeta | null> {
  let pending = memory.get(id);
  if (!pending) {
    const url = `https://open.spotify.com/oembed?url=${encodeURIComponent(`https://open.spotify.com/playlist/${id}`)}`;
    pending = fetch(url)
      .then((res) => (res.ok ? (res.json() as Promise<{ title?: string; thumbnail_url?: string }>) : null))
      .then((body) => {
        if (!body?.title) return null;
        const meta = { title: body.title, thumbnail: body.thumbnail_url ?? null };
        writeCache(id, meta);
        return meta;
      })
      .catch(() => null);
    memory.set(id, pending);
  }
  return pending;
}
