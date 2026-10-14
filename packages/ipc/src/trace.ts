import { CONTRACT_VERSION, TRACE_PAGE_COMMAND } from "./generated";
import type {
  TracePageDto,
  TracePageRequest,
  TraceEntryDto,
  TraceEventKind,
} from "./generated";
import { CoreInfoError, invokeNative } from "./transport";
import { parseRuntimeBoundary } from "./runtime";
export type {
  TracePageDto,
  TracePageRequest,
  TraceEntryDto,
  TraceEventKind,
  TraceSequenceId,
} from "./generated";

const kinds: TraceEventKind[] = [
  "SessionLoaded",
  "InputStaged",
  "ProcessAttached",
  "ScanCompleted",
  "ScanFaulted",
  "RuntimeReset",
  "ProcessReset",
  "ProcessDetached",
  "ExecutionStarted",
  "ExecutionPaused",
  "ExecutionResumed",
  "ExecutionStopped",
];
function invalid(): never {
  throw new CoreInfoError(
    "APP-IPC-002",
    "The desktop returned an incompatible trace response.",
  );
}
function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value))
    invalid();
  return value as Record<string, unknown>;
}
function exact(v: Record<string, unknown>, keys: string[]) {
  if (Object.keys(v).sort().join(",") !== keys.sort().join(",")) invalid();
}
function decimal(v: unknown, nonzero = false): string {
  if (
    typeof v !== "string" ||
    v.length > 20 ||
    !/^(0|[1-9][0-9]*)$/.test(v) ||
    BigInt(v) > 18446744073709551615n ||
    (nonzero && v === "0")
  )
    invalid();
  return v;
}
function entry(value: unknown): TraceEntryDto {
  const v = record(value);
  exact(v, [
    "sequence",
    "kind",
    "completedScans",
    "simulationTimeMicros",
    "runtimeState",
    "executionMode",
    "executionRunId",
    "variables",
    "functionBlocks",
    "process",
    "fault",
    "stagedInput",
  ]);
  if (!kinds.includes(v.kind as TraceEventKind)) invalid();
  const boundary = parseRuntimeBoundary({ ...v, state: v.runtimeState });
  let stagedInput: TraceEntryDto["stagedInput"] = null;
  if (v.kind === "InputStaged") {
    const input = record(v.stagedInput);
    exact(input, ["variableId", "name"]);
    const id = decimal(input.variableId, true);
    if (
      !boundary.variables.some(
        (p) =>
          p.variableId === id &&
          p.name === input.name &&
          p.role === "Input" &&
          p.inputAuthority === "USER",
      )
    )
      invalid();
    stagedInput = { variableId: id, name: input.name as string };
  } else if (v.stagedInput !== null) invalid();
  if (
    v.kind === "ScanFaulted" &&
    (boundary.state !== "FAULTED" ||
      boundary.fault?.category !== "runtimeFault")
  )
    invalid();
  const controlMode = (
    {
      ExecutionStarted: "RUNNING",
      ExecutionResumed: "RUNNING",
      ExecutionPaused: "PAUSED",
      ExecutionStopped: "STOPPED",
    } as Record<string, string>
  )[v.kind as string];
  if (
    controlMode &&
    (boundary.state !== "READY" ||
      boundary.executionMode !== controlMode ||
      boundary.executionRunId === null)
  )
    invalid();
  if (
    [
      "SessionLoaded",
      "ProcessAttached",
      "ProcessDetached",
      "ProcessReset",
      "RuntimeReset",
    ].includes(String(v.kind)) &&
    boundary.executionMode !== "STOPPED"
  )
    invalid();
  if (v.kind === "ScanCompleted" && boundary.state !== "READY") invalid();
  if (v.kind === "ProcessAttached" && boundary.process === null) invalid();
  if (v.kind === "ProcessDetached" && boundary.process !== null) invalid();
  const sequence = decimal(v.sequence, true),
    completedScans = decimal(v.completedScans),
    simulationTimeMicros = decimal(v.simulationTimeMicros);
  if (
    v.kind === "SessionLoaded" &&
    (sequence !== "1" ||
      completedScans !== "0" ||
      simulationTimeMicros !== "0" ||
      boundary.state !== "READY" ||
      boundary.executionRunId !== null ||
      boundary.process !== null)
  )
    invalid();
  return {
    sequence,
    kind: v.kind as TraceEventKind,
    completedScans,
    simulationTimeMicros,
    runtimeState: boundary.state,
    executionMode: boundary.executionMode,
    executionRunId: boundary.executionRunId,
    variables: boundary.variables,
    functionBlocks: boundary.functionBlocks,
    process: boundary.process,
    fault: boundary.fault,
    stagedInput,
  };
}
export function parseTracePage(value: unknown): TracePageDto {
  const v = record(value);
  exact(v, [
    "contractVersion",
    "sessionId",
    "entries",
    "capacity",
    "oldestSequence",
    "latestSequence",
    "droppedCount",
    "saturated",
    "gap",
  ]);
  if (
    v.contractVersion !== CONTRACT_VERSION ||
    !Array.isArray(v.entries) ||
    v.entries.length > 256 ||
    typeof v.capacity !== "number" ||
    !Number.isInteger(v.capacity) ||
    v.capacity < 1 ||
    v.capacity > 4096 ||
    typeof v.saturated !== "boolean" ||
    typeof v.gap !== "boolean"
  )
    invalid();
  const oldestSequence =
      v.oldestSequence === null ? null : decimal(v.oldestSequence, true),
    latestSequence =
      v.latestSequence === null ? null : decimal(v.latestSequence, true),
    droppedCount = decimal(v.droppedCount),
    entries = v.entries.map(entry);
  if ((oldestSequence === null) !== (latestSequence === null)) invalid();
  if (oldestSequence === null) {
    if (entries.length || droppedCount !== "0" || v.saturated || v.gap)
      invalid();
  } else {
    if (
      BigInt(oldestSequence) > BigInt(latestSequence!) ||
      BigInt(oldestSequence) !== BigInt(droppedCount) + 1n ||
      BigInt(latestSequence!) - BigInt(oldestSequence) + 1n > BigInt(v.capacity)
    )
      invalid();
    for (let i = 0; i < entries.length; i++) {
      const n = BigInt(entries[i]!.sequence);
      if (
        n < BigInt(oldestSequence) ||
        n > BigInt(latestSequence!) ||
        (i > 0 && n !== BigInt(entries[i - 1]!.sequence) + 1n)
      )
        invalid();
    }
  }
  if (v.saturated && latestSequence !== "18446744073709551615") invalid();
  return {
    contractVersion: CONTRACT_VERSION,
    sessionId: decimal(v.sessionId, true),
    entries,
    capacity: v.capacity,
    oldestSequence,
    latestSequence,
    droppedCount,
    saturated: v.saturated,
    gap: v.gap,
  };
}
export async function getTracePage(
  sessionId: string,
  afterSequence: string | null = null,
  limit = 256,
): Promise<TracePageDto> {
  if (!Number.isInteger(limit) || limit < 1 || limit > 256) invalid();
  const request: TracePageRequest = {
    contractVersion: CONTRACT_VERSION,
    sessionId: decimal(sessionId, true),
    afterSequence: afterSequence === null ? null : decimal(afterSequence, true),
    limit,
  };
  const page = parseTracePage(await invokeNative(TRACE_PAGE_COMMAND, request));
  if (page.sessionId !== request.sessionId || page.entries.length > limit)
    invalid();
  const cursor = afterSequence === null ? 0n : BigInt(afterSequence);
  if (
    afterSequence !== null &&
    (page.latestSequence === null || cursor > BigInt(page.latestSequence))
  )
    invalid();
  const gap =
    afterSequence !== null &&
    page.oldestSequence !== null &&
    cursor < BigInt(page.oldestSequence) - 1n;
  if (page.gap !== gap) invalid();
  if (page.latestSequence !== null && cursor < BigInt(page.latestSequence)) {
    const first =
      cursor + 1n > BigInt(page.oldestSequence!)
        ? cursor + 1n
        : BigInt(page.oldestSequence!);
    if (
      page.entries.length === 0 ||
      BigInt(page.entries[0]!.sequence) !== first
    )
      invalid();
  } else if (page.entries.length) invalid();
  return page;
}
