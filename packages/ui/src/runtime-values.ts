import { parseRuntimeValue } from "@plc-lab/ipc";
import type { RuntimeValue, RuntimeVariable } from "@plc-lab/ipc";

/** Display-only duration formatting; never advances simulation time. */
export function formatDuration(microseconds: string): string {
  const value = BigInt(microseconds);
  return value % 1000n === 0n ? `${value / 1000n} ms` : `${value} µs`;
}
export function formatRuntimeValue(value: RuntimeValue): string {
  switch (value.type) {
    case "BOOL":
      return value.value ? "TRUE" : "FALSE";
    case "TIME": {
      const n = BigInt(value.microseconds);
      return n % 1000n === 0n ? `T#${n / 1000n}ms` : `T#${n}us`;
    }
    default:
      return String(value.value);
  }
}
export function inputDraft(value: RuntimeValue): string {
  return value.type === "TIME" ? value.microseconds : String(value.value);
}
/** Bare input-widget validation, not ST literal parsing or PLC evaluation. */
export function requestedInput(
  type: RuntimeVariable["typeName"],
  draft: string,
): RuntimeValue {
  if (type === "TIME") {
    try {
      return parseRuntimeValue({ type, microseconds: draft });
    } catch {
      throw new Error(
        "TIME requires decimal microseconds from 0 to 18446744073709551615.",
      );
    }
  }
  if (type === "BOOL") throw new Error("Use the BOOL toggle.");
  if (type === "INT" && !/^-?(0|[1-9][0-9]*)$/.test(draft))
    throw new Error("INT requires a signed decimal integer.");
  if (
    type === "REAL" &&
    !/^[+-]?(?:[0-9]+\.?[0-9]*|\.[0-9]+)(?:[eE][+-]?[0-9]+)?$/.test(draft)
  )
    throw new Error("REAL requires a finite decimal number.");
  const value = Number(draft);
  try {
    return parseRuntimeValue({ type, value: value === 0 ? 0 : value });
  } catch {
    throw new Error(
      type === "INT"
        ? "INT must be between -32768 and 32767."
        : "REAL must fit a finite binary32 value.",
    );
  }
}
