import { describe, expect, it } from "vitest";

import { formatRemaining, parseState, resolve } from "../src/state";

const uri = "spotify:playlist:37i9dQZF1DX8NTLI2TtZa6";
const active = { protocol_version: 1, active: true, remaining: 3600, allowed_uris: [uri] };
const idle = { protocol_version: 1, active: false, remaining: 0, allowed_uris: [] };

describe("parseState", () => {
  it("accepts the service's JSON", () => {
    expect(parseState(active)).toEqual(active);
    expect(parseState(idle)).toEqual(idle);
  });

  it("rejects other shapes and protocol versions", () => {
    for (const body of [null, "x", {}, { ...active, protocol_version: 2 }, { ...active, allowed_uris: [1] }]) {
      expect(parseState(body)).toBeNull();
    }
  });
});

describe("resolve", () => {
  const now = 1_000_000;

  it("follows the service when it answers", () => {
    expect(resolve(active, null, now)).toEqual({ allowedUris: [uri], endsAt: now + 3_600_000 });
    expect(resolve(idle, { allowedUris: [uri], endsAt: now + 5_000 }, now)).toBeNull();
  });

  it("keeps enforcing the cached session while the service is unreachable", () => {
    const cached = { allowedUris: [uri], endsAt: now + 5_000 };
    expect(resolve("unreachable", cached, now)).toEqual(cached);
    expect(resolve("unreachable", cached, now + 5_000)).toBeNull();
    expect(resolve("unreachable", null, now)).toBeNull();
  });

  it("never enforces an empty allowlist", () => {
    expect(resolve({ ...active, allowed_uris: [] }, null, now)).toBeNull();
  });
});

describe("formatRemaining", () => {
  it("matches the CLI format", () => {
    expect(formatRemaining(45_000)).toBe("45s");
    expect(formatRemaining((2 * 86_400 + 3 * 3_600 + 5 * 60) * 1000)).toBe("2d 3h 5m");
    expect(formatRemaining(-1)).toBe("0s");
  });
});
