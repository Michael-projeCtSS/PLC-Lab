import { fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, expect, it, vi } from "vitest";
import type { AlarmDto, AlarmsDto } from "@plc-lab/ipc";
import { frame } from "../../../../tests/frontend/fixtures";
import { Context } from "../locale/context";
import { defaults } from "../locale/preferences";
import type { Lab } from "./useLab";

const fake = vi.hoisted(() => ({
  alarms: vi.fn(),
  acknowledgeAlarms: vi.fn(),
  noteEvidence: vi.fn(),
}));
vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  api: fake,
}));

import { AlarmIndicator, AlarmsTab } from "./AlarmsTab";

const alarm = (overrides: Partial<AlarmDto>): AlarmDto => ({
  component: "tank",
  kind: "HIGH_LEVEL",
  text: "Tank T-100: level high-high",
  id: "tank.high-level",
  sourceLabel: "Tank T-100",
  category: "PROCESS",
  priority: "ALARM",
  state: "ACTIVE_UNACKNOWLEDGED",
  condition: true,
  activatedUs: "13500000",
  changedUs: "13500000",
  acknowledgments: 0,
  ...overrides,
});
const overflow = alarm({
  kind: "OVERFLOW",
  text: "Tank T-100: overflow",
  id: "tank.overflow",
  priority: "CRITICAL",
  state: "ACTIVE_ACKNOWLEDGED",
  acknowledgments: 1,
});
const high = alarm({});
const active = [overflow, high];
const list = (overrides: Partial<AlarmsDto> = {}): AlarmsDto => ({
  worldTimeUs: "20000000",
  configured: 12,
  alarms: active,
  capacity: 500,
  recorded: "3",
  kept: 3,
  matching: 1,
  history: [
    {
      seq: "2",
      timeUs: "15000000",
      alarm: "tank.overflow",
      code: "OVERFLOW",
      source: "tank",
      sourceLabel: "Tank T-100",
      category: "PROCESS",
      priority: "CRITICAL",
      message: "Tank T-100: overflow",
      transition: "ACKNOWLEDGED",
      from: "ACTIVE_UNACKNOWLEDGED",
      to: "ACTIVE_ACKNOWLEDGED",
    },
  ],
  ...overrides,
});

function lab(alarms: AlarmDto[] = active): Lab {
  // The view reads only these members of the lab.
  return {
    ws: { workspaceId: "1" },
    workspace: { exercise: null },
    frame: frame({ alarms }),
    busy: null,
    act: vi.fn(async (_label: string, work: () => Promise<unknown>) => work()),
  } as unknown as Lab;
}
function inLanguage(language: "en" | "pl", children: ReactNode) {
  const preferences = defaults(language);
  return (
    <Context.Provider
      value={{ preferences, update: vi.fn(), problem: null, theme: "dark" }}
    >
      {children}
    </Context.Provider>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  fake.alarms.mockResolvedValue(list());
  fake.acknowledgeAlarms.mockResolvedValue({ acknowledged: 1, frame: frame() });
});

it("basic mode lists what needs attention and acknowledges only through the backend", () => {
  render(
    inLanguage(
      "en",
      <AlarmsTab lab={lab()} advanced={false} inspect={vi.fn()} />,
    ),
  );
  const table = screen.getByRole("table", { name: "Active alarms" });
  const row = within(table).getByRole("row", { name: /level high-high/ });
  expect(within(row).getByText("13.500 s")).toBeInTheDocument();
  expect(within(row).getByText("present")).toBeInTheDocument();
  expect(within(row).getByText("unacknowledged")).toBeInTheDocument();
  // Acknowledged and still active: no second acknowledgment is offered.
  const critical = within(table).getByRole("row", { name: /overflow/ });
  expect(within(critical).getByText("acknowledged")).toBeInTheDocument();
  expect(
    within(critical).queryByRole("button", { name: "Acknowledge" }),
  ).toBeNull();
  fireEvent.click(within(row).getByRole("button", { name: "Acknowledge" }));
  expect(fake.acknowledgeAlarms).toHaveBeenCalledWith({ workspaceId: "1" }, [
    "tank.high-level",
  ]);
  fireEvent.click(screen.getByRole("button", { name: "Acknowledge all" }));
  expect(fake.acknowledgeAlarms).toHaveBeenLastCalledWith(
    { workspaceId: "1" },
    null,
  );
  // Basic mode keeps the history, filters and selection for advanced mode.
  expect(screen.queryByRole("group", { name: "Alarm filters" })).toBeNull();
  expect(screen.queryByRole("checkbox")).toBeNull();
  expect(screen.queryByRole("table", { name: "Alarm history" })).toBeNull();
  expect(fake.alarms).not.toHaveBeenCalled();
});

