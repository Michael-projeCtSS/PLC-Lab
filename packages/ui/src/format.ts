import type { SignalValueDto } from "@plc-lab/ipc";

/** IEC 61131-3 duration literal, e.g. T#1s500ms. Input in microseconds (exact). */
export function plcTime(microseconds: string | bigint): string {
  let rest = BigInt(microseconds);
  if (rest === 0n) return "T#0ms";
  const units: [string, bigint][] = [
    ["d", 86_400_000_000n],
    ["h", 3_600_000_000n],
    ["m", 60_000_000n],
    ["s", 1_000_000n],
    ["ms", 1_000n],
    ["us", 1n],
  ];
  let out = "T#";
  for (const [name, size] of units) {
    const n = rest / size;
    if (n > 0n) {
      out += `${n}${name}`;
      rest -= n * size;
    }
  }
  return out;
}

/** World time as seconds with millisecond resolution, e.g. 12.300 s. */
export function seconds(microseconds: string | bigint): string {
  const us = BigInt(microseconds);
  const ms = us / 1000n;
  return `${ms / 1000n}.${String(ms % 1000n).padStart(3, "0")} s`;
}

export function formatValue(
  value: SignalValueDto | null | undefined,
  unit?: string | null,
): string {
  if (!value) return "—";
  switch (value.type) {
    case "BOOL":
      return value.value ? "TRUE" : "FALSE";
    case "INT":
      return String(value.value);
    case "REAL":
      return Number.isInteger(value.value)
        ? value.value.toFixed(1)
        : String(Number(value.value.toPrecision(7)));
    case "TIME":
      return plcTime(value.value);
    case "COUNT":
      return formatQuantity(value.value, unit ?? null);
  }
}

/** Physical integer quantities carry unit hints in their names (permille, ml, ma). */
export function formatQuantity(value: number, unit: string | null): string {
  switch (unit) {
    case "‰":
      return `${(value / 10).toFixed(1)} %`;
    case "ml":
      return value >= 1000 ? `${(value / 1000).toFixed(2)} L` : `${value} mL`;
    case "ml/s":
      return `${(value / 1000).toFixed(2)} L/s`;
    case "mA":
      return `${(value / 1000).toFixed(2)} A`;
    case "µA":
      return `${(value / 1000).toFixed(2)} mA`;
    case "mV":
      return `${(value / 1000).toFixed(2)} V`;
    default:
      return unit ? `${value} ${unit}` : String(value);
  }
}

export function valueClass(value: SignalValueDto | null | undefined): string {
  if (value?.type === "BOOL") return value.value ? "value true" : "value false";
  return "value";
}

export function isTrue(value: SignalValueDto | null | undefined): boolean {
  return value?.type === "BOOL" && value.value;
}

/** Number for plotting; TIME in milliseconds. */
export function numeric(value: SignalValueDto): number {
  switch (value.type) {
    case "BOOL":
      return value.value ? 1 : 0;
    case "TIME":
      return Number(BigInt(value.value) / 1000n);
    default:
      return value.value;
  }
}

export function quantityUnit(name: string): string | null {
  if (name.endsWith("Permille")) return "‰";
  if (name.endsWith("MlPerS")) return "ml/s";
  if (name.endsWith("Ml")) return "ml";
  if (name.endsWith("Ma")) return "mA";
  return null;
}

export function humanize(name: string): string {
  return name
    .replace(/(Permille|MlPerS|Ml|Ma)$/, "")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/^./, (c) => c.toUpperCase());
}
