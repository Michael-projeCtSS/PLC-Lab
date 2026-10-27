import { invoke, isTauri } from "@tauri-apps/api/core";

/** A typed failure reported by the native application or by response validation. */
export class IpcClientError extends Error {
  constructor(
    public readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = "IpcClientError";
  }
}

export function nativeAvailable(): boolean {
  return isTauri();
}

/** Invoke one native command with a request object. Errors are normalized. */
export async function invokeNative(
  command: string,
  request: unknown,
): Promise<unknown> {
  if (!isTauri())
    throw new IpcClientError(
      "APP-IPC-NATIVE",
      "Open PLC Lab as the desktop application to use this feature.",
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
      throw new IpcClientError(error.code, error.message);
    throw new IpcClientError(
      "APP-IPC-TRANSPORT",
      error instanceof Error ? error.message : "The native request failed.",
    );
  }
}
