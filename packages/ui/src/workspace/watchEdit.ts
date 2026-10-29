/** Pure helpers for watch tables: identities, names, row edits and value formats. Watch
 * rows name canonical signal paths, never editor text (ADR-0032). */
import type { SignalValueDto, WatchTableDto } from "@plc-lab/ipc";
import { formatValue } from "../format";

export const MAX_TABLES = 16;
export const MAX_ROWS = 64;
export type WatchFormat = WatchTableDto["rows"][number]["format"];

/** A table id not used yet: `watch-<n>` with the smallest free n. */
export function newTableId(tables: readonly WatchTableDto[]): string {
  const taken = new Set(tables.map((t) => t.id));
  let n = 1;
  while (taken.has(`watch-${n}`)) n++;
  return `watch-${n}`;
}

/** A default name not used yet: `Watch <n>`. */
export function newTableName(tables: readonly WatchTableDto[]): string {
  const taken = new Set(tables.map((t) => t.name));
  let n = 1;
  while (taken.has(`Watch ${n}`)) n++;
  return `Watch ${n}`;
}

export function addTable(tables: readonly WatchTableDto[]): WatchTableDto[] {
  if (tables.length >= MAX_TABLES) return [...tables];
  return [
    ...tables,
    { id: newTableId(tables), name: newTableName(tables), rows: [] },
  ];
}

/** Replace a table's rows by signal paths, keeping the format of rows that stay. */
export function setRows(
  table: WatchTableDto,
  signals: readonly string[],
): WatchTableDto {
  const formats = new Map(table.rows.map((r) => [r.signal, r.format]));
  const unique = [...new Set(signals)].slice(0, MAX_ROWS);
  return {
    ...table,
    rows: unique.map((signal) => ({
      signal,
      format: formats.get(signal) ?? "auto",
    })),
  };
}

export function updateTable(
  tables: readonly WatchTableDto[],
  next: WatchTableDto,
): WatchTableDto[] {
  return tables.some((t) => t.id === next.id)
    ? tables.map((t) => (t.id === next.id ? next : t))
    : [...tables, next];
}

/** An integer value in the row's format; other values as usual. */
export function formatWatchValue(
  value: SignalValueDto | null | undefined,
  format: WatchFormat,
  unit: string | null,
): string {
  if (!value || value.type !== "INT" || format === "auto")
    return formatValue(value, unit);
  const bits = value.value & 0xffff;
  switch (format) {
    case "decimal":
      return String(value.value);
    case "hex":
      return `16#${bits.toString(16).toUpperCase().padStart(4, "0")}`;
    case "binary":
      return `2#${bits.toString(2).padStart(16, "0")}`;
  }
}
