import { describe, expect, it } from "vitest";
import {
  frame,
  library,
  project,
  workspace,
} from "../../../tests/frontend/fixtures";
import { COMMANDS } from "./client";
import { IpcClientError } from "./transport";
import * as V from "./validators";

describe("strict response validation", () => {
  it("accepts contract-valid fixtures unchanged", () => {
    expect(V.workspace(workspace(), "w")).toEqual(workspace());
    expect(V.frame(frame(), "f")).toEqual(frame());
    expect(V.project(project(), "p")).toEqual(project());
    expect(V.library(library, "l")).toEqual(library);
  });

  it("rejects unknown and missing fields like Rust deny_unknown_fields", () => {
    expect(() => V.frame({ ...frame(), extra: 1 }, "f")).toThrow(
      IpcClientError,
    );
    const { cycle: _omit, ...missing } = frame();
    void _omit;
    expect(() => V.frame(missing, "f")).toThrow(/cycle/);
  });

  it("keeps 64-bit values exact and canonical", () => {
    expect(
      V.frame(frame({ worldTimeUs: "18446744073709551615" }), "f").worldTimeUs,
    ).toBe("18446744073709551615");
    for (const bad of ["18446744073709551616", "01", "-1", "1e3", 12])
      expect(() => V.frame(frame({ worldTimeUs: bad as string }), "f")).toThrow(
        IpcClientError,
      );
  });

  it("checks PLC value ranges and tagged unions", () => {
    expect(V.signalValue({ type: "INT", value: -32768 }, "v")).toEqual({
      type: "INT",
      value: -32768,
    });
    expect(() => V.signalValue({ type: "INT", value: 40000 }, "v")).toThrow();
    expect(() =>
      V.signalValue({ type: "REAL", value: Number.NaN }, "v"),
    ).toThrow();
    expect(() => V.signalValue({ type: "DINT", value: 1 }, "v")).toThrow();
    expect(() =>
      V.signalValue({ type: "BOOL", value: true, x: 1 }, "v"),
    ).toThrow();
  });

  it("rejects unknown enumerated states", () => {
    const bad = frame();
    expect(() =>
      V.frame({ ...bad, plc: { ...bad.plc, mode: "HALT" } }, "f"),
    ).toThrow(/RUN \| STOP \| FAULT/);
  });

  it("names every command the desktop host registers", async () => {
    const { readFileSync } = await import("node:fs");
    const { join } = await import("node:path");
    const host = readFileSync(
      join(process.cwd(), "apps/desktop/src-tauri/src/lib.rs"),
      "utf8",
    );
    const registered = [...host.matchAll(/commands::([a-z_]+)/g)]
      .map((m) => m[1])
      .sort();
    expect([...COMMANDS].sort()).toEqual(registered);
  });
});
