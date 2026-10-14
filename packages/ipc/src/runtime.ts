import {
  CONTRACT_VERSION,
  LOAD_PROGRAM_COMMAND,
  SET_INPUT_COMMAND,
  SINGLE_SCAN_COMMAND,
  RESET_RUNTIME_COMMAND,
  UNLOAD_PROGRAM_COMMAND,
  RUNTIME_SNAPSHOT_COMMAND,
  ATTACH_PROCESS_COMMAND,
  DETACH_PROCESS_COMMAND,
  RESET_PROCESS_COMMAND,
} from "./generated";
import type {
  RuntimeValue,
  RuntimeSnapshotDto,
  RuntimeVariable,
  RuntimeFunctionBlock,
  RuntimeDiagnostic,
  LoadProgramResponse,
  UnloadProgramResponse,
  LoadStProgramRequest,
  RuntimeSessionRequest,
  SetRuntimeInputRequest,
  ProcessSnapshotDto,
  AttachProcessSceneRequest,
  ProcessSceneKind,
} from "./generated";
import { CoreInfoError, invokeNative } from "./transport";
import {
  parseCompileResponse,
  parseCompilerSpan,
} from "./compilation-response";
export type {
  RuntimeValue,
  RuntimeSnapshotDto,
  RuntimeVariable,
  RuntimeFunctionBlock,
  RuntimeDiagnostic,
  LoadProgramResponse,
  ProcessSnapshotDto,
  TankProcessSnapshotDto,
  ProcessSceneKind,
  InputAuthority,
} from "./generated";

