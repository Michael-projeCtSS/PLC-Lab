import { invoke, isTauri } from "@tauri-apps/api/core";
import { CONTRACT_VERSION, CORE_INFO_COMMAND } from "./generated";
import type { CoreInfo, CoreInfoRequest } from "./generated";

export type { CoreInfo, IpcError } from "./generated";

export class CoreInfoError extends Error {
  constructor(
    public readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = "CoreInfoError";
  }
}

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
    value.execution !== "unavailable" ||
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