it("advanced mode filters in the backend, acknowledges a selection and shows details and history", async () => {
  const inspect = vi.fn();
  render(
    inLanguage("en", <AlarmsTab lab={lab()} advanced inspect={inspect} />),
  );
  const history = await screen.findByRole("table", { name: "Alarm history" });
  expect(within(history).getByText("Acknowledged")).toBeInTheDocument();
  expect(
    screen.getByText(/1 matching of 3 kept transitions/),
  ).toBeInTheDocument();
  // Filters are applied by the backend, to the list and the history alike.
  fireEvent.change(screen.getByLabelText("Filter by priority"), {
    target: { value: "CRITICAL" },
  });
  fireEvent.change(screen.getByLabelText("Filter by state"), {
    target: { value: "ACTIVE_ACKNOWLEDGED" },
  });
  fireEvent.change(screen.getByLabelText("Find a source"), {
    target: { value: "tank" },
  });
  await vi.waitFor(() =>
    expect(fake.alarms).toHaveBeenLastCalledWith(
      { workspaceId: "1" },
      {
        offset: 0,
        limit: 50,
        priority: "CRITICAL",
        state: "ACTIVE_ACKNOWLEDGED",
        source: "tank",
      },
    ),
  );
  // Only alarms waiting for an acknowledgment can be selected.
  const table = screen.getByRole("table", { name: "Active alarms" });
  const waiting = within(table).getByRole("checkbox", {
    name: "Select Tank T-100: level high-high",
  });
  const done = within(table).getByRole("checkbox", {
    name: "Select Tank T-100: overflow",
  });
  expect(done).toBeDisabled();
  const selected = screen.getByRole("button", {
    name: "Acknowledge selected (0)",
  });
  expect(selected).toBeDisabled();
  fireEvent.click(waiting);
  fireEvent.click(
    screen.getByRole("button", { name: "Acknowledge selected (1)" }),
  );
  expect(fake.acknowledgeAlarms).toHaveBeenCalledWith({ workspaceId: "1" }, [
    "tank.high-level",
  ]);
  // Details show the canonical identity and lead to the source.
  const row = within(table).getByRole("row", { name: /level high-high/ });
  fireEvent.click(within(row).getByRole("button", { name: "Details" }));
  const details = screen.getByRole("region", { name: "Alarm details" });
  expect(within(details).getByText("tank.high-level")).toBeInTheDocument();
  expect(
    within(details).getByText("Active, unacknowledged"),
  ).toBeInTheDocument();
  fireEvent.click(
    within(details).getByRole("button", { name: "Inspect source" }),
  );
  expect(inspect).toHaveBeenCalledWith(high);
});

it("states when nothing needs attention or nothing matches the filters", async () => {
  fake.alarms.mockResolvedValue(list({ alarms: [], history: [], matching: 0 }));
  const { unmount } = render(
    inLanguage(
      "en",
      <AlarmsTab lab={lab([])} advanced={false} inspect={vi.fn()} />,
    ),
  );
  expect(screen.getByText("No alarm needs attention.")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Acknowledge all" })).toBeNull();
  unmount();
  render(
    inLanguage("en", <AlarmsTab lab={lab()} advanced inspect={vi.fn()} />),
  );
  fireEvent.change(screen.getByLabelText("Find a source"), {
    target: { value: "pump" },
  });
  // Both the active list and the history report that nothing matches.
  await vi.waitFor(() =>
    expect(screen.getAllByText("No alarm matches the filters.")).toHaveLength(
      2,
    ),
  );
});

it("presents the alarms in Polish", () => {
  render(
    inLanguage(
      "pl",
      <AlarmsTab lab={lab()} advanced={false} inspect={vi.fn()} />,
    ),
  );
  const table = screen.getByRole("table", { name: "Aktywne alarmy" });
  expect(within(table).getByText("poziom bardzo wysoki")).toBeInTheDocument();
  expect(within(table).getByText("niepotwierdzony")).toBeInTheDocument();
  expect(within(table).getByText("Krytyczny")).toBeInTheDocument();
  expect(within(table).getAllByText("występuje")).toHaveLength(2);
  expect(
    within(table).getByRole("button", { name: "Potwierdź" }),
  ).toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: "Potwierdź wszystkie" }),
  ).toBeInTheDocument();
});

it("the basic-mode indicator names the most urgent alarm and opens the alarm view", () => {
  const open = vi.fn();
  const { rerender } = render(
    inLanguage("en", <AlarmIndicator alarms={active} open={open} />),
  );
  const indicator = screen.getByRole("status", {
    name: "Alarms need attention",
  });
  expect(indicator).toHaveTextContent("Alarms needing attention: 2.");
  expect(indicator).toHaveTextContent("Tank T-100: overflow (Critical)");
  fireEvent.click(
    within(indicator).getByRole("button", { name: "Open alarms" }),
  );
  expect(open).toHaveBeenCalled();
  rerender(inLanguage("pl", <AlarmIndicator alarms={active} open={open} />));
  expect(
    screen.getByRole("status", { name: "Alarmy wymagają uwagi" }),
  ).toHaveTextContent("Najpilniejszy: Tank T-100: przelanie (Krytyczny)");
  rerender(inLanguage("en", <AlarmIndicator alarms={[]} open={open} />));
  expect(screen.queryByRole("status")).toBeNull();
});

it("every alarm action is a native control reachable from the keyboard", async () => {
  render(
    inLanguage("en", <AlarmsTab lab={lab()} advanced inspect={vi.fn()} />),
  );
  await screen.findByRole("table", { name: "Alarm history" });
  const controls = [
    ...screen.getAllByRole("button"),
    ...screen.getAllByRole("checkbox"),
    ...screen.getAllByRole("combobox"),
    screen.getByRole("textbox", { name: "Find a source" }),
  ].filter((c) => !(c as HTMLButtonElement).disabled);
  expect(controls.length).toBeGreaterThan(6);
  for (const control of controls) {
    expect(control.tabIndex).toBeGreaterThanOrEqual(0);
    control.focus();
    expect(document.activeElement).toBe(control);
  }
});
