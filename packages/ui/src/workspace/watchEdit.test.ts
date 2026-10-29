import { expect, it } from "vitest";
import type { WatchTableDto } from "@plc-lab/ipc";
import {
  MAX_ROWS,
  MAX_TABLES,
  addTable,
  formatWatchValue,
  newTableId,
  newTableName,
  setRows,
  updateTable,
} from "./watchEdit";

const table: WatchTableDto = {
  id: "watch-1",
  name: "Watch 1",
  rows: [
    { signal: "plc.LevelRaw", format: "hex" },
    { signal: "plc.StartPB", format: "auto" },
  ],
};

it("new tables get unused ids and names, up to the limit", () => {
  expect(newTableId([table])).toBe("watch-2");
  expect(newTableName([table])).toBe("Watch 2");
  const many = Array.from({ length: MAX_TABLES }, (_, i) => ({
    ...table,
    id: `watch-${i + 1}`,
  }));
  expect(addTable(many)).toHaveLength(MAX_TABLES);
  expect(addTable([table]).at(-1)).toEqual({
    id: "watch-2",
    name: "Watch 2",
    rows: [],
  });
});

it("rows follow canonical signal paths and keep their formats", () => {
  const next = setRows(table, [
    "plc.StartPB",
    "io.ai-r0s4.ch0",
    "plc.LevelRaw",
    "plc.StartPB",
  ]);
  expect(next.rows).toEqual([
    { signal: "plc.StartPB", format: "auto" },
    { signal: "io.ai-r0s4.ch0", format: "auto" },
    { signal: "plc.LevelRaw", format: "hex" },
  ]);
  const long = setRows(
    table,
    Array.from({ length: MAX_ROWS + 5 }, (_, i) => `plc.V${i}`),
  );
  expect(long.rows).toHaveLength(MAX_ROWS);
  expect(updateTable([table], next)).toEqual([next]);
  expect(updateTable([], next)).toEqual([next]);
});

it("integer values show in decimal, hexadecimal or binary", () => {
  const v = { type: "INT", value: 2000 } as const;
  expect(formatWatchValue(v, "auto", null)).toBe("2000");
  expect(formatWatchValue(v, "decimal", null)).toBe("2000");
  expect(formatWatchValue(v, "hex", null)).toBe("16#07D0");
  expect(formatWatchValue(v, "binary", null)).toBe("2#0000011111010000");
  expect(formatWatchValue({ type: "INT", value: -1 }, "hex", null)).toBe(
    "16#FFFF",
  );
  expect(formatWatchValue({ type: "BOOL", value: true }, "hex", null)).toBe(
    "TRUE",
  );
  expect(formatWatchValue(null, "hex", null)).toBe("—");
});
