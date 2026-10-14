import { beforeEach, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  parseProcessSnapshot,
  parseRuntimeSnapshot,
  attachProcessScene,
  detachProcessScene,
  resetProcessScene,
  IpcTransportError,
} from "./index";
import { CONTRACT_VERSION } from "./generated";
import {
  tankSnapshot,
  readySnapshot,
} from "../../../tests/frontend/runtime-fixtures";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(isTauri).mockReturnValue(true);
});
it("accepts strict process and combined observations without evaluating sensors", () => {
  const s = tankSnapshot();
  expect(parseRuntimeSnapshot(s)).toEqual(s);
  expect(parseProcessSnapshot(null)).toBeNull();
  const supplied = {
    ...s.process!,
    levelPermille: 0,
    lowLevel: false,
    highLevel: true,
  };
  expect(parseProcessSnapshot(supplied)).toEqual(supplied);
});
it.each([-1, 1001, 1.5, NaN, Infinity, "800", null])(
  "rejects malformed tank level %j",
  (levelPermille) => {
    expect(() =>
      parseProcessSnapshot({ ...tankSnapshot().process, levelPermille }),
    ).toThrow(IpcTransportError);
  },
);
it.each([
  { kind: "OTHER" },
  { lowLevel: 1 },
  { highLevel: "false" },
  { inletValveOpen: null },
  { drainPumpRunning: 1 },
  { lowThresholdPermille: 1001 },
  { lowThresholdPermille: 900 },
  { fillRatePermillePerSecond: 0 },
  { drainRatePermillePerSecond: 1.5 },
  { remainder: 5 },
])("rejects malformed process field %j", (field) => {
  expect(() =>
    parseProcessSnapshot({ ...tankSnapshot().process, ...field }),
  ).toThrow(IpcTransportError);
});
it("rejects missing process fields and missing mandatory process envelope", () => {
  const p = { ...tankSnapshot().process };
  delete (p as Partial<typeof p>).levelPermille;
  expect(() => parseProcessSnapshot(p)).toThrow();
  const s = readySnapshot();
  delete (s as Partial<typeof s>).process;
  expect(() => parseRuntimeSnapshot(s)).toThrow();
});
it("checks authority role/scene/binding metadata without deriving physical behavior", () => {
  for (const mutate of [
    (s: ReturnType<typeof tankSnapshot>) => {
      s.process = null;
    },
    (s: ReturnType<typeof tankSnapshot>) => {
      s.variables[0]!.inputAuthority = "USER";
    },
    (s: ReturnType<typeof tankSnapshot>) => {
      s.variables[3]!.inputAuthority = "PROCESS";
    },
    (s: ReturnType<typeof tankSnapshot>) => {
      s.variables[1]!.name = "Other";
    },
  ]) {
    const s = tankSnapshot();
    mutate(s);
    expect(() => parseRuntimeSnapshot(s)).toThrow();
  }
  const user = readySnapshot();
  (user.variables[0] as unknown as Record<string, unknown>).inputAuthority =
    "FORCE";
  expect(() => parseRuntimeSnapshot(user)).toThrow();
});
it("process commands send exact targeted requests and return typed combined snapshots", async () => {
  vi.mocked(invoke).mockResolvedValue(tankSnapshot());
  await expect(attachProcessScene("1", "TANK_FILLING")).resolves.toEqual(
    tankSnapshot(),
  );
  expect(invoke).toHaveBeenLastCalledWith("attach_process_scene", {
    request: {
      contractVersion: CONTRACT_VERSION,
      sessionId: "1",
      sceneKind: "TANK_FILLING",
    },
  });
  await resetProcessScene("1");
  expect(invoke).toHaveBeenLastCalledWith("reset_process_scene", {
    request: { contractVersion: CONTRACT_VERSION, sessionId: "1" },
  });
  await detachProcessScene("1");
  expect(invoke).toHaveBeenLastCalledWith("detach_process_scene", {
    request: { contractVersion: CONTRACT_VERSION, sessionId: "1" },
  });
});
it("rejects unsupported scene requests and absent native service without substituting a model", async () => {
  await expect(
    attachProcessScene("1", "OTHER" as "TANK_FILLING"),
  ).rejects.toMatchObject({ code: "APP-IPC-002" });
  expect(invoke).not.toHaveBeenCalled();
  vi.mocked(isTauri).mockReturnValue(false);
  await expect(attachProcessScene("1", "TANK_FILLING")).rejects.toMatchObject({
    code: "APP-IPC-003",
  });
  expect(invoke).not.toHaveBeenCalled();
});
