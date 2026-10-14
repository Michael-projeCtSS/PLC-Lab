import { CoreInfoError } from "./transport";
export { CoreInfoError, IpcTransportError } from "./transport";
import { parseCompileResponse } from "./compilation-response";
export { parseCompileResponse } from "./compilation-response";
export * from "./runtime";
import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  CONTRACT_VERSION,
  CORE_INFO_COMMAND,
  COMPILE_ST_COMMAND,
} from "./generated";
import type { CoreInfo, CoreInfoRequest } from "./generated";
import type { CompileResponse, CompileStRequest } from "./generated";

export type { CoreInfo, IpcError } from "./generated";
export type {
  CompileResponse,
  CompilerDiagnostic,
  CompilerSpan,
  CompilerSymbol,
} from "./generated";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function parseCoreInfo(value: unknown): CoreInfo {
  if (
    !isRecord(value) ||
    typeof value.applicationName !== "string" ||
    value.applicationName.length === 0 ||
    typeof value.appVersion !== "string" ||
    value.appVersion.length === 0 ||
    typeof value.coreVersion !== "string" ||
    value.coreVersion.length === 0 ||
    value.contractVersion !== CONTRACT_VERSION ||
    value.execution !== "manual-single-scan" ||
    (value.buildContext !== "development" && value.buildContext !== "release")
  ) {
    throw new CoreInfoError(
      "APP-IPC-002",
      "The desktop returned an incompatible core information response.",
    );
  }
  return {
    applicationName: value.applicationName,
    appVersion: value.appVersion,
    coreVersion: value.coreVersion,
    contractVersion: value.contractVersion,
    execution: value.execution,
    buildContext: value.buildContext,
  };
}

export async function getCoreInfo(): Promise<CoreInfo> {
  if (!isTauri()) {
    throw new CoreInfoError(
      "APP-IPC-003",
      "Native core unavailable. Open PLC Lab as a desktop application.",
    );
  }
  const request: CoreInfoRequest = { contractVersion: CONTRACT_VERSION };
  let value: unknown;
  try {
    value = await invoke<unknown>(CORE_INFO_COMMAND, { request });
  } catch (error: unknown) {
    if (
      isRecord(error) &&
      typeof error.code === "string" &&
      typeof error.message === "string"
    ) {
      throw new CoreInfoError(error.code, error.message);
    }
    throw new CoreInfoError(
      "APP-IPC-004",
      error instanceof Error
        ? error.message
        : "The native core information request failed.",
    );
  }
  return parseCoreInfo(value);
}

export async function compileSt(source: string): Promise<CompileResponse> {
  if (!isTauri())
    throw new CoreInfoError(
      "APP-IPC-003",
      "Compilation requires the native PLC Lab desktop application.",
    );
  const request: CompileStRequest = {
    contractVersion: CONTRACT_VERSION,
    source,
  };
  let response: unknown;
  try {
    response = await invoke<unknown>(COMPILE_ST_COMMAND, { request });
  } catch (error: unknown) {
    if (
      isRecord(error) &&
      typeof error.code === "string" &&
      typeof error.message === "string"
    )
      throw new CoreInfoError(error.code, error.message);
    throw new CoreInfoError(
      "APP-IPC-004",
      error instanceof Error
        ? error.message
        : "The native compilation request failed.",
    );
  }
  return parseCompileResponse(response);
}
