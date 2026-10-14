import { beforeEach, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  parseExecutionWake,
  parseRuntimeSnapshot,
  startExecution,
  pauseExecution,
  stopExecution,
  parseTracePage,
} from "./index";
import { CONTRACT_VERSION } from "./generated";
import { readySnapshot } from "../../../tests/frontend/runtime-fixtures";
import { traceEntry, tracePage } from "../../../tests/frontend/trace-fixtures";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(isTauri).mockReturnValue(true);
});
it("strictly validates v7 live and historical execution modes and canonical run identities", () => {
  for (const executionMode of ["STOPPED", "RUNNING", "PAUSED"] as const) {
    const s = {
      ...readySnapshot(),
      executionMode,
      executionRunId: "18446744073709551615",
    };
    expect(parseRuntimeSnapshot(s)).toEqual(s);
    const p = tracePage([traceEntry("2", "ScanCompleted", s)]);
    expect(parseTracePage(p)).toEqual(p);
  }
  for (const patch of [
    { contractVersion: 6 },
    { executionMode: undefined },
    { executionMode: "running" },
    { executionMode: "FAULTED" },
    { executionMode: "RUNNING", executionRunId: null },
    { executionRunId: "01" },
    { executionRunId: 1 },
    { executionRunId: "18446744073709551616" },
    { state: "FAULTED", executionMode: "RUNNING" },
    { epoch: "1" },
  ])
    expect(() =>
      parseRuntimeSnapshot({ ...readySnapshot(), ...patch }),
    ).toThrow();
});
it("wake is identity only, rejects state payloads, wrong versions and malformed identities", () => {
  const w = { contractVersion: CONTRACT_VERSION, sessionId: "1", runId: "2" };
  expect(parseExecutionWake(w)).toEqual(w);
  for (const value of [
    null,
    [],
    { ...w, contractVersion: 6 },
    { ...w, sessionId: "0" },
    { ...w, runId: "01" },
    { ...w, runId: "18446744073709551616" },
    { ...w, snapshot: readySnapshot() },
    { ...w, epoch: "2" },
  ])
    expect(() => parseExecutionWake(value)).toThrow();
});
it("control commands send only strict version/session targets and validate confirmed responses", async () => {
  vi.mocked(invoke).mockResolvedValue(readySnapshot());
  for (const [client, command] of [
    [startExecution, "start_execution"],
    [pauseExecution, "pause_execution"],
    [stopExecution, "stop_execution"],
  ] as const) {
    expect(await client("1")).toEqual(readySnapshot());
    expect(invoke).toHaveBeenLastCalledWith(command, {
      request: { contractVersion: CONTRACT_VERSION, sessionId: "1" },
    });
  }
});
it("control trace rows require post-transition modes and Faulted always requires STOPPED", () => {
  for (const [kind, mode] of [
    ["ExecutionStarted", "RUNNING"],
    ["ExecutionResumed", "RUNNING"],
    ["ExecutionPaused", "PAUSED"],
    ["ExecutionStopped", "STOPPED"],
  ] as const) {
    const e = traceEntry("2", kind, {
      ...readySnapshot(),
      executionMode: mode,
      executionRunId: "1",
    });
    expect(parseTracePage(tracePage([e])).entries[0]).toEqual(e);
    expect(() =>
      parseTracePage(
        tracePage([
          { ...e, executionMode: mode === "STOPPED" ? "RUNNING" : "STOPPED" },
        ]),
      ),
    ).toThrow();
  }
});
