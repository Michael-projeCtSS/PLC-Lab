import { describe, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { compileSt, parseCompileResponse } from "./index";
import { CONTRACT_VERSION } from "./generated";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
const success = {
  success: true,
  programName: "Main",
  symbols: [{ name: "Motor", typeName: "BOOL", role: "Output" }],
  diagnostics: [],
};
const failure = {
  success: false,
  diagnostics: [
    {
      code: "PLC-ST-006",
      severity: "error",
      message: "Unknown variable 'Missing'.",
      span: { start: { line: 3, column: 10 }, end: { line: 3, column: 17 } },
    },
  ],
};
describe("typed ST compilation boundary", () => {
  it("invokes only compile_st with versioned source and accepts metadata", async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockResolvedValue(success);
    await expect(compileSt("PROGRAM Main END_PROGRAM")).resolves.toEqual(
      success,
    );
    expect(invoke).toHaveBeenCalledWith("compile_st", {
      request: {
        contractVersion: CONTRACT_VERSION,
        source: "PROGRAM Main END_PROGRAM",
      },
    });
  });
  it("accepts backend failure as compiler data", () => {
    expect(parseCompileResponse(failure)).toEqual(failure);
  });
  it.each([
    null,
    {},
    { ...success, executionPlan: {} },
    { ...success, success: false },
    { ...success, symbols: [{ name: "X", typeName: "DINT", role: "Output" }] },
    { ...failure, diagnostics: [] },
    {
      ...failure,
      diagnostics: [
        {
          ...failure.diagnostics[0],
          span: { start: { line: 0, column: 1 }, end: { line: 1, column: 1 } },
        },
      ],
    },
    {
      ...failure,
      diagnostics: [
        {
          ...failure.diagnostics[0],
          span: { start: { line: 3, column: 10 }, end: { line: 3, column: 1 } },
        },
      ],
    },
    {
      ...failure,
      diagnostics: [{ ...failure.diagnostics[0], severity: "unknown" }],
    },
  ])(
    "rejects malformed, contradictory or executable-shaped responses: %j",
    (value) => {
      expect(() => parseCompileResponse(value)).toThrow();
    },
  );
  it("never substitutes a browser compiler", async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    await expect(compileSt("")).rejects.toMatchObject({ code: "APP-IPC-003" });
    expect(invoke).not.toHaveBeenCalled();
  });
  it("preserves structured backend transport errors", async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockRejectedValue({
      code: "APP-IPC-001",
      message: "Wrong version.",
    });
    await expect(compileSt("")).rejects.toMatchObject({ code: "APP-IPC-001" });
  });
});
