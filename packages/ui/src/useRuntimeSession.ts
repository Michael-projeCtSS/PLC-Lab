import { useEffect, useRef, useState } from "react";
import { coalescedRefresh } from "./coalesced-refresh";
import {
  loadStProgram,
  startExecution,
  pauseExecution,
  stopExecution,
  getRuntimeSnapshot,
  listenExecutionWake,
  setRuntimeInput,
  singleScan,
  resetRuntime,
  unloadProgram,
  attachProcessScene,
  detachProcessScene,
  resetProcessScene,
  IpcTransportError,
} from "@plc-lab/ipc";
import type {
  RuntimeSnapshotDto,
  RuntimeValue,
  CompilerDiagnostic,
} from "@plc-lab/ipc";

type Action =
  | "run"
  | "pause"
  | "stop"
  | "load"
  | "input"
  | "scan"
  | "reset"
  | "unload"
  | "attach"
  | "detach"
  | "resetProcess";
type View = {
  snapshot: RuntimeSnapshotDto | null;
  current: boolean;
  pending: Action | null;
  error: { code: string; message: string } | null;
  loadDiagnostics: CompilerDiagnostic[];
  observationKey: string;
};
const initial: View = {
  snapshot: null,
  current: false,
  pending: null,
  error: null,
  loadDiagnostics: [],
  observationKey: "0",
};

