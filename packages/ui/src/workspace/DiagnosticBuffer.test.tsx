import { fireEvent, render, screen, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import type { DiagnosticBufferDto, DiagnosticEntryDto } from "@plc-lab/ipc";
import { frame, project } from "../../../../tests/frontend/fixtures";
import type { Lab } from "./useLab";

const fake = vi.hoisted(() => ({ diagnosticBuffer: vi.fn() }));
vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  api: fake,
}));

import { DiagnosticBuffer } from "./DiagnosticBuffer";

const entry = (over: Partial<DiagnosticEntryDto>): DiagnosticEntryDto => ({
  timeUs: "1000000",
  source: "CPU",
  kind: "ModeChange",
  event: "EVENT",
  text: "CPU operating state changed from STOP to RUN.",
  slot: null,
  channel: null,
  moduleId: null,
  severity: "info",
  category: "CPU",
  details: [
    { name: "from", value: "STOP" },
    { name: "to", value: "RUN" },
  ],
  active: false,
  clearedUs: null,
  ...over,
});
const buffer: DiagnosticBufferDto = {
  capacity: 200,
  recorded: "3",
  matching: 3,
  entries: [
    entry({}),
    entry({
      timeUs: "2000000",
      source: "Rack 0 slot 3",
      kind: "NoFieldPower",
      event: "INCOMING",
      text: "Load/field supply voltage missing on the module.",
      moduleId: "do-r0s3",
      severity: "error",
      category: "HARDWARE",
      details: [{ name: "module", value: "do-r0s3" }],
      clearedUs: "4500000",
    }),
    entry({
      timeUs: "5000000",
      source: "Rack 0 slot 4 channel 0 (%IW64)",
      kind: "WireBreak",
      event: "INCOMING",
      text: "Wire break on analog input.",
      severity: "error",
      category: "HARDWARE",
      details: [{ name: "channel", value: "ai-r0s4.ch0" }],
      active: true,
    }),
  ],
};

function lab(): Lab {
  // The view reads only these members of the lab.
  return {
    ws: { workspaceId: "1" },
    frame: frame(),
    project: project(),
  } as unknown as Lab;
}

it("shows severity, category, state and details from the backend and filters by request", async () => {
  fake.diagnosticBuffer.mockResolvedValue(buffer);
  render(<DiagnosticBuffer lab={lab()} />);
  const table = await screen.findByRole("table", { name: "Diagnostic buffer" });
  expect(
    await within(table).findByText("from: STOP, to: RUN"),
  ).toBeInTheDocument();
  expect(within(table).getByText("event")).toBeInTheDocument();
  expect(within(table).getByText("cleared after 2.500 s")).toBeInTheDocument();
  expect(within(table).getByText("active")).toBeInTheDocument();
  expect(within(table).getByText("channel: ai-r0s4.ch0")).toBeInTheDocument();
  expect(fake.diagnosticBuffer).toHaveBeenLastCalledWith(
    { workspaceId: "1" },
    { offset: 0, limit: 50, severity: null, category: null, moduleId: null },
  );
  // Filters are request parameters: the backend filters.
  fireEvent.change(screen.getByLabelText("Severity"), {
    target: { value: "error" },
  });
  fireEvent.change(screen.getByLabelText("Category"), {
    target: { value: "HARDWARE" },
  });
  fireEvent.change(screen.getByLabelText("Module"), {
    target: { value: "di-r0s1" },
  });
  await vi.waitFor(() =>
    expect(fake.diagnosticBuffer).toHaveBeenLastCalledWith(
      { workspaceId: "1" },
      {
        offset: 0,
        limit: 50,
        severity: "error",
        category: "HARDWARE",
        moduleId: "di-r0s1",
      },
    ),
  );
  // Three entries fit on one page: nothing older, nothing newer.
  expect(screen.getByRole("button", { name: "Older" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Newer" })).toBeDisabled();
});
