import { describe, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { CoreInfoError, getCoreInfo, parseCoreInfo } from "./index";
import { CONTRACT_VERSION } from "./generated";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: vi.fn() }));

const response = {
  applicationName: "PLC Lab",
  appVersion: "0.1.0",
  coreVersion: "0.1.0",
  contractVersion: CONTRACT_VERSION,
  execution: "unavailable",
  buildContext: "development",
};

describe("core information boundary", () => {
  it("invokes the registered command with the typed version request", async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockResolvedValue(response);
    await expect(getCoreInfo()).resolves.toEqual(response);
    expect(invoke).toHaveBeenCalledWith("get_core_info", {
      request: { contractVersion: CONTRACT_VERSION },
    });
  });

  it("reports unavailable native IPC in a browser without invoking", async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    await expect(getCoreInfo()).rejects.toMatchObject({ code: "APP-IPC-003" });
    expect(invoke).not.toHaveBeenCalled();
  });

  it.each([
    null,
    {},
    { ...response, appVersion: "" },
    { ...response, coreVersion: 4 },
    { ...response, contractVersion: CONTRACT_VERSION + 1 },
    { ...response, execution: "running" },
    { ...response, buildContext: undefined },
    { ...response, buildContext: "unknown" },
  ])("rejects malformed or incompatible responses: %j", (value) => {
    expect(() => parseCoreInfo(value)).toThrow(CoreInfoError);
  });

  it("preserves a structured Rust rejection", async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockRejectedValue({
      code: "APP-IPC-001",
      message: "Unsupported contract version.",
    });
    await expect(getCoreInfo()).rejects.toMatchObject({
      code: "APP-IPC-001",
      message: "Unsupported contract version.",
    });
  });

  it("accepts authoritative release build context", () => {
    expect(
      parseCoreInfo({ ...response, buildContext: "release" }).buildContext,
    ).toBe("release");
  });

  it.each([new Error("Transport disconnected"), "unexpected failure"])(
    "normalizes transport failures: %j",
    async (error) => {
      vi.mocked(isTauri).mockReturnValue(true);
      vi.mocked(invoke).mockRejectedValue(error);
      await expect(getCoreInfo()).rejects.toMatchObject({
        code: "APP-IPC-004",
      });
    },
  );
});
