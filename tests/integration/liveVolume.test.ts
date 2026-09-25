import { beforeEach, describe, expect, it, vi } from "vitest";
import * as bindings from "@/generated/bindings";
import { commitAppVolume, previewAppVolume } from "@/lib/stores";

type Call = { cmd: string; volume: number; resolve: () => void };

let calls: Call[];

beforeEach(() => {
  calls = [];
  // Each call stays open until the test resolves it, like a slow IPC round trip.
  vi.spyOn(bindings, "invokeCommand").mockImplementation(
    (cmd: string, args?: Record<string, unknown>) =>
      new Promise((done) => {
        const volume = (args?.volume as number | undefined) ?? NaN;
        calls.push({ cmd, volume, resolve: () => done(undefined) });
      }),
  );
});

async function settle(): Promise<void> {
  for (let i = 0; i < 5; i += 1) await Promise.resolve();
}

/**
 * The level used to reach Windows only when the fader was released, so the
 * sound jumped instead of following the drag. It is now sent while the fader
 * moves, but a fast drag must not queue one IPC call per pointer event.
 */
describe("live volume while dragging", () => {
  it("reaches Windows during the drag, not only on release", async () => {
    previewAppVolume("drag-a.exe", 0.4);
    await settle();
    expect(calls.map((c) => [c.cmd, c.volume])).toEqual([["set_app_volume", 0.4]]);
  });

  it("keeps one call in flight and ends on the newest value", async () => {
    for (let step = 1; step <= 50; step += 1) previewAppVolume("drag-b.exe", step / 100);
    await settle();
    // 50 pointer events, one call in flight.
    expect(calls).toHaveLength(1);
    expect(calls[0].volume).toBe(0.01);

    calls[0].resolve();
    await settle();
    // The intermediate values were dropped; the newest one follows at once.
    expect(calls).toHaveLength(2);
    expect(calls[1].volume).toBe(0.5);

    const released = commitAppVolume("drag-b.exe", 0.5);
    calls[1].resolve();
    await settle();
    calls.slice(2).forEach((c) => c.resolve());
    await released;
    expect(calls.at(-1)?.volume).toBe(0.5);
    expect(calls.length).toBeLessThanOrEqual(3);
  });
});
