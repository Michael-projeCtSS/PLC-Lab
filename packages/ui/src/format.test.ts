import { expect, it } from "vitest";
import { formatQuantity, formatValue, plcTime, seconds } from "./format";

it("formats TIME values in IEC notation from exact microseconds", () => {
  expect(plcTime("0")).toBe("T#0ms");
  expect(plcTime("1500000")).toBe("T#1s500ms");
  expect(plcTime("90000000")).toBe("T#1m30s");
  expect(plcTime("250")).toBe("T#250us");
  expect(plcTime("18446744073709551615")).toMatch(/^T#\d+d/);
  expect(formatValue({ type: "TIME", value: "5000000" })).toBe("T#5s");
});

it("formats PLC and process values for display", () => {
  expect(formatValue({ type: "BOOL", value: true })).toBe("TRUE");
  expect(formatValue({ type: "INT", value: -5 })).toBe("-5");
  expect(formatValue({ type: "REAL", value: 2 })).toBe("2.0");
  expect(formatValue(null)).toBe("—");
  expect(formatQuantity(735, "‰")).toBe("73.5 %");
  expect(formatQuantity(12000, "µA")).toBe("12.00 mA");
  expect(seconds("1234567")).toBe("1.234 s");
});
