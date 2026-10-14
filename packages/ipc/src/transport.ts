import { invoke, isTauri } from "@tauri-apps/api/core";
export class CoreInfoError extends Error {
  constructor(
    public readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = "CoreInfoError";
  }
}

export { CoreInfoError as IpcTransportError };
export async function invokeNative(
  command: string,
  request: unknown,
): Promise<unknown> {
  if (!isTauri())
    throw new CoreInfoError(
      "APP-IPC-003",
      "Open PLC Lab as a native desktop application.",
    );
  try {
    return await invoke<unknown>(command, { request });
  } catch (error: unknown) {
    if (
      typeof error === "object" &&
      error !== null &&
      "code" in error &&
      "message" in error &&
      typeof error.code === "string" &&
      typeof error.message === "string"
    )
      throw new CoreInfoError(error.code, error.message);
    throw new CoreInfoError(
      "APP-IPC-004",
      error instanceof Error ? error.message : "The native request failed.",
    );
  }
}