export function useRuntimeSession(source: string, compiled: boolean) {
  const [view, setView] = useState<View>(initial);
  const snapshot = useRef<RuntimeSnapshotDto | null>(null),
    sourceRevision = useRef(0n),
    loadedRevision = useRef<bigint | null>(null),
    generation = useRef(0n),
    busy = useRef(false),
    starting = useRef(false),
    mounted = useRef(false);
  const refresh = useRef<ReturnType<typeof coalescedRefresh> | null>(null),
    wakeQueued = useRef(false),
    observation = useRef(0n);
  const observed = () => (++observation.current).toString();
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      generation.current += 1n;
    };
  }, []);
  useEffect(() => {
    let active = true,
      unlisten: (() => void) | undefined;
    const failed = (error: unknown) => {
      if (active)
        setView((v) => ({
          ...v,
          error: {
            code:
              error instanceof IpcTransportError
                ? error.code
                : "APP-EXECUTION-REFRESH",
            message:
              error instanceof Error
                ? error.message
                : "Execution refresh failed.",
          },
          current: false,
        }));
    };
    const reader = coalescedRefresh(async () => {
      const before = snapshot.current;
      if (
        !before ||
        busy.current ||
        loadedRevision.current !== sourceRevision.current ||
        before.executionMode !== "RUNNING"
      )
        return;
      const token = generation.current,
        revision = sourceRevision.current;
      try {
        const next = await getRuntimeSnapshot(before.sessionId);
        if (
          !active ||
          generation.current !== token ||
          sourceRevision.current !== revision ||
          busy.current
        )
          return;
        if (
          next.sessionId !== before.sessionId ||
          next.executionRunId !== before.executionRunId
        )
          throw new IpcTransportError(
            "APP-IPC-002",
            "Execution observation belongs to another run.",
          );
        snapshot.current = next;
        setView((v) => ({ ...v, snapshot: next, observationKey: observed() }));
      } catch (error) {
        if (active && generation.current === token) failed(error);
      }
    });
    refresh.current = reader;
    void listenExecutionWake((wake) => {
      const live = snapshot.current;
      if (
        !active ||
        !live ||
        live.executionMode !== "RUNNING" ||
        wake.sessionId !== live.sessionId ||
        wake.runId !== live.executionRunId ||
        loadedRevision.current !== sourceRevision.current
      )
        return;
      if (busy.current) wakeQueued.current = true;
      else reader.trigger();
    }, failed).then((stop) => {
      if (active) unlisten = stop;
      else stop();
    }, failed);
    return () => {
      active = false;
      reader.dispose();
      unlisten?.();
      refresh.current = null;
    };
  }, []);
  const invalidateSource = () => {
    if (
      starting.current ||
      (snapshot.current?.executionMode !== undefined &&
        snapshot.current.executionMode !== "STOPPED")
    )
      return false;
    sourceRevision.current += 1n;
    generation.current += 1n;
    busy.current = false;
    loadedRevision.current = null;
    setView((v) => ({
      ...v,
      current: false,
      pending: null,
      error: null,
      loadDiagnostics: [],
    }));
    return true;
  };
  const perform = async (
    action: Action,
    operation: (id: string) => Promise<RuntimeSnapshotDto | null>,
  ) => {
    if (busy.current || (action !== "load" && !snapshot.current)) return;
    busy.current = true;
    starting.current = action === "run";
    const token = ++generation.current,
      revision = sourceRevision.current,
      expected = snapshot.current?.sessionId ?? "";
    const current = () =>
      mounted.current &&
      generation.current === token &&
      sourceRevision.current === revision;
    setView((v) => ({
      ...v,
      pending: action,
      error: null,
      loadDiagnostics: [],
      current: action === "load" ? false : v.current,
    }));
    try {
      const next = await operation(expected);
      if (!current()) return;
      if (next && action !== "load" && next.sessionId !== expected)
        throw new IpcTransportError(
          "APP-IPC-002",
          "The desktop returned a different runtime session.",
        );
      snapshot.current = next;
      if (action === "load") loadedRevision.current = next ? revision : null;
      if (action === "unload") loadedRevision.current = null;
      setView((v) => ({
        ...v,
        snapshot: next,
        current: next !== null && loadedRevision.current === revision,
        pending: null,
        observationKey: observed(),
      }));
    } catch (error: unknown) {
      if (current()) {
        if (error instanceof LoadDiagnosticError) {
          loadedRevision.current = null;
          setView((v) => ({
            ...v,
            pending: null,
            current: false,
            error: null,
            loadDiagnostics: error.diagnostics,
          }));
          return;
        }
        const code =
          error instanceof IpcTransportError ? error.code : "APP-INTERNAL-001";
        // Only a documented input validation rejection proves no mutation.
        // Transport/internal failures leave the observation uncertain.
        const invalid =
          action !== "input" ||
          !["APP-RUNTIME-INPUT", "PLC-PROCESS-004"].includes(code);
        if (invalid) loadedRevision.current = null;
        setView((v) => ({
          ...v,
          pending: null,
          current: invalid ? false : v.current,
          error: {
            code,
            message:
              error instanceof Error
                ? error.message
                : "Runtime request failed.",
          },
        }));
      }
    } finally {
      if (current()) {
        busy.current = false;
        starting.current = false;
        if (wakeQueued.current || action === "run") {
          wakeQueued.current = false;
          // A terminal tick can precede delivery of the Run/Resume response.
          refresh.current?.trigger();
        }
      }
    }
  };
  const load = () => {
    if (
      !compiled ||
      (snapshot.current && snapshot.current.executionMode !== "STOPPED")
    )
      return Promise.resolve();
    const candidate = source;
    return perform("load", async () => {
      const response = await loadStProgram(candidate);
      if (response.kind === "compileFailed") {
        // The operation token prevents late failures from replacing newer diagnostics.
        throw new LoadDiagnosticError(response.diagnostics);
      }
      return response.snapshot;
    });
  };
  const canMutate =
    view.current &&
    compiled &&
    view.snapshot?.state === "READY" &&
    view.pending === null;
  const stopped = !view.snapshot || view.snapshot.executionMode === "STOPPED";
  const failedClosed =
    !view.current &&
    [
      "APP-EXECUTION-WORKER",
      "APP-EXECUTION-NOTIFICATION",
      "APP-PROCESS-INTERNAL",
      "APP-RUNTIME-INTERNAL",
    ].includes(view.error?.code ?? "");
  const canManageProcess =
    view.current &&
    compiled &&
    stopped &&
    view.snapshot !== null &&
    view.pending === null;
  return {
    ...view,
    invalidateSource,
    canScan: canMutate && stopped,
    canRun: canMutate && view.snapshot?.executionMode !== "RUNNING",
    canPause: canMutate && view.snapshot?.executionMode === "RUNNING",
    canStop: canMutate && !stopped,
    sourceLocked: !stopped || view.pending === "run",
    canReplace: stopped && view.pending === null,
    canUnload: (stopped || failedClosed) && view.pending === null,
    run: () =>
      canMutate && view.snapshot?.executionMode !== "RUNNING"
        ? perform("run", (id) => startExecution(id))
        : Promise.resolve(),
    pause: () =>
      canMutate && !stopped
        ? perform("pause", (id) => pauseExecution(id))
        : Promise.resolve(),
    stop: () =>
      canMutate && !stopped
        ? perform("stop", (id) => stopExecution(id))
        : Promise.resolve(),
    canEditInputs: canMutate,
    canManageProcess,
    load,
    scan: () =>
      canMutate && stopped
        ? perform("scan", (id) => singleScan(id))
        : Promise.resolve(),
    setInput: (id: string, value: RuntimeValue) =>
      canMutate
        ? perform("input", (session) => setRuntimeInput(session, id, value))
        : Promise.resolve(),
    reset: () =>
      stopped ? perform("reset", (id) => resetRuntime(id)) : Promise.resolve(),
    unload: () =>
      stopped || failedClosed
        ? perform("unload", async (id) => {
            const response = await unloadProgram(id);
            if (response.sessionId !== id)
              throw new IpcTransportError(
                "APP-IPC-002",
                "Unload targeted another session.",
              );
            return null;
          })
        : Promise.resolve(),
    attach: () =>
      canMutate && stopped
        ? perform("attach", (id) => attachProcessScene(id, "TANK_FILLING"))
        : Promise.resolve(),
    detach: () =>
      canManageProcess
        ? perform("detach", (id) => detachProcessScene(id))
        : Promise.resolve(),
    resetProcess: () =>
      canManageProcess
        ? perform("resetProcess", (id) => resetProcessScene(id))
        : Promise.resolve(),
  };
}
class LoadDiagnosticError extends Error {
  constructor(public diagnostics: CompilerDiagnostic[]) {
    super("Load failed because Rust rejected the source.");
  }
}
