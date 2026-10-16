// Mirror of `crates/protocol` (control channel). Durations are whole seconds.
// Keep in sync with the Rust types; the service validates every request.

export const PROTOCOL_VERSION = 1;

export type LockMode = "flexible" | "locked";

/** Playlist IDs (22 base62 characters). */
export type Allowlist = string[];

export interface SessionInfo {
  mode: LockMode;
  duration: number;
  remaining: number;
  allowlist: Allowlist;
}

export interface Status {
  protocol_version: number;
  service_version: string;
  session: SessionInfo | null;
}

export interface Preset {
  name: string;
  mode: LockMode;
  duration: number;
}

export type Request =
  | { type: "status" }
  | { type: "start_session"; mode: LockMode; duration: number; allowlist: Allowlist }
  | { type: "start_preset"; name: string }
  | { type: "cancel" }
  | { type: "extend"; by: number }
  | { type: "harden" }
  | { type: "list_presets" }
  | { type: "save_preset"; preset: Preset }
  | { type: "delete_preset"; name: string }
  | { type: "get_allowlist" }
  | { type: "set_allowlist"; allowlist: Allowlist };

/** Error responses never reach the UI: the backend turns them into rejections. */
export type Response =
  | ({ type: "status" } & Status)
  | { type: "presets"; presets: Preset[] }
  | { type: "allowlist"; allowlist: Allowlist | null }
  | { type: "schedules"; schedules: unknown[] }
  | { type: "done" };

// Setup of the Spotify side (`tunelock-setup`), not part of the service protocol.

export interface SetupStatus {
  spotify: "missing" | "store" | "ready";
  spicetify: boolean;
  extension: "missing" | "outdated" | "ready";
}

export function setupReady(s: SetupStatus): boolean {
  return s.spotify === "ready" && s.spicetify && s.extension === "ready";
}
