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
        inputAuthority: "USER",
        value: { type: "BOOL", value: false },
      },
      {
        variableId: "2",
        name: "Motor",
        typeName: "BOOL",
        role: "Output",
        inputAuthority: null,
        value: { type: "BOOL", value: false },
      },
      {
        variableId: "3",
        name: "Internal",
        typeName: "BOOL",
        role: "Local",
        inputAuthority: null,
        value: { type: "BOOL", value: false },
      },
    ],
    functionBlocks: [],
    fault: null,
    process: null,
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

export function tankSnapshot(): RuntimeSnapshotDto {
  return {
    ...readySnapshot(),
    completedScans: "17",
    simulationTimeMicros: "123456",
    variables: [
      {
        variableId: "1",
        name: "LevelLow",
        typeName: "BOOL",
        role: "Input",
        inputAuthority: "PROCESS",
        value: { type: "BOOL", value: false },
      },
      {
        variableId: "2",
        name: "LevelHigh",
        typeName: "BOOL",
        role: "Input",
        inputAuthority: "PROCESS",
        value: { type: "BOOL", value: false },
      },
      {
        variableId: "3",
        name: "LevelPercent",
        typeName: "REAL",
        role: "Input",
        inputAuthority: "PROCESS",
        value: { type: "REAL", value: 30 },
      },
      {
        variableId: "4",
        name: "InletValve",
        typeName: "BOOL",
        role: "Output",
        inputAuthority: null,
        value: { type: "BOOL", value: true },
      },
      {
        variableId: "5",
        name: "DrainPump",
        typeName: "BOOL",
        role: "Output",
        inputAuthority: null,
        value: { type: "BOOL", value: false },
      },
      {
        variableId: "6",
        name: "User",
        typeName: "BOOL",
        role: "Input",
        inputAuthority: "USER",
        value: { type: "BOOL", value: false },
      },
    ],
    process: {
      kind: "TANK_FILLING",
      levelPermille: 300,
      lowLevel: false,
      highLevel: false,
      inletValveOpen: true,
      drainPumpRunning: false,
      lowThresholdPermille: 200,
      highThresholdPermille: 800,
      fillRatePermillePerSecond: 1000,
      drainRatePermillePerSecond: 800,
    },
  };
}
