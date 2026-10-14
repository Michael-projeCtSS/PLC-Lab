import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import * as ipc from "@plc-lab/ipc";
import { useRuntimeSession } from "./useRuntimeSession";
import { coalescedRefresh } from "./coalesced-refresh";
import {
  readySnapshot,
  runtimeFault,
} from "../../../tests/frontend/runtime-fixtures";
import { CONTRACT_VERSION } from "../../ipc/src/generated";
vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  loadStProgram: vi.fn(),
  startExecution: vi.fn(),
  pauseExecution: vi.fn(),
  stopExecution: vi.fn(),
  getRuntimeSnapshot: vi.fn(),
  listenExecutionWake: vi.fn(),
  setRuntimeInput: vi.fn(),
  singleScan: vi.fn(),
  resetRuntime: vi.fn(),
  unloadProgram: vi.fn(),
}));
let wake: (event: ipc.ExecutionWake) => void;
const stopped = () => readySnapshot(),
  running = (run = "1"): ipc.RuntimeSnapshotDto => ({
    ...stopped(),
    executionMode: "RUNNING",
    executionRunId: run,
  }),
  paused = (): ipc.RuntimeSnapshotDto => ({
    ...running(),
    executionMode: "PAUSED",
  });
const signal = (runId = "1", sessionId = "1") =>
  wake({ contractVersion: CONTRACT_VERSION, sessionId, runId });
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(ipc.listenExecutionWake).mockImplementation(async (callback) => {
    wake = callback;
    return () => {};
  });
  vi.mocked(ipc.loadStProgram).mockResolvedValue({
    kind: "loaded",
    snapshot: stopped(),
  });
  vi.mocked(ipc.startExecution).mockResolvedValue(running());
  vi.mocked(ipc.getRuntimeSnapshot).mockImplementation(
    async () =>
      (await vi.mocked(ipc.startExecution).mock.results.at(-1)!
        .value) as ipc.RuntimeSnapshotDto,
  );
  vi.mocked(ipc.pauseExecution).mockResolvedValue(paused());
  vi.mocked(ipc.stopExecution).mockResolvedValue({
    ...running(),
    executionMode: "STOPPED",
  });
});
const mount = () => renderHook(() => useRuntimeSession("source", true));
it("confirms controls with backend mode, locks source/topology, keeps USER staging and Stop preserves state", async () => {
  const h = mount();
  await act(() => h.result.current.load());
  expect([
    h.result.current.canRun,
    h.result.current.canPause,
    h.result.current.canStop,
    h.result.current.canScan,
  ]).toEqual([true, false, false, true]);
  let confirm!: (s: ipc.RuntimeSnapshotDto) => void;
  vi.mocked(ipc.startExecution).mockReturnValueOnce(
    new Promise((r) => {
      confirm = r;
    }),
  );
  let request!: Promise<void>;
  act(() => {
    request = h.result.current.run();
  });
  expect(h.result.current.snapshot?.executionMode).toBe("STOPPED");
  expect(h.result.current.sourceLocked).toBe(true);
  act(() => {
    expect(h.result.current.invalidateSource()).toBe(false);
  });
  await act(async () => {
    confirm({
      ...running(),
      completedScans: "7",
      simulationTimeMicros: "700000",
    });
    await request;
  });
  expect([
    h.result.current.canRun,
    h.result.current.canPause,
    h.result.current.canStop,
    h.result.current.canScan,
  ]).toEqual([false, true, true, false]);
  expect(h.result.current.sourceLocked).toBe(true);
  expect(h.result.current.canManageProcess).toBe(false);
  expect(h.result.current.canEditInputs).toBe(true);
  act(() => h.result.current.invalidateSource());
  expect(h.result.current.current).toBe(true);
  await act(async () => {
    await h.result.current.scan();
    await h.result.current.load();
    await h.result.current.reset();
    await h.result.current.unload();
  });
  expect(ipc.singleScan).not.toHaveBeenCalled();
  expect(ipc.loadStProgram).toHaveBeenCalledTimes(1);
  expect(ipc.resetRuntime).not.toHaveBeenCalled();
  expect(ipc.unloadProgram).not.toHaveBeenCalled();
  await act(() => h.result.current.pause());
  expect([
    h.result.current.canRun,
    h.result.current.canPause,
    h.result.current.canStop,
    h.result.current.canScan,
  ]).toEqual([true, false, true, false]);
  expect(h.result.current.canEditInputs).toBe(true);
  vi.mocked(ipc.stopExecution).mockResolvedValueOnce({
    ...running(),
    executionMode: "STOPPED",
    completedScans: "7",
    simulationTimeMicros: "700000",
  });
  await act(() => h.result.current.stop());
  expect(h.result.current.snapshot?.completedScans).toBe("7");
  expect(h.result.current.snapshot?.simulationTimeMicros).toBe("700000");
  expect(h.result.current.canScan).toBe(true);
  expect(ipc.resetRuntime).not.toHaveBeenCalled();
});
it("coalesces wake bursts and ignores old run/session signals and replies after Pause/Stop/Resume", async () => {
  const h = mount();
  await act(() => h.result.current.load());
  await act(() => h.result.current.run());
  vi.mocked(ipc.getRuntimeSnapshot).mockClear();
  let complete!: (s: ipc.RuntimeSnapshotDto) => void;
  vi.mocked(ipc.getRuntimeSnapshot)
    .mockReturnValueOnce(
      new Promise((r) => {
        complete = r;
      }),
    )
    .mockResolvedValue({ ...running(), completedScans: "2" });
  act(() => {
    for (let i = 0; i < 100; i++) signal();
  });
  expect(ipc.getRuntimeSnapshot).toHaveBeenCalledTimes(1);
  await act(async () => complete({ ...running(), completedScans: "1" }));
  await waitFor(() => expect(ipc.getRuntimeSnapshot).toHaveBeenCalledTimes(2));
  expect(h.result.current.snapshot?.completedScans).toBe("2");
  vi.mocked(ipc.getRuntimeSnapshot).mockReturnValueOnce(
    new Promise((r) => {
      complete = r;
    }),
  );
  act(() => signal());
  await act(() => h.result.current.pause());
  await act(async () => complete({ ...running(), completedScans: "99" }));
  expect(h.result.current.snapshot?.executionMode).toBe("PAUSED");
  act(() => signal());
  expect(ipc.getRuntimeSnapshot).toHaveBeenCalledTimes(3);
  vi.mocked(ipc.startExecution).mockResolvedValueOnce(running("2"));
  vi.mocked(ipc.getRuntimeSnapshot).mockResolvedValue(running("2"));
  await act(() => h.result.current.run());
  act(() => {
    signal("1");
    signal("2", "999");
  });
  expect(ipc.getRuntimeSnapshot).toHaveBeenCalledTimes(4);
  await act(() => h.result.current.stop());
  act(() => signal("2"));
  expect(ipc.getRuntimeSnapshot).toHaveBeenCalledTimes(4);
});
it("requeries authoritative faulted STOPPED state and exposes APP refresh failures distinctly", async () => {
  const h = mount();
  await act(() => h.result.current.load());
  await act(() => h.result.current.run());
  vi.mocked(ipc.getRuntimeSnapshot).mockResolvedValue({
    ...running(),
    state: "FAULTED",
    executionMode: "STOPPED",
    fault: runtimeFault,
  });
  await act(async () => signal());
  expect(h.result.current.snapshot?.fault?.category).toBe("runtimeFault");
  expect([
    h.result.current.canRun,
    h.result.current.canPause,
    h.result.current.canStop,
    h.result.current.canScan,
  ]).toEqual([false, false, false, false]);
  expect(h.result.current.canReplace).toBe(true);
});
it("coalescer permits one running chain and one followup without an unbounded queue", async () => {
  let resolve!: () => void;
  const read = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise<void>((r) => {
          resolve = r;
        }),
    )
    .mockResolvedValue(undefined);
  const refresh = coalescedRefresh(read);
  for (let i = 0; i < 1000; i++) refresh.trigger();
  expect(read).toHaveBeenCalledTimes(1);
  resolve();
  await waitFor(() => expect(read).toHaveBeenCalledTimes(2));
  refresh.dispose();
  refresh.trigger();
  expect(read).toHaveBeenCalledTimes(2);
});
it("old session notifications after Unload cannot trigger reads or restore source association", async () => {
  const h = mount();
  await act(() => h.result.current.load());
  await act(() => h.result.current.run());
  vi.mocked(ipc.getRuntimeSnapshot).mockClear();
  await act(() => h.result.current.stop());
  vi.mocked(ipc.unloadProgram).mockResolvedValue({
    contractVersion: CONTRACT_VERSION,
    sessionId: "1",
  });
  await act(() => h.result.current.unload());
  act(() => signal());
  expect(ipc.getRuntimeSnapshot).not.toHaveBeenCalled();
  expect(h.result.current.snapshot).toBeNull();
  expect(h.result.current.current).toBe(false);
});
it("reads a terminal fault that preceded delivery of the Run confirmation", async () => {
  const h = mount();
  await act(() => h.result.current.load());
  let confirm!: (s: ipc.RuntimeSnapshotDto) => void;
  vi.mocked(ipc.startExecution).mockReturnValueOnce(
    new Promise((r) => {
      confirm = r;
    }),
  );
  vi.mocked(ipc.getRuntimeSnapshot).mockResolvedValue({
    ...running(),
    state: "FAULTED",
    executionMode: "STOPPED",
    fault: runtimeFault,
  });
  let request!: Promise<void>;
  act(() => {
    request = h.result.current.run();
    signal();
  });
  expect(ipc.getRuntimeSnapshot).not.toHaveBeenCalled();
  await act(async () => {
    confirm(running());
    await request;
  });
  expect(ipc.getRuntimeSnapshot).toHaveBeenCalledTimes(1);
  expect(h.result.current.snapshot?.executionMode).toBe("STOPPED");
  expect(h.result.current.snapshot?.fault).toEqual(runtimeFault);
});
it("surfaces application refresh failure separately from a student fault", async () => {
  const h = mount();
  await act(() => h.result.current.load());
  await act(() => h.result.current.run());
  vi.mocked(ipc.getRuntimeSnapshot).mockRejectedValue(
    new ipc.IpcTransportError(
      "APP-EXECUTION-WORKER",
      "Host worker unavailable",
    ),
  );
  await act(async () => signal());
  expect(h.result.current.error?.code).toBe("APP-EXECUTION-WORKER");
  expect(h.result.current.snapshot?.fault).toBeNull();
  expect(h.result.current.current).toBe(false);
  expect(h.result.current.canUnload).toBe(true);
  expect(h.result.current.canReplace).toBe(false);
  vi.mocked(ipc.unloadProgram).mockResolvedValue({
    contractVersion: CONTRACT_VERSION,
    sessionId: "1",
  });
  await act(() => h.result.current.unload());
  expect(h.result.current.snapshot).toBeNull();
  expect(h.result.current.sourceLocked).toBe(false);
});
