import { describe, expect, it } from "vitest";

import { sessionCss } from "../src/compat";
import { isAllowedContext, isAllowedPath, landingPath, playlistId } from "../src/guards";

const FOCUS = "37i9dQZF1DX8NTLI2TtZa6";
const PHONK = "0vvXsWCC9xrXsKd4FyS8kM";
const allowed = [`spotify:playlist:${FOCUS}`];

describe("playlistId", () => {
  it("reads current and legacy playlist URIs", () => {
    expect(playlistId(`spotify:playlist:${FOCUS}`)).toBe(FOCUS);
    expect(playlistId(`spotify:user:someone:playlist:${FOCUS}`)).toBe(FOCUS);
  });

  it("ignores everything else", () => {
    for (const uri of [undefined, "", `spotify:album:${FOCUS}`, `spotify:playlist:${FOCUS}x`]) {
      expect(playlistId(uri)).toBeNull();
    }
  });
});

describe("isAllowedContext", () => {
  it("allows only listed playlists", () => {
    expect(isAllowedContext(`spotify:playlist:${FOCUS}`, allowed)).toBe(true);
    expect(isAllowedContext(`spotify:playlist:${PHONK}`, allowed)).toBe(false);
    expect(isAllowedContext(`spotify:album:${FOCUS}`, allowed)).toBe(false);
    expect(isAllowedContext("spotify:collection:tracks", allowed)).toBe(false);
    expect(isAllowedContext(undefined, allowed)).toBe(false);
  });
});

describe("isAllowedPath", () => {
  it("allows the allowed playlist pages, the queue and lyrics", () => {
    expect(isAllowedPath(`/playlist/${FOCUS}`, allowed)).toBe(true);
    expect(isAllowedPath(`/playlist/${FOCUS}/`, allowed)).toBe(true);
    expect(isAllowedPath("/queue", allowed)).toBe(true);
    expect(isAllowedPath("/lyrics", allowed)).toBe(true);
  });

  it("blocks everything else", () => {
    for (const path of [
      "/",
      "/search",
      "/search/phonk",
      `/playlist/${PHONK}`,
      `/album/${FOCUS}`,
      "/collection/tracks",
      "/queuex",
      `/playlist/${FOCUS}/extra`,
    ]) {
      expect(isAllowedPath(path, allowed), path).toBe(false);
    }
  });
});

describe("landingPath", () => {
  it("is the first allowed playlist", () => {
    expect(landingPath([`spotify:playlist:${PHONK}`, ...allowed])).toBe(`/playlist/${PHONK}`);
    expect(landingPath([])).toBe("/");
  });
});

describe("sessionCss", () => {
  it("keeps allowed library rows and hides navigation", () => {
    const css = sessionCss([FOCUS]);
    expect(css).toContain("[data-top-bar-search]");
    expect(css).toContain(".main-globalNav-navLink");
    expect(css).toContain(`:not(:has([id="listrow-title-spotify:playlist:${FOCUS}"]))`);
    expect(css.trim().endsWith("{ display: none !important; }")).toBe(true);
  });

  it("never hides the sidebar or the main view with the search box", () => {
    const css = sessionCss([FOCUS]);
    expect(css).toContain(":not(:has(#Desktop_LeftSidebar_Id))");
    expect(css).toContain(":not(:has(.main-view-container))");
  });
});
