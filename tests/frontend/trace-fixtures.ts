import type {
  TraceEntryDto,
  TraceEventKind,
  TracePageDto,
  RuntimeSnapshotDto,
} from "@plc-lab/ipc";
import { CONTRACT_VERSION } from "../../packages/ipc/src/generated";
import { readySnapshot } from "./runtime-fixtures";
// Fixed transport observations only; no PLC/process execution occurs in fixtures.
export function traceEntry(
  sequence = "1",
  kind: TraceEventKind = "SessionLoaded",
  snapshot: RuntimeSnapshotDto = readySnapshot(),
): TraceEntryDto {
  return {
    sequence,
    kind,
    completedScans: snapshot.completedScans,
    simulationTimeMicros: snapshot.simulationTimeMicros,
    runtimeState: snapshot.state,
    executionMode: snapshot.executionMode,
    executionRunId: snapshot.executionRunId,
    variables: snapshot.variables,
    functionBlocks: snapshot.functionBlocks,
    process: snapshot.process,
    fault: snapshot.fault,
    stagedInput: null,
  };
}
export function tracePage(
  entries: TraceEntryDto[] = [traceEntry()],
  sessionId = "1",
): TracePageDto {
  return {
    contractVersion: CONTRACT_VERSION,
    sessionId,
    entries,
    capacity: 4096,
    oldestSequence: "1",
    latestSequence: entries.at(-1)?.sequence ?? "1",
    droppedCount: "0",
    saturated: false,
    gap: false,
  };
}
