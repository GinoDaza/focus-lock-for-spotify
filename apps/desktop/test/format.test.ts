import { describe, expect, it } from "vitest";
import {
  durationError,
  formatCountdown,
  formatDuration,
  parsePlaylistId,
  toParts,
  toSeconds,
} from "../src/format";

const ID = "37i9dQZF1DX8NTLI2TtZa6";

describe("formatDuration", () => {
  it("matches the core formatting", () => {
    expect(formatDuration(45)).toBe("45s");
    expect(formatDuration(0)).toBe("0s");
    expect(formatDuration(2 * 3600 + 30 * 60)).toBe("2h 30m");
    expect(formatDuration(2 * 86_400 + 3 * 3600 + 5 * 60 + 9)).toBe("2d 3h 5m");
  });
});

describe("formatCountdown", () => {
  it("pads the clock and prefixes days", () => {
    expect(formatCountdown(65)).toBe("00:01:05");
    expect(formatCountdown(86_400 + 4 * 3600 + 5 * 60 + 6)).toBe("1d 04:05:06");
    expect(formatCountdown(-3)).toBe("00:00:00");
  });
});

describe("duration parts", () => {
  it("round-trips", () => {
    const parts = { days: 1, hours: 2, minutes: 30 };
    expect(toParts(toSeconds(parts))).toEqual(parts);
  });

  it("validates like the service", () => {
    expect(durationError(0)).not.toBeNull();
    expect(durationError(60)).toBeNull();
    expect(durationError(30 * 86_400)).toBeNull();
    expect(durationError(30 * 86_400 + 1)).not.toBeNull();
  });
});

describe("parsePlaylistId", () => {
  it("accepts IDs, URIs and share links", () => {
    expect(parsePlaylistId(ID)).toBe(ID);
    expect(parsePlaylistId(` spotify:playlist:${ID} `)).toBe(ID);
    expect(parsePlaylistId(`https://open.spotify.com/playlist/${ID}?si=abc`)).toBe(ID);
    expect(parsePlaylistId(`https://open.spotify.com/intl-es/playlist/${ID}`)).toBe(ID);
  });

  it("rejects anything else", () => {
    expect(parsePlaylistId("")).toBeNull();
    expect(parsePlaylistId("nope")).toBeNull();
    expect(parsePlaylistId(`https://open.spotify.com/album/${ID}`)).toBeNull();
    expect(parsePlaylistId(`spotify:album:${ID}`)).toBeNull();
  });
});
