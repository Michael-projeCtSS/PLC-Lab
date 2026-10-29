import { fireEvent, render, screen, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import type { HardwareDto } from "@plc-lab/ipc";
import { frame, station, workspace } from "../../../../tests/frontend/fixtures";
import type { Lab } from "./useLab";

const fake = vi.hoisted(() => ({
  hardware: vi.fn(),
  noteEvidence: vi.fn(),
  measure: vi.fn(),
}));
vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  api: fake,
}));

import { HardwareTab } from "./HardwareTab";

const hardware: HardwareDto = {
  station,
  cpu: "cpu.compact",
  cpuName: "Compact CPU",
  cpuProduct: null,
  cpuLeds: [{ name: "RUN", state: "GREEN" }],
  modules: [
    {
      moduleId: "di-r0s1",
      position: { rack: 0, slot: 1 },
      configured: "di8.24vdc.sink",
      installed: "do8.24vdc",
      configuredName: "DI 8",
      installedName: "DO 8",
      product: null,
      status: "MISMATCH",
      statusLed: "RED_FLASHING",
      channels: [
        {
          channelId: "di-r0s1.ch0",
          channel: 0,
          mode: "digital",
          tag: "StartPB",
          address: "%IX0.0",
          value: { type: "BOOL", value: false },
          quality: "MODULE_FAULT",
          output: null,
          led: false,
        },
      ],
    },
  ],
  diagnostics: [
    {
      timeUs: "0",
      source: "Rack 0 slot 1",
      kind: "MODULE_MISMATCH",
      event: "INCOMING",
      text: "Wrong module type",
    },
    {
      timeUs: "0",
      source: "Rack 0 slot 10",
      kind: "MODULE_MISSING",
      event: "INCOMING",
      text: "Other slot",
    },
  ],
  catalog: [
    {
      id: "di8.24vdc.sink",
      name: "DI 8",
      product: null,
      channels: 8,
      class: "DI",
    },
  ],
  findings: [
    {
      category: "WRONG_MODULE_TYPE",
      severity: "error",
      slot: { rack: 0, slot: 1 },
      channel: null,
      moduleId: "di-r0s1",
      expected: "DI 8",
      detected: "DO 8",
      message: "di-r0s1 at rack 0 slot 1: configured DI 8, detected DO 8",
    },
  ],
  signals: [
    {
      tag: "StartPB",
      channel: { rack: 0, slot: 1, channel: 0 },
      channelId: "di-r0s1.ch0",
      address: "%IX0.0",
      value: { type: "BOOL", value: false },
      quality: "MODULE_FAULT",
      output: null,
      drawnDevice: "Start S1",
      drawnPort: "contact",
      deviceState: "actuated",
    },
  ],
};

function lab(): Lab {
  // The view reads only these members of the lab.
  return {
    ws: { workspaceId: "1" },
    frame: frame(),
    workspace: workspace(),
    log: vi.fn(),
    busy: null,
    buildAndLoad: vi.fn(),
  } as unknown as Lab;
}

it("compares configured and detected hardware and follows signals", async () => {
  fake.hardware.mockResolvedValue(hardware);
  fake.noteEvidence.mockResolvedValue(null);
  const select = vi.fn();
  const { rerender } = render(
    <HardwareTab
      lab={lab()}
      selection={null}
      select={select}
      advanced={false}
    />,
  );
  const findings = await screen.findByRole("region", {
    name: "Configured versus actual",
  });
  expect(within(findings).getByText("Wrong module type")).toBeInTheDocument();
  expect(within(findings).getByText("DO 8")).toBeInTheDocument();
  // The detected station shows what is installed, the project what is configured.
  fireEvent.click(screen.getByRole("tab", { name: "Actual (detected)" }));
  expect(
    within(screen.getByRole("list", { name: "Detected station" })).getByText(
      "DO8.24VDC",
    ),
  ).toBeInTheDocument();
  fireEvent.click(screen.getByRole("tab", { name: "Configured (project)" }));
  expect(
    within(screen.getByRole("list", { name: "Configured station" })).getByText(
      "di-r0s1",
    ),
  ).toBeInTheDocument();
  fireEvent.click(screen.getByRole("tab", { name: "Signals" }));
  const signals = screen.getByRole("region", { name: "Signal mapping" });
  expect(within(signals).getByText("Start S1 (contact)")).toBeInTheDocument();
  expect(within(signals).getByText("actuated")).toBeInTheDocument();
  expect(within(signals).getByText("module fault")).toBeInTheDocument();
  // The inspector of a module: overview, power, and its own diagnostics only.
  fireEvent.click(screen.getByRole("tab", { name: /^Compare/ }));
  rerender(
    <HardwareTab
      lab={lab()}
      selection={{ kind: "module", rack: 0, slot: 1 }}
      select={select}
      advanced={false}
    />,
  );
  expect(screen.getByText("di-r0s1", { selector: "dd" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("tab", { name: "Power" }));
  expect(screen.getByText("the module answers the CPU")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("tab", { name: "Diagnostics" }));
  const diagnostics = screen.getByRole("region", {
    name: "Module diagnostics",
  });
  expect(
    within(diagnostics).getByText("Wrong module type", { selector: "td" }),
  ).toBeInTheDocument();
  expect(within(diagnostics).queryByText("Other slot")).toBeNull();
});
