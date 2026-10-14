import type { RuntimeSnapshotDto, RuntimeDiagnostic } from "@plc-lab/ipc";
import { CONTRACT_VERSION } from "../../packages/ipc/src/generated";
// Fixed backend response fixtures test presentation, never evaluate a PLC program.
export function readySnapshot(sessionId = "1"): RuntimeSnapshotDto {
  return {
    contractVersion: CONTRACT_VERSION,
    sessionId,
    programName: "Main",
    state: "READY",
    completedScans: "0",
    simulationTimeMicros: "0",
    cyclePeriodMicros: "100000",
    operationBudget: "10000",
    variables: [
      {
        variableId: "1",
        name: "Start",
        typeName: "BOOL",
        role: "Input",
        value: { type: "BOOL", value: false },
      },
      {
        variableId: "2",
        name: "Motor",
        typeName: "BOOL",
        role: "Output",
        value: { type: "BOOL", value: false },
      },
      {
        variableId: "3",
        name: "Internal",
        typeName: "BOOL",
        role: "Local",
        value: { type: "BOOL", value: false },
      },
    ],
    functionBlocks: [],
    fault: null,
  };
}
export const runtimeFault: RuntimeDiagnostic = {
  code: "PLC-RUNTIME-003",
  severity: "error",
  message: "INT division by zero is not allowed.",
  span: { start: { line: 12, column: 10 }, end: { line: 12, column: 28 } },
  category: "runtimeFault",
  relatedVariable: { variableId: "2", name: "Motor" },
};
