import { fireEvent, render, screen, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import type { SignalDto, WatchDto, WatchTableDto } from "@plc-lab/ipc";
import { frame } from "../../../../tests/frontend/fixtures";
import type { Lab } from "./useLab";

const fake = vi.hoisted(() => ({ watch: vi.fn() }));
vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  api: fake,
}));

import { WatchTables } from "./WatchTables";

const signal = (
  id: number,
  path: string,
  label: string,
  kind: SignalDto["kind"],
  valueType: SignalDto["valueType"],
): SignalDto => ({
  id,
  path,
  label,
  kind,
  valueType,
  unit: null,
  active: true,
});
const signals = [
  signal(1, "plc.LevelRaw", "LevelRaw", "PLC_INPUT", "INT"),
  signal(2, "plc.InletValve", "InletValve", "PLC_OUTPUT", "BOOL"),
  signal(3, "io.ai-r0s4.ch0", "R0/S4/CH0", "CHANNEL", "INT"),
];
const tables: WatchTableDto[] = [
  {
    id: "watch-1",
    name: "Filling",
    rows: [
      { signal: "plc.LevelRaw", format: "hex" },
      { signal: "plc.InletValve", format: "auto" },
      { signal: "plc.Removed", format: "auto" },
    ],
  },
];
const live: WatchDto = {
  worldTimeUs: "1500000",
  cycle: "15",
  rows: [
    {
      signal: 1,
      path: "plc.LevelRaw",
      label: "LevelRaw",
      kind: "PLC_INPUT",
      valueType: "INT",
      unit: null,
      active: true,
      value: { type: "INT", value: 2000 },
      scanValue: { type: "INT", value: 2000 },
      committed: null,
      staged: { type: "INT", value: 2010 },
      source: "%IW64",
      quality: "WIRE_BREAK",
      forced: null,
    },
    {
      signal: 2,
      path: "plc.InletValve",
      label: "InletValve",
      kind: "PLC_OUTPUT",
      valueType: "BOOL",
      unit: null,
      active: true,
      value: { type: "BOOL", value: true },
      scanValue: null,
      committed: { type: "BOOL", value: false },
      staged: null,
      source: "%QX0.0",
      quality: "GOOD",
      forced: { type: "BOOL", value: true },
    },
  ],
};

function lab(saveWatchTables = vi.fn()): Lab {
  // The panel reads only these members of the lab.
  return {
    ws: { workspaceId: "1" },
    frame: frame(),
    signals,
    watchTables: tables,
    activeTable: "watch-1",
    selectTable: vi.fn(),
    saveWatchTables,
  } as unknown as Lab;
}

it("shows the runtime's values, quality and forces for canonical signals", async () => {
  fake.watch.mockResolvedValue(live);
  render(<WatchTables lab={lab()} />);
  const grid = await screen.findByRole("table", {
    name: "Watch table Filling",
  });
  expect(await within(grid).findByText("16#07D0")).toBeInTheDocument();
  expect(within(grid).getByText("wire break")).toBeInTheDocument();
  expect(within(grid).getByText("F TRUE")).toBeInTheDocument();
  expect(
    within(grid).getByText(/plc\.Removed: not in the loaded program/),
  ).toBeInTheDocument();
  // Only resolved signals are queried, by id.
  expect(fake.watch).toHaveBeenCalledWith({ workspaceId: "1" }, [1, 2]);
  // More columns on request: canonical id, scan input, next input.
  fireEvent.click(screen.getByText("Columns"));
  fireEvent.click(screen.getByLabelText("Canonical ID"));
  fireEvent.click(screen.getByLabelText("Next input"));
  expect(within(grid).getByText("plc.LevelRaw")).toBeInTheDocument();
  expect(within(grid).getByText("2010")).toBeInTheDocument();
});

it("tables and rows are edited through project saves", async () => {
  fake.watch.mockResolvedValue(live);
  const save = vi.fn();
  render(<WatchTables lab={lab(save)} />);
  await screen.findByRole("table", { name: "Watch table Filling" });
  // Add a hardware channel by search.
  fireEvent.change(screen.getByLabelText("Find a signal"), {
    target: { value: "s4" },
  });
  fireEvent.click(screen.getByRole("button", { name: /Add R0\/S4\/CH0/ }));
  const [added, active] = save.mock.calls[0]!;
  expect(active).toBe("watch-1");
  expect((added as WatchTableDto[])[0]!.rows.at(-1)).toEqual({
    signal: "io.ai-r0s4.ch0",
    format: "auto",
  });
  // Format of a row.
  fireEvent.change(screen.getByLabelText("Format of InletValve"), {
    target: { value: "binary" },
  });
  expect((save.mock.calls[1]![0] as WatchTableDto[])[0]!.rows[1]).toEqual({
    signal: "plc.InletValve",
    format: "binary",
  });
  // New table, rename, delete.
  fireEvent.click(screen.getByRole("button", { name: "New table" }));
  expect(save.mock.calls[2]).toEqual([
    [...tables, { id: "watch-2", name: "Watch 1", rows: [] }],
    "watch-2",
  ]);
  fireEvent.change(screen.getByLabelText("Name"), {
    target: { value: "Level loop" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Rename" }));
  expect((save.mock.calls[3]![0] as WatchTableDto[])[0]!.name).toBe(
    "Level loop",
  );
  fireEvent.click(screen.getByRole("button", { name: "Delete table" }));
  expect(save.mock.calls[4]).toEqual([[], null]);
});
