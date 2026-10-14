import { listen } from "@tauri-apps/api/event";
import { isTauri } from "@tauri-apps/api/core";
import { CONTRACT_VERSION, EXECUTION_WAKE_EVENT } from "./generated";
import type { ExecutionWake } from "./generated";
import { IpcTransportError } from "./transport";
export type { ExecutionMode, ExecutionRunId, ExecutionWake } from "./generated";

export function parseExecutionWake(value: unknown): ExecutionWake {
  const v = value as Partial<ExecutionWake> | null;
  const identity = (id: unknown) =>
    typeof id === "string" &&
    /^[1-9][0-9]{0,19}$/.test(id) &&
    BigInt(id) <= 18446744073709551615n;
  if (
    !v ||
    typeof v !== "object" ||
    Object.keys(v).sort().join(",") !== "contractVersion,runId,sessionId" ||
    v.contractVersion !== CONTRACT_VERSION ||
    !identity(v.sessionId) ||
    !identity(v.runId)
  )
    throw new IpcTransportError(
      "APP-IPC-002",
      "The desktop returned an incompatible execution wake signal.",
    );
  return v as ExecutionWake;
}
export async function listenExecutionWake(
  onWake: (wake: ExecutionWake) => void,
  onError: (error: unknown) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  return listen<unknown>(EXECUTION_WAKE_EVENT, (event) => {
    try {
      onWake(parseExecutionWake(event.payload));
    } catch (error) {
      onError(error);
    }
  });
}
