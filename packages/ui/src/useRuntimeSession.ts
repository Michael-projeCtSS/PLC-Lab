import { useEffect, useRef, useState } from "react";
import {
  loadStProgram,
  setRuntimeInput,
  singleScan,
  resetRuntime,
  unloadProgram,
  IpcTransportError,
} from "@plc-lab/ipc";
import type {
  RuntimeSnapshotDto,
  RuntimeValue,
  CompilerDiagnostic,
} from "@plc-lab/ipc";

type Action = "load" | "input" | "scan" | "reset" | "unload";
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
    mounted = useRef(false);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      generation.current += 1n;
    };
  }, []);
  const invalidateSource = () => {
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
  };
  const perform = async (
    action: Action,
    operation: (id: string) => Promise<RuntimeSnapshotDto | null>,
  ) => {
    if (busy.current || (action !== "load" && !snapshot.current)) return;
    busy.current = true;
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
        observationKey: token.toString(),
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
        const invalid = action !== "input" || code !== "APP-RUNTIME-INPUT";
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
      if (current()) busy.current = false;
    }
  };
  const load = () => {
    if (!compiled) return Promise.resolve();
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
  return {
    ...view,
    invalidateSource,
    canScan: canMutate,
    canEditInputs: canMutate,
    load,
    scan: () =>
      canMutate ? perform("scan", (id) => singleScan(id)) : Promise.resolve(),
    setInput: (id: string, value: RuntimeValue) =>
      canMutate
        ? perform("input", (session) => setRuntimeInput(session, id, value))
        : Promise.resolve(),
    reset: () => perform("reset", (id) => resetRuntime(id)),
    unload: () =>
      perform("unload", async (id) => {
        const response = await unloadProgram(id);
        if (response.sessionId !== id)
          throw new IpcTransportError(
            "APP-IPC-002",
            "Unload targeted another session.",
          );
        return null;
      }),
  };
}
class LoadDiagnosticError extends Error {
  constructor(public diagnostics: CompilerDiagnostic[]) {
    super("Load failed because Rust rejected the source.");
  }
}
