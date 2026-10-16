// Typed calls to the service through the Tauri backend's `request` command.

import { invoke } from "@tauri-apps/api/core";
import type { Allowlist, LockMode, Preset, Request, Response, Status } from "./protocol";

type Of<T extends Response["type"]> = Extract<Response, { type: T }>;

async function request<T extends Response["type"]>(req: Request, expected: T): Promise<Of<T>> {
  const response = await invoke<Response>("request", { request: req });
  if (response.type !== expected) {
    throw new Error(`unexpected response from the service: ${response.type}`);
  }
  return response as Of<T>;
}

async function done(req: Request): Promise<void> {
  await request(req, "done");
}

export const api = {
  async status(): Promise<Status> {
    const { type: _, ...status } = await request({ type: "status" }, "status");
    return status;
  },
  start: (mode: LockMode, duration: number, allowlist: Allowlist) =>
    done({ type: "start_session", mode, duration, allowlist }),
  startPreset: (name: string) => done({ type: "start_preset", name }),
  cancel: () => done({ type: "cancel" }),
  extend: (by: number) => done({ type: "extend", by }),
  harden: () => done({ type: "harden" }),
  async presets(): Promise<Preset[]> {
    return (await request({ type: "list_presets" }, "presets")).presets;
  },
  savePreset: (preset: Preset) => done({ type: "save_preset", preset }),
  deletePreset: (name: string) => done({ type: "delete_preset", name }),
  async allowlist(): Promise<Allowlist> {
    return (await request({ type: "get_allowlist" }, "allowlist")).allowlist ?? [];
  },
  setAllowlist: (allowlist: Allowlist) => done({ type: "set_allowlist", allowlist }),
};

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
