// Mirrors `ExtensionState` in crates/protocol and decides what to enforce.

export const PROTOCOL_VERSION = 1;

export interface ExtensionState {
  protocol_version: number;
  active: boolean;
  /** Seconds. */
  remaining: number;
  allowed_uris: string[];
}

/** What the extension enforces right now. */
export interface Enforcement {
  allowedUris: string[];
  /** Epoch milliseconds. */
  endsAt: number;
}

export type Fetched = ExtensionState | "unreachable";

/** Validates a JSON body from `/v1/state`. */
export function parseState(json: unknown): ExtensionState | null {
  if (typeof json !== "object" || json === null) return null;
  const s = json as Record<string, unknown>;
  const valid =
    s.protocol_version === PROTOCOL_VERSION &&
    typeof s.active === "boolean" &&
    typeof s.remaining === "number" &&
    Array.isArray(s.allowed_uris) &&
    s.allowed_uris.every((uri) => typeof uri === "string");
  return valid ? (s as unknown as ExtensionState) : null;
}

/**
 * The service is the source of truth. When it cannot be reached, the last
 * known session keeps being enforced until its end time, so stopping the
 * service does not lift the restriction.
 */
export function resolve(
  fetched: Fetched,
  cached: Enforcement | null,
  now: number,
): Enforcement | null {
  if (fetched === "unreachable") {
    return cached !== null && cached.endsAt > now ? cached : null;
  }
  if (!fetched.active || fetched.allowed_uris.length === 0) return null;
  return {
    allowedUris: fetched.allowed_uris,
    endsAt: now + fetched.remaining * 1000,
  };
}

/** Formats milliseconds like the CLI: `2d 3h 5m`, or `45s` under a minute. */
export function formatRemaining(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const parts: string[] = [];
  const days = Math.floor(total / 86_400);
  const hours = Math.floor((total % 86_400) / 3_600);
  const minutes = Math.floor((total % 3_600) / 60);
  if (days) parts.push(`${days}d`);
  if (hours) parts.push(`${hours}h`);
  if (minutes) parts.push(`${minutes}m`);
  return parts.length ? parts.join(" ") : `${total % 60}s`;
}
