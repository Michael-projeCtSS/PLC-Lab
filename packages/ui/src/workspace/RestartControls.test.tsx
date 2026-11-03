import { fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, expect, it, vi } from "vitest";
import type { LiveFrameDto } from "@plc-lab/ipc";
import { frame } from "../../../../tests/frontend/fixtures";
import { Context } from "../locale/context";
import { defaults } from "../locale/preferences";
import type { Lab } from "./useLab";

const fake = vi.hoisted(() => ({
  plcRestart: vi.fn(),
  plcReset: vi.fn(),
}));
vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  api: fake,
}));

import { RestartControls } from "./RestartControls";

function lab(plc: Partial<LiveFrameDto["plc"]> = {}): Lab {
  const base = frame();
  // The controls read only these members of the lab.
  return {
    ws: { workspaceId: "1" },
    frame: { ...base, plc: { ...base.plc, ...plc } },
    busy: null,
    act: vi.fn(async (_label: string, work: () => Promise<unknown>) => work()),
  } as unknown as Lab;
}
function inLanguage(language: "en" | "pl", children: ReactNode) {
  const preferences = defaults(language);
  return (
    <Context.Provider value={{ preferences, update: vi.fn(), problem: null }}>
      {children}
    </Context.Provider>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  fake.plcRestart.mockResolvedValue(frame());
  fake.plcReset.mockResolvedValue(frame());
});

it("basic mode stays uncluttered and offers only the memory reset a faulted CPU needs", () => {
  const { unmount } = render(
    inLanguage("en", <RestartControls lab={lab()} advanced={false} />),
  );
  expect(screen.queryByRole("button")).toBeNull();
  unmount();
  render(
    inLanguage(
      "en",
      <RestartControls lab={lab({ mode: "FAULT" })} advanced={false} />,
    ),
  );
  const buttons = screen.getAllByRole("button");
  expect(buttons.map((b) => b.textContent)).toEqual(["Reset PLC"]);
});

it("each restart asks first and only the confirmed choice reaches the backend", () => {
  render(inLanguage("en", <RestartControls lab={lab()} advanced />));
  fireEvent.click(screen.getByRole("button", { name: "Warm restart" }));
  let dialog = screen.getByRole("dialog", { name: "Warm restart the PLC?" });
  expect(dialog).toHaveTextContent("Retentive variables (VAR RETAIN) keep");
  // The safe choice has the focus; cancelling sends nothing.
  const cancel = within(dialog).getByRole("button", { name: "Cancel" });
  expect(document.activeElement).toBe(cancel);
  fireEvent.click(cancel);
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(fake.plcRestart).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Cold restart" }));
  dialog = screen.getByRole("dialog", { name: "Cold restart the PLC?" });
  fireEvent.click(within(dialog).getByRole("button", { name: "Cold restart" }));
  expect(fake.plcRestart).toHaveBeenCalledWith({ workspaceId: "1" }, "COLD");
  fireEvent.click(screen.getByRole("button", { name: "Warm restart" }));
  dialog = screen.getByRole("dialog", { name: "Warm restart the PLC?" });
  fireEvent.click(within(dialog).getByRole("button", { name: "Warm restart" }));
  expect(fake.plcRestart).toHaveBeenLastCalledWith(
    { workspaceId: "1" },
    "WARM",
  );
  // A memory reset asks too, and Escape withdraws the question.
  fireEvent.click(screen.getByRole("button", { name: "Reset PLC" }));
  screen.getByRole("dialog", { name: "Reset the PLC memory?" });
  fireEvent.keyDown(window, { key: "Escape" });
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(fake.plcReset).not.toHaveBeenCalled();
});

it("a restart needs a running CPU and the start state comes from the backend", () => {
  render(
    inLanguage(
      "en",
      <RestartControls
        lab={lab({
          mode: "STOP",
          lastStart: "WARM",
          retainedVariables: 2,
          firstScanPending: true,
        })}
        advanced
      />,
    ),
  );
  const warm = screen.getByRole("button", { name: "Warm restart" });
  expect(warm).toBeDisabled();
  expect(warm.title).toMatch(/needs the CPU in RUN/);
  expect(screen.getByRole("button", { name: "Reset PLC" })).toBeEnabled();
  expect(
    screen.getByRole("status", { name: "PLC start state" }),
  ).toHaveTextContent(
    "Last start: warm · Retentive variables: 2 · First PLC scan pending",
  );
});

it("the restart controls and questions are available in Polish", () => {
  render(
    inLanguage(
      "pl",
      <RestartControls lab={lab({ lastStart: "COLD" })} advanced />,
    ),
  );
  expect(
    screen.getByRole("status", { name: "Stan startu PLC" }),
  ).toHaveTextContent("Ostatni start: zimny");
  fireEvent.click(screen.getByRole("button", { name: "Restart zimny" }));
  const dialog = screen.getByRole("dialog", {
    name: "Wykonać restart zimny PLC?",
  });
  expect(dialog).toHaveTextContent("także zmienne retencyjne");
  expect(
    within(dialog).getByRole("button", { name: "Anuluj" }),
  ).toBeInTheDocument();
});
