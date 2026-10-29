import { render, screen, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import type { HardwareDto, SignalDto, TrendDto, WatchDto } from "@plc-lab/ipc";
import { frame } from "../../../../tests/frontend/fixtures";
import type { Lab } from "./useLab";

const fake = vi.hoisted(() => ({
  watch: vi.fn(),
  trend: vi.fn(),
  hardware: vi.fn(),
}));
vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  api: fake,
}));

import { VariableDetail } from "./VariableDetail";

const signal: SignalDto = {
  id: 3,
  path: "plc.InletClosed",
  label: "InletClosed",
  kind: "PLC_INPUT",
  valueType: "BOOL",
  unit: null,
  active: true,
};
const live: WatchDto = {
  worldTimeUs: "3000000",
  cycle: "30",
  rows: [
    {
      signal: 3,
      path: "plc.InletClosed",
      label: "InletClosed",
      kind: "PLC_INPUT",
      valueType: "BOOL",
      unit: null,
      active: true,
      value: { type: "BOOL", value: false },
      scanValue: { type: "BOOL", value: false },
      committed: null,
      staged: { type: "BOOL", value: true },
      source: "%IX0.4",
      quality: "NO_FIELD_POWER",
      forced: null,
    },
  ],
};
const trend: TrendDto = {
  oldestUs: "0",
  latestUs: "3000000",
  series: [
    {
      signal: 3,
      layer: "INPUTS",
      points: [
        { t: "1000000", value: { type: "BOOL", value: true } },
        { t: "1500000", value: { type: "BOOL", value: true } },
        { t: "2000000", value: { type: "BOOL", value: false } },
      ],
      decimated: false,
    },
  ],
};

function lab(): Lab {
  // The detail reads only these members of the lab.
  return {
    ws: { workspaceId: "1" },
    frame: frame(),
    signals: [signal],
    act: vi.fn(),
  } as unknown as Lab;
}

it("shows the runtime's view of one signal, its binding and its recent changes", async () => {
  fake.watch.mockResolvedValue(live);
  fake.trend.mockResolvedValue(trend);
  fake.hardware.mockResolvedValue({
    signals: [
      {
        tag: "InletClosed",
        channel: { rack: 0, slot: 2, channel: 4 },
        channelId: "di-r0s2.ch4",
        address: "%IX0.4",
        value: { type: "BOOL", value: false },
        quality: "NO_FIELD_POWER",
        output: null,
        drawnDevice: "Inlet valve XV-301",
        drawnPort: "closedSwitch",
        deviceState: "contact closed",
      },
    ],
  } as unknown as HardwareDto);
  render(<VariableDetail lab={lab()} signal={3} />);
  const detail = screen.getByLabelText("Online variable");
  expect(within(detail).getByText("plc.InletClosed")).toBeInTheDocument();
  expect(await within(detail).findByText("no field power")).toBeInTheDocument();
  expect(within(detail).getByText("Next input")).toBeInTheDocument();
  expect(await within(detail).findByText("di-r0s2.ch4")).toBeInTheDocument();
  expect(
    within(detail).getByText("Inlet valve XV-301 (closedSwitch)"),
  ).toBeInTheDocument();
  // Only value changes are listed: TRUE at 1 s, FALSE at 2 s (not the repeat at 1.5 s).
  const changes = await within(detail).findByRole("table", {
    name: "Recent changes",
  });
  expect(within(changes).getAllByRole("row")).toHaveLength(2);
  expect(within(changes).getByText("2.000 s")).toBeInTheDocument();
  // PLC inputs can be forced from here.
  expect(within(detail).getByTitle("Force TRUE")).toBeInTheDocument();
});
