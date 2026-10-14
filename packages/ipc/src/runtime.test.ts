import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  parseRuntimeValue,
  parseRuntimeSnapshot,
  parseLoadResponse,
  parseUnloadResponse,
  loadStProgram,
  setRuntimeInput,
  singleScan,
  resetRuntime,
  unloadProgram,
  getRuntimeSnapshot,
  IpcTransportError,
} from "./index";
import { CONTRACT_VERSION } from "./generated";
import {
  readySnapshot,
  runtimeFault,
} from "../../../tests/frontend/runtime-fixtures";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(isTauri).mockReturnValue(true);
});
describe("runtime wire validation", () => {
  it("accepts the shortest JSON decimal encoding of maximum finite binary32", () => {
    const value = { type: "REAL" as const, value: 3.4028235e38 };
    expect(parseRuntimeValue(value)).toEqual(value);
  });
  it("accepts all checked Alpha tags and exact maximum TIME", () => {
    for (const value of [
      { type: "BOOL", value: true },
      { type: "INT", value: -32768 },
      { type: "REAL", value: 1.5 },
      { type: "TIME", microseconds: "18446744073709551615" },
    ])
      expect(parseRuntimeValue(value)).toEqual(value);
  });
  it.each([
    null,
    { type: "BOOL", value: 1 },
    { type: "INT", value: 32768 },
    { type: "INT", value: 1.5 },
    { type: "REAL", value: NaN },
    { type: "REAL", value: Infinity },
    { type: "REAL", value: 1e100 },
    { type: "REAL", value: -0 },
    { type: "TIME", microseconds: 9007199254740992 },
    { type: "TIME", microseconds: "18446744073709551616" },
    { type: "TIME", microseconds: "01" },
    { type: "TIME", microseconds: "-1" },
    { type: "DINT", value: 1 },
    { type: "BOOL", value: true, extra: 1 },
  ])("rejects malformed typed value %j", (value) => {
    expect(() => parseRuntimeValue(value)).toThrow(IpcTransportError);
  });
  it("accepts coherent ready and faulted complete observations", () => {
    const s = readySnapshot();
    expect(parseRuntimeSnapshot(s)).toEqual(s);
    const fault = { ...s, state: "FAULTED", fault: runtimeFault };
    expect(parseRuntimeSnapshot(fault)).toEqual(fault);
  });
  it.each([
    { contractVersion: CONTRACT_VERSION - 1 },
    { sessionId: "0" },
    { sessionId: "01" },
    { sessionId: 1 },
    { completedScans: 9007199254740992 },
    { simulationTimeMicros: 1 },
    { operationBudget: "0" },
    { cyclePeriodMicros: "0" },
    { state: "RUNNING" },
    {
      variables: [
        { ...readySnapshot().variables[0], value: { type: "INT", value: 1 } },
      ],
    },
    {
      functionBlocks: [
        {
          kind: "TON",
          instanceId: "1",
          name: "D",
          q: false,
          etMicros: "0",
          activeSince: "0",
        },
      ],
    },
    { fault: runtimeFault },
    { extra: "IR" },
  ])("rejects incompatible snapshot fields %j", (fields) => {
    expect(() =>
      parseRuntimeSnapshot({ ...readySnapshot(), ...fields }),
    ).toThrow(IpcTransportError);
  });
  it("rejects duplicate variable identity and inconsistent fault metadata", () => {
    const s = readySnapshot();
    expect(() =>
      parseRuntimeSnapshot({
        ...s,
        variables: [s.variables[0], s.variables[0]],
      }),
    ).toThrow();
    expect(() =>
      parseRuntimeSnapshot({
        ...s,
        state: "FAULTED",
        fault: {
          ...runtimeFault,
          relatedVariable: { variableId: "9", name: "Wrong" },
        },
      }),
    ).toThrow();
    expect(() =>
      parseRuntimeSnapshot({
        ...s,
        state: "FAULTED",
        fault: {
          ...runtimeFault,
          span: { start: { line: 1, column: 2 }, end: { line: 1, column: 1 } },
        },
      }),
    ).toThrow();
  });
  it("strictly validates load discriminants and domain compiler rejection", () => {
    const failure = {
      kind: "compileFailed",
      diagnostics: [
        {
          code: "PLC-ST-002",
          severity: "error",
          message: "Expected PROGRAM.",
          span: null,
        },
      ],
    };
    expect(parseLoadResponse(failure)).toEqual(failure);
    expect(
      parseLoadResponse({ kind: "loaded", snapshot: readySnapshot() }),
    ).toEqual({ kind: "loaded", snapshot: readySnapshot() });
    for (const x of [
      { kind: "compiled", snapshot: readySnapshot() },
      { kind: "loaded", snapshot: readySnapshot(), plan: {} },
      { kind: "compileFailed", diagnostics: [] },
    ])
      expect(() => parseLoadResponse(x)).toThrow();
  });
  it("strictly validates unload version and generation", () => {
    expect(
      parseUnloadResponse({
        contractVersion: CONTRACT_VERSION,
        sessionId: "1",
      }),
    ).toEqual({ contractVersion: CONTRACT_VERSION, sessionId: "1" });
    for (const x of [
      { contractVersion: 0, sessionId: "1" },
      { contractVersion: CONTRACT_VERSION, sessionId: "0" },
      { contractVersion: CONTRACT_VERSION, sessionId: "1", extra: true },
    ])
      expect(() => parseUnloadResponse(x)).toThrow();
  });
});
describe("runtime commands", () => {
  it("uses one typed native load request with no frontend defaults or executable data", async () => {
    vi.mocked(invoke).mockResolvedValue({
      kind: "loaded",
      snapshot: readySnapshot(),
    });
    await expect(loadStProgram("PROGRAM Main END_PROGRAM")).resolves.toEqual({
      kind: "loaded",
      snapshot: readySnapshot(),
    });
    expect(invoke).toHaveBeenCalledWith("load_st_program", {
      request: {
        contractVersion: CONTRACT_VERSION,
        source: "PROGRAM Main END_PROGRAM",
      },
    });
  });
  it("set-input sends exact value/session/variable identities", async () => {
    vi.mocked(invoke).mockResolvedValue(readySnapshot());
    await setRuntimeInput("1", "4", {
      type: "TIME",
      microseconds: "18446744073709551615",
    });
    expect(invoke).toHaveBeenCalledWith("set_runtime_input", {
      request: {
        contractVersion: CONTRACT_VERSION,
        sessionId: "1",
        variableId: "4",
        value: { type: "TIME", microseconds: "18446744073709551615" },
      },
    });
  });
  it.each([
    ["single_scan", singleScan],
    ["reset_runtime", resetRuntime],
    ["get_runtime_snapshot", getRuntimeSnapshot],
  ] as const)(
    "%s targets exactly one generation",
    async (command, operation) => {
      vi.mocked(invoke).mockResolvedValue(readySnapshot());
      expect(await operation("1")).toEqual(readySnapshot());
      expect(invoke).toHaveBeenCalledWith(command, {
        request: { contractVersion: CONTRACT_VERSION, sessionId: "1" },
      });
    },
  );
  it("unload preserves targeted identity", async () => {
    vi.mocked(invoke).mockResolvedValue({
      contractVersion: CONTRACT_VERSION,
      sessionId: "1",
    });
    await unloadProgram("1");
    expect(invoke).toHaveBeenCalledWith("unload_program", {
      request: { contractVersion: CONTRACT_VERSION, sessionId: "1" },
    });
  });
  it("student runtime fault is response data, not a transport exception", async () => {
    const fault = { ...readySnapshot(), state: "FAULTED", fault: runtimeFault };
    vi.mocked(invoke).mockResolvedValue(fault);
    expect((await singleScan("1")).fault?.code).toBe("PLC-RUNTIME-003");
  });
  it("rejects browser-only execution and structured transport failures", async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    await expect(singleScan("1")).rejects.toMatchObject({
      code: "APP-IPC-003",
    });
    expect(invoke).not.toHaveBeenCalled();
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockRejectedValue({
      code: "APP-RUNTIME-STALE-SESSION",
      message: "Load again.",
    });
    await expect(singleScan("1")).rejects.toMatchObject({
      code: "APP-RUNTIME-STALE-SESSION",
    });
  });
  it("malformed IDs are rejected before invocation", async () => {
    await expect(singleScan("01")).rejects.toMatchObject({
      code: "APP-IPC-002",
    });
    expect(invoke).not.toHaveBeenCalled();
  });
});
