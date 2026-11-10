import { describe, expect, it } from "vitest";
import { flag, parseParameter, real, timeSeconds } from "./tuning";

describe("tuning values", () => {
  it("parses REAL parameters with a decimal point or comma", () => {
    expect(parseParameter("REAL", "2.5")).toEqual({ type: "REAL", value: 2.5 });
    expect(parseParameter("REAL", " 2,5 ")).toEqual({
      type: "REAL",
      value: 2.5,
    });
    expect(parseParameter("REAL", "-10")).toEqual({ type: "REAL", value: -10 });
    expect(parseParameter("REAL", "1e2")).toEqual({ type: "REAL", value: 100 });
  });

  it("parses TIME parameters in seconds into microseconds", () => {
    expect(parseParameter("TIME", "20")).toEqual({
      type: "TIME",
      value: "20000000",
    });
    expect(parseParameter("TIME", "0,5")).toEqual({
      type: "TIME",
      value: "500000",
    });
    expect(parseParameter("TIME", "0")).toEqual({ type: "TIME", value: "0" });
  });

  it("rejects what is not a value of the parameter's type", () => {
    for (const text of ["", "  ", "abc", "2.5.1", "1,2,3", "Infinity", "NaN"])
      expect(parseParameter("REAL", text)).toBeNull();
    expect(parseParameter("TIME", "-1")).toBeNull();
    expect(parseParameter("TIME", "86401")).toBeNull();
    expect(parseParameter("BOOL", "1")).toBeNull();
  });

  it("reads typed values and nothing else", () => {
    expect(real({ type: "REAL", value: 1.5 })).toBe(1.5);
    expect(real({ type: "INT", value: 1 })).toBeNull();
    expect(timeSeconds({ type: "TIME", value: "2500000" })).toBe(2.5);
    expect(timeSeconds(null)).toBeNull();
    expect(flag({ type: "BOOL", value: true })).toBe(true);
    expect(flag({ type: "REAL", value: 1 })).toBeNull();
  });
});
