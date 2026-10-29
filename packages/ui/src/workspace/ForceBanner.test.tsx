import { fireEvent, render, screen, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import type { ForceListDto, LiveFrameDto } from "@plc-lab/ipc";
import { frame } from "../../../../tests/frontend/fixtures";
import type { Lab } from "./useLab";

const fake = vi.hoisted(() => ({
  forces: vi.fn(),
  force: vi.fn(),
  removeAllForces: vi.fn(),
}));
vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  api: fake,
}));

import { ForceBanner } from "./ForceBanner";

const list: ForceListDto = {
  forces: [
    {
      signal: 4,
      path: "plc.LevelRaw",
      label: "LevelRaw",
      target: "INPUT_IMAGE",
      value: { type: "INT", value: 1234 },
    },
    {
      signal: 7,
      path: "plc.InletValve",
      label: "InletValve",
      target: "OUTPUT_IMAGE",
      value: { type: "BOOL", value: false },
    },
  ],
};

function lab(forces: number, act = vi.fn()): Lab {
  const f: LiveFrameDto = frame();
  // The banner reads only these members of the lab.
  return {
    ws: { workspaceId: "1" },
    frame: { ...f, plc: { ...f.plc, forces } },
    busy: null,
    act,
  } as unknown as Lab;
}

it("stays hidden without forces and warns persistently while any force is active", async () => {
  fake.forces.mockResolvedValue(list);
  const { rerender } = render(<ForceBanner lab={lab(0)} />);
  expect(screen.queryByRole("alert", { name: "Forces active" })).toBeNull();
  const act = vi.fn();
  rerender(<ForceBanner lab={lab(2, act)} />);
  const banner = screen.getByRole("alert", { name: "Forces active" });
  expect(within(banner).getByText("FORCES ACTIVE")).toBeInTheDocument();
  expect(within(banner).getByText(/2 signals are forced/)).toBeInTheDocument();
  // The list names what each force replaces.
  fireEvent.click(screen.getByRole("button", { name: "Show forces" }));
  const table = await screen.findByRole("table", { name: "Active forces" });
  expect(within(table).getByText(/PLC input image/)).toBeInTheDocument();
  expect(within(table).getByText(/PLC output image/)).toBeInTheDocument();
  expect(within(table).getByText("F 1234")).toBeInTheDocument();
  // Removing goes to the backend: one force, or all of them in one request.
  fireEvent.click(
    within(table).getAllByRole("button", { name: "Remove force" })[0]!,
  );
  await act.mock.calls[0]![1]();
  expect(fake.force).toHaveBeenCalledWith({ workspaceId: "1" }, 4, null);
  fireEvent.click(screen.getByRole("button", { name: "Remove all forces" }));
  await act.mock.calls[1]![1]();
  expect(fake.removeAllForces).toHaveBeenCalledWith({ workspaceId: "1" });
});
