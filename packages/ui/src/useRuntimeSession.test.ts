import { act, renderHook } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import {
  loadStProgram,
  setRuntimeInput,
  singleScan,
  resetRuntime,
  unloadProgram,
  IpcTransportError,
} from "@plc-lab/ipc";
import { useRuntimeSession } from "./useRuntimeSession";
import { readySnapshot } from "../../../tests/frontend/runtime-fixtures";
import type { RuntimeSnapshotDto, LoadProgramResponse } from "@plc-lab/ipc";
import { CONTRACT_VERSION } from "../../ipc/src/generated";
vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  loadStProgram: vi.fn(),
  setRuntimeInput: vi.fn(),
  singleScan: vi.fn(),
  resetRuntime: vi.fn(),
  unloadProgram: vi.fn(),
}));
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(loadStProgram).mockResolvedValue({
    kind: "loaded",
    snapshot: readySnapshot(),
  });
});
const mount = () => renderHook(() => useRuntimeSession("source", true));
it("requires explicit successful load before scan and blocks duplicate mutations", async () => {
  const h = mount();
  expect(h.result.current.canScan).toBe(false);
  await act(() => h.result.current.load());
  expect(h.result.current.canScan).toBe(true);
  let complete!: (v: RuntimeSnapshotDto) => void;
  vi.mocked(singleScan).mockReturnValue(
    new Promise((resolve) => {
      complete = resolve;
    }),
  );
  let pending!: Promise<void>;
  act(() => {
    pending = h.result.current.scan();
    void h.result.current.scan();
  });
  expect(singleScan).toHaveBeenCalledTimes(1);
  expect(h.result.current.canScan).toBe(false);
  await act(async () => {
    complete({
      ...readySnapshot(),
      completedScans: "7",
      simulationTimeMicros: "123",
    });
    await pending;
  });
  expect(h.result.current.snapshot?.completedScans).toBe("7");
});
it("late load after source edit cannot associate executable source", async () => {
  let complete!: (v: LoadProgramResponse) => void;
  vi.mocked(loadStProgram).mockReturnValue(
    new Promise((resolve) => {
      complete = resolve;
    }),
  );
  const h = mount();
  let pending!: Promise<void>;
  act(() => {
    pending = h.result.current.load();
  });
  act(() => h.result.current.invalidateSource());
  await act(async () => {
    complete({ kind: "loaded", snapshot: readySnapshot() });
    await pending;
  });
  expect(h.result.current.snapshot).toBeNull();
  expect(h.result.current.canScan).toBe(false);
});
it("obsolete stale-load rejection cannot invalidate a newer loaded source", async () => {
  let rejectOld!: (error: Error) => void;
  vi.mocked(loadStProgram).mockReturnValueOnce(
    new Promise((_, reject) => {
      rejectOld = reject;
    }),
  );
  const h = renderHook(({ source }) => useRuntimeSession(source, true), {
    initialProps: { source: "A" },
  });
  let old!: Promise<void>;
  act(() => {
    old = h.result.current.load();
  });
  act(() => h.result.current.invalidateSource());
  h.rerender({ source: "B" });
  vi.mocked(loadStProgram).mockResolvedValue({
    kind: "loaded",
    snapshot: readySnapshot("2"),
  });
  await act(() => h.result.current.load());
  await act(async () => {
    rejectOld(new IpcTransportError("APP-RUNTIME-STALE-LOAD", "Superseded."));
    await old;
  });
  expect(loadStProgram).toHaveBeenNthCalledWith(1, "A");
  expect(loadStProgram).toHaveBeenNthCalledWith(2, "B");
  expect(loadStProgram).toHaveBeenCalledTimes(2);
  expect(h.result.current.snapshot).toEqual(readySnapshot("2"));
  expect(h.result.current.error).toBeNull();
  expect(h.result.current.pending).toBeNull();
  expect(h.result.current.canScan).toBe(true);
});
it("authoritative stale-load rejection fails closed until an explicit reload", async () => {
  const h = mount();
  await act(() => h.result.current.load());
  vi.mocked(loadStProgram).mockRejectedValueOnce(
    new IpcTransportError("APP-RUNTIME-STALE-LOAD", "Superseded. Reload."),
  );
  await act(() => h.result.current.load());
  expect(h.result.current.snapshot).toEqual(readySnapshot());
  expect(h.result.current.error?.code).toBe("APP-RUNTIME-STALE-LOAD");
  expect(h.result.current.canScan).toBe(false);
  expect(h.result.current.canEditInputs).toBe(false);
  expect(loadStProgram).toHaveBeenCalledTimes(2);
  await act(() => h.result.current.scan());
  expect(singleScan).not.toHaveBeenCalled();
  await act(() => h.result.current.load());
  expect(loadStProgram).toHaveBeenCalledTimes(3);
  expect(h.result.current.canScan).toBe(true);
  expect(h.result.current.error).toBeNull();
});
it("late scan cannot overwrite source edit and replacement", async () => {
  const h = mount();
  await act(() => h.result.current.load());
  let complete!: (v: RuntimeSnapshotDto) => void;
  vi.mocked(singleScan).mockReturnValue(
    new Promise((resolve) => {
      complete = resolve;
    }),
  );
  let pending!: Promise<void>;
  act(() => {
    pending = h.result.current.scan();
  });
  act(() => h.result.current.invalidateSource());
  vi.mocked(loadStProgram).mockResolvedValue({
    kind: "loaded",
    snapshot: readySnapshot("2"),
  });
  await act(() => h.result.current.load());
  await act(async () => {
    complete({ ...readySnapshot(), completedScans: "9" });
    await pending;
  });
  expect(h.result.current.snapshot?.sessionId).toBe("2");
  expect(h.result.current.snapshot?.completedScans).toBe("0");
});
it("failed replacement retains last snapshot and compiler diagnostics as data", async () => {
  const h = mount();
  await act(() => h.result.current.load());
  const diagnostics = [
    {
      code: "PLC-ST-002",
      severity: "error" as const,
      message: "Expected PROGRAM.",
      span: null,
    },
  ];
  vi.mocked(loadStProgram).mockResolvedValue({
    kind: "compileFailed",
    diagnostics,
  });
  await act(() => h.result.current.load());
  expect(h.result.current.snapshot).toEqual(readySnapshot());
  expect(h.result.current.loadDiagnostics).toEqual(diagnostics);
  expect(h.result.current.error).toBeNull();
  expect(h.result.current.canScan).toBe(false);
});
it("rejected input keeps last authoritative values and executable association", async () => {
  const h = mount();
  await act(() => h.result.current.load());
  vi.mocked(setRuntimeInput).mockRejectedValue(
    new IpcTransportError("APP-RUNTIME-INPUT", "Wrong type."),
  );
  await act(() =>
    h.result.current.setInput("1", { type: "BOOL", value: true }),
  );
  expect(h.result.current.snapshot).toEqual(readySnapshot());
  expect(h.result.current.error?.code).toBe("APP-RUNTIME-INPUT");
  expect(h.result.current.canScan).toBe(true);
});
it("stale backend generation invalidates presentation association", async () => {
  const h = mount();
  await act(() => h.result.current.load());
  vi.mocked(singleScan).mockRejectedValue(
    new IpcTransportError("APP-RUNTIME-STALE-SESSION", "Reload."),
  );
  await act(() => h.result.current.scan());
  expect(h.result.current.canScan).toBe(false);
  expect(h.result.current.snapshot?.sessionId).toBe("1");
});
it("uncertain input transport failure disables execution without predicting state", async () => {
  const h = mount();
  await act(() => h.result.current.load());
  vi.mocked(setRuntimeInput).mockRejectedValue(
    new IpcTransportError("APP-IPC-004", "Connection failed."),
  );
  await act(() =>
    h.result.current.setInput("1", { type: "BOOL", value: true }),
  );
  expect(h.result.current.snapshot).toEqual(readySnapshot());
  expect(h.result.current.canScan).toBe(false);
});
it("wrong-generation response fails closed", async () => {
  const h = mount();
  await act(() => h.result.current.load());
  vi.mocked(singleScan).mockResolvedValue(readySnapshot("2"));
  await act(() => h.result.current.scan());
  expect(h.result.current.snapshot?.sessionId).toBe("1");
  expect(h.result.current.error?.code).toBe("APP-IPC-002");
  expect(h.result.current.canScan).toBe(false);
});
it("reset and unload use the current backend generation", async () => {
  const h = mount();
  await act(() => h.result.current.load());
  vi.mocked(resetRuntime).mockResolvedValue(readySnapshot());
  await act(() => h.result.current.reset());
  expect(resetRuntime).toHaveBeenCalledWith("1");
  vi.mocked(unloadProgram).mockResolvedValue({
    contractVersion: CONTRACT_VERSION,
    sessionId: "1",
  });
  await act(() => h.result.current.unload());
  expect(h.result.current.snapshot).toBeNull();
  expect(h.result.current.canScan).toBe(false);
});
it("late unmounted load is discarded", async () => {
  let complete!: (v: LoadProgramResponse) => void;
  vi.mocked(loadStProgram).mockReturnValue(
    new Promise((resolve) => {
      complete = resolve;
    }),
  );
  const h = mount();
  let pending!: Promise<void>;
  act(() => {
    pending = h.result.current.load();
  });
  h.unmount();
  complete({ kind: "loaded", snapshot: readySnapshot() });
  await pending;
});
