// Pure helpers shared by the views. Tested in `test/format.test.ts`.

const MINUTE = 60;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** Same as the service's limit (`tunelock_core::MAX_DURATION`). */
export const MAX_DURATION = 30 * DAY;

/** `2d 3h 5m`, or `45s` under a minute. Matches `tunelock_core::format_duration`. */
export function formatDuration(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  const parts = [
    [Math.floor(s / DAY), "d"],
    [Math.floor((s % DAY) / HOUR), "h"],
    [Math.floor((s % HOUR) / MINUTE), "m"],
  ]
    .filter(([value]) => (value as number) > 0)
    .map(([value, unit]) => `${value}${unit}`);
  return parts.length > 0 ? parts.join(" ") : `${s % MINUTE}s`;
}

/** `1d 04:05:06` or `04:05:06`, for a ticking countdown. */
export function formatCountdown(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  const pad = (n: number) => String(n).padStart(2, "0");
  const clock = `${pad(Math.floor((s % DAY) / HOUR))}:${pad(Math.floor((s % HOUR) / MINUTE))}:${pad(s % MINUTE)}`;
  const days = Math.floor(s / DAY);
  return days > 0 ? `${days}d ${clock}` : clock;
}

export interface DurationParts {
  days: number;
  hours: number;
  minutes: number;
}

export function toSeconds({ days, hours, minutes }: DurationParts): number {
  return Math.round(days * DAY + hours * HOUR + minutes * MINUTE);
}

export function toParts(seconds: number): DurationParts {
  return {
    days: Math.floor(seconds / DAY),
    hours: Math.floor((seconds % DAY) / HOUR),
    minutes: Math.floor((seconds % HOUR) / MINUTE),
  };
}

/** Why a duration would be rejected, or `null` if it is valid. */
export function durationError(seconds: number): string | null {
  if (!Number.isFinite(seconds) || seconds <= 0) return "Choose a duration.";
  if (seconds > MAX_DURATION) return `The maximum is ${MAX_DURATION / DAY} days.`;
  return null;
}

const ID = /^[A-Za-z0-9]{22}$/;

/**
 * Extracts a playlist ID from a bare ID, a `spotify:playlist:<id>` URI or an
 * `https://open.spotify.com/playlist/<id>?si=...` link. Mirrors
 * `PlaylistId::parse`; the service has the final say.
 */
export function parsePlaylistId(input: string): string | null {
  const trimmed = input.trim();
  let candidate = trimmed;
  if (trimmed.startsWith("spotify:playlist:")) {
    candidate = trimmed.slice("spotify:playlist:".length);
  } else if (trimmed.includes("open.spotify.com/")) {
    const path = trimmed.split("open.spotify.com/")[1]?.split(/[?#]/)[0] ?? "";
    const segments = path.split("/").filter(Boolean);
    const index = segments.indexOf("playlist");
    candidate = index >= 0 ? (segments[index + 1] ?? "") : "";
  }
  return ID.test(candidate) ? candidate : null;
}

/** Local wall-clock time `seconds` from `now`, e.g. "Fri 18:30". */
export function endsAt(seconds: number, now: Date): string {
  const end = new Date(now.getTime() + seconds * 1000);
  const sameDay = end.toDateString() === now.toDateString();
  const time = end.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
  return sameDay
    ? time
    : `${end.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" })} ${time}`;
}