function invalid(): never {
  throw new CoreInfoError(
    "APP-IPC-002",
    "The desktop returned an incompatible runtime response.",
  );
}
function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value))
    invalid();
  return value as Record<string, unknown>;
}
function exact(value: Record<string, unknown>, fields: string[]) {
  if (Object.keys(value).sort().join(",") !== fields.sort().join(","))
    invalid();
}
function text(value: unknown): string {
  if (typeof value !== "string" || !value.trim()) invalid();
  return value;
}
function decimal(
  value: unknown,
  maximum = 18446744073709551615n,
  nonzero = false,
): string {
  if (
    typeof value !== "string" ||
    value.length > 20 ||
    !/^(0|[1-9][0-9]*)$/.test(value) ||
    BigInt(value) > maximum ||
    (nonzero && value === "0")
  )
    invalid();
  return value;
}
export function parseRuntimeValue(value: unknown): RuntimeValue {
  const v = record(value);
  if (v.type === "TIME") {
    exact(v, ["type", "microseconds"]);
    return { type: "TIME", microseconds: decimal(v.microseconds) };
  }
  exact(v, ["type", "value"]);
  if (v.type === "BOOL" && typeof v.value === "boolean")
    return { type: "BOOL", value: v.value };
  if (
    v.type === "INT" &&
    typeof v.value === "number" &&
    Number.isInteger(v.value) &&
    v.value >= -32768 &&
    v.value <= 32767
  )
    return { type: "INT", value: v.value };
  // Representation validation only; Rust alone quantizes/executes PLC REAL.
  if (
    v.type === "REAL" &&
    typeof v.value === "number" &&
    Number.isFinite(v.value) &&
    Number.isFinite(Math.fround(v.value)) &&
    !Object.is(v.value, -0)
  )
    return { type: "REAL", value: v.value };
  return invalid();
}
function parseVariable(value: unknown): RuntimeVariable {
  const v = record(value);
  exact(v, [
    "variableId",
    "name",
    "typeName",
    "role",
    "value",
    "inputAuthority",
  ]);
  const parsed = parseRuntimeValue(v.value);
  if (
    v.typeName !== parsed.type ||
    !["Input", "Output", "Local"].includes(String(v.role))
  )
    invalid();
  if (
    v.role === "Input"
      ? !["USER", "PROCESS"].includes(String(v.inputAuthority))
      : v.inputAuthority !== null
  )
    invalid();
  return {
    variableId: decimal(v.variableId, 4294967295n, true),
    name: text(v.name),
    typeName: parsed.type,
    role: v.role as RuntimeVariable["role"],
    inputAuthority: v.inputAuthority as RuntimeVariable["inputAuthority"],
    value: parsed,
  };
}
function parseBlock(value: unknown): RuntimeFunctionBlock {
  const v = record(value);
  exact(v, ["kind", "instanceId", "name", "q", "etMicros"]);
  if (v.kind !== "TON" || typeof v.q !== "boolean") invalid();
  return {
    kind: "TON",
    instanceId: decimal(v.instanceId, 4294967295n, true),
    name: text(v.name),
    q: v.q,
    etMicros: decimal(v.etMicros),
  };
}
function parseFault(value: unknown): RuntimeDiagnostic | null {
  if (value === null) return null;
  const v = record(value);
  exact(v, [
    "code",
    "severity",
    "message",
    "span",
    "category",
    "relatedVariable",
  ]);
  if (
    typeof v.code !== "string" ||
    !/^[A-Z0-9]+(?:-[A-Z0-9]+)+$/.test(v.code) ||
    v.severity !== "error" ||
    !["runtimeFault", "internalApplicationError"].includes(String(v.category))
  )
    invalid();
  if (
    (v.category === "runtimeFault" && !v.code.startsWith("PLC-RUNTIME-")) ||
    (v.category === "internalApplicationError" && !v.code.startsWith("APP-"))
  )
    invalid();
  let related: RuntimeDiagnostic["relatedVariable"] = null;
  if (v.relatedVariable !== null) {
    const r = record(v.relatedVariable);
    exact(r, ["variableId", "name"]);
    related = {
      variableId: decimal(r.variableId, 4294967295n, true),
      name: text(r.name),
    };
  }
  return {
    code: v.code,
    severity: "error",
    message: text(v.message),
    span: parseCompilerSpan(v.span),
    category: v.category as RuntimeDiagnostic["category"],
    relatedVariable: related,
  };
}
export function parseRuntimeBoundary(v: Record<string, unknown>) {
  if (
    !["READY", "FAULTED"].includes(String(v.state)) ||
    !Array.isArray(v.variables) ||
    !Array.isArray(v.functionBlocks)
  )
    invalid();
  const variables = v.variables.map(parseVariable),
    functionBlocks = v.functionBlocks.map(parseBlock),
    fault = parseFault(v.fault);
  const process = parseProcessSnapshot(v.process);
  const owned = variables.filter((v) => v.inputAuthority === "PROCESS");
  if (process === null && owned.length !== 0) invalid();
  if (process !== null) {
    const expected = new Map([
      ["LEVELLOW", "BOOL"],
      ["LEVELHIGH", "BOOL"],
      ["LEVELPERCENT", "REAL"],
    ]);
    if (
      owned.length !== 3 ||
      new Set(owned.map((v) => v.name.toUpperCase())).size !== 3 ||
      owned.some((v) => v.typeName !== expected.get(v.name.toUpperCase()))
    )
      invalid();
    for (const name of ["INLETVALVE", "DRAINPUMP"]) {
      if (
        !variables.some(
          (v) =>
            v.name.toUpperCase() === name &&
            v.role === "Output" &&
            v.typeName === "BOOL",
        )
      )
        invalid();
    }
  }
  if (
    (v.state === "READY" && fault !== null) ||
    (v.state === "FAULTED" && fault === null)
  )
    invalid();
  if (
    new Set(variables.map((v) => v.variableId)).size !== variables.length ||
    new Set(functionBlocks.map((b) => b.instanceId)).size !==
      functionBlocks.length
  )
    invalid();
  if (
    fault?.relatedVariable &&
    !variables.some(
      (v) =>
        v.variableId === fault.relatedVariable?.variableId &&
        v.name === fault.relatedVariable.name,
    )
  )
    invalid();
  return {
    state: v.state as RuntimeSnapshotDto["state"],
    variables,
    functionBlocks,
    fault,
    process,
  };
}
export function parseRuntimeSnapshot(value: unknown): RuntimeSnapshotDto {
  const v = record(value);
  exact(v, [
    "contractVersion",
    "sessionId",
    "programName",
    "state",
    "completedScans",
    "simulationTimeMicros",
    "cyclePeriodMicros",
    "operationBudget",
    "variables",
    "functionBlocks",
    "fault",
    "process",
  ]);
  if (v.contractVersion !== CONTRACT_VERSION) invalid();
  const { state, variables, functionBlocks, fault, process } =
    parseRuntimeBoundary(v);
  return {
    contractVersion: CONTRACT_VERSION,
    sessionId: decimal(v.sessionId, 18446744073709551615n, true),
    programName: text(v.programName),
    state,
    completedScans: decimal(v.completedScans),
    simulationTimeMicros: decimal(v.simulationTimeMicros),
    cyclePeriodMicros: decimal(
      v.cyclePeriodMicros,
      18446744073709551615n,
      true,
    ),
    operationBudget: decimal(v.operationBudget, 18446744073709551615n, true),
    variables,
    functionBlocks,
    fault,
    process,
  };
}
function integer(value: unknown, maximum: number, minimum = 0): number {
  if (
    typeof value !== "number" ||
    !Number.isInteger(value) ||
    value < minimum ||
    value > maximum
  )
    invalid();
  return value;
}
export function parseProcessSnapshot(
  value: unknown,
): ProcessSnapshotDto | null {
  if (value === null) return null;
  const v = record(value);
  exact(v, [
    "kind",
    "levelPermille",
    "lowLevel",
    "highLevel",
    "inletValveOpen",
    "drainPumpRunning",
    "lowThresholdPermille",
    "highThresholdPermille",
    "fillRatePermillePerSecond",
    "drainRatePermillePerSecond",
  ]);
  if (
    v.kind !== "TANK_FILLING" ||
    [v.lowLevel, v.highLevel, v.inletValveOpen, v.drainPumpRunning].some(
      (value) => typeof value !== "boolean",
    )
  )
    invalid();
  const low = integer(v.lowThresholdPermille, 1000),
    high = integer(v.highThresholdPermille, 1000);
  if (low > high) invalid();
  return {
    kind: "TANK_FILLING",
    levelPermille: integer(v.levelPermille, 1000),
    lowLevel: v.lowLevel as boolean,
    highLevel: v.highLevel as boolean,
    inletValveOpen: v.inletValveOpen as boolean,
    drainPumpRunning: v.drainPumpRunning as boolean,
    lowThresholdPermille: low,
    highThresholdPermille: high,
    fillRatePermillePerSecond: integer(v.fillRatePermillePerSecond, 65535, 1),
    drainRatePermillePerSecond: integer(v.drainRatePermillePerSecond, 65535, 1),
  };
}
export function parseLoadResponse(value: unknown): LoadProgramResponse {
  const v = record(value);
  if (v.kind === "loaded") {
    exact(v, ["kind", "snapshot"]);
    return { kind: "loaded", snapshot: parseRuntimeSnapshot(v.snapshot) };
  }
  if (v.kind === "compileFailed") {
    exact(v, ["kind", "diagnostics"]);
    const result = parseCompileResponse({
      success: false,
      diagnostics: v.diagnostics,
    });
    return { kind: "compileFailed", diagnostics: result.diagnostics };
  }
  return invalid();
}
export function parseUnloadResponse(value: unknown): UnloadProgramResponse {
  const v = record(value);
  exact(v, ["contractVersion", "sessionId"]);
  if (v.contractVersion !== CONTRACT_VERSION) invalid();
  return {
    contractVersion: CONTRACT_VERSION,
    sessionId: decimal(v.sessionId, 18446744073709551615n, true),
  };
}
export async function loadStProgram(
  source: string,
): Promise<LoadProgramResponse> {
  const request: LoadStProgramRequest = {
    contractVersion: CONTRACT_VERSION,
    source,
  };
  return parseLoadResponse(await invokeNative(LOAD_PROGRAM_COMMAND, request));
}
const target = (sessionId: string): RuntimeSessionRequest => ({
  contractVersion: CONTRACT_VERSION,
  sessionId: decimal(sessionId, 18446744073709551615n, true),
});
export async function setRuntimeInput(
  sessionId: string,
  variableId: string,
  value: RuntimeValue,
): Promise<RuntimeSnapshotDto> {
  const request: SetRuntimeInputRequest = {
    ...target(sessionId),
    variableId: decimal(variableId, 4294967295n, true),
    value: parseRuntimeValue(value),
  };
  return parseRuntimeSnapshot(await invokeNative(SET_INPUT_COMMAND, request));
}
export async function singleScan(sessionId: string) {
  return parseRuntimeSnapshot(
    await invokeNative(SINGLE_SCAN_COMMAND, target(sessionId)),
  );
}
export async function resetRuntime(sessionId: string) {
  return parseRuntimeSnapshot(
    await invokeNative(RESET_RUNTIME_COMMAND, target(sessionId)),
  );
}
export async function getRuntimeSnapshot(sessionId: string) {
  return parseRuntimeSnapshot(
    await invokeNative(RUNTIME_SNAPSHOT_COMMAND, target(sessionId)),
  );
}
export async function unloadProgram(sessionId: string) {
  return parseUnloadResponse(
    await invokeNative(UNLOAD_PROGRAM_COMMAND, target(sessionId)),
  );
}
export async function attachProcessScene(
  sessionId: string,
  sceneKind: ProcessSceneKind,
) {
  if (sceneKind !== "TANK_FILLING") invalid();
  const request: AttachProcessSceneRequest = {
    ...target(sessionId),
    sceneKind,
  };
  return parseRuntimeSnapshot(
    await invokeNative(ATTACH_PROCESS_COMMAND, request),
  );
}
export async function detachProcessScene(sessionId: string) {
  return parseRuntimeSnapshot(
    await invokeNative(DETACH_PROCESS_COMMAND, target(sessionId)),
  );
}
export async function resetProcessScene(sessionId: string) {
  return parseRuntimeSnapshot(
    await invokeNative(RESET_PROCESS_COMMAND, target(sessionId)),
  );
}
