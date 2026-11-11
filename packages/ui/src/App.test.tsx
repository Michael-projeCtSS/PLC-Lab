import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import type { SignalValueDto } from "@plc-lab/ipc";
import {
  frame,
  library,
  project,
  workspace,
} from "../../../tests/frontend/fixtures";

const fake = vi.hoisted(() => ({
  available: () => true,
  coreInfo: vi.fn(),
  knowledgeStatus: vi.fn(),
  library: vi.fn(),
  projects: vi.fn(),
  project: vi.fn(),
  openWorkspace: vi.fn(),
  closeWorkspace: vi.fn(),
  frame: vi.fn(),
  compile: vi.fn(),
  buildAndLoad: vi.fn(),
  simulation: vi.fn(),
  plcMode: vi.fn(),
  operate: vi.fn(),
  controlLoops: vi.fn(),
  watch: vi.fn(),
  modify: vi.fn(),
  trend: vi.fn(),
  ladderState: vi.fn(),
  variables: vi.fn(),
  events: vi.fn(),
  check: vi.fn(),
  diagnose: vi.fn(),
  repair: vi.fn(),
  createProject: vi.fn(),
  updateSource: vi.fn(),
  saveProject: vi.fn(),
  catalog: vi.fn(),
  search: vi.fn(),
  takeOpenRequest: vi.fn(),
  commissioning: vi.fn(),
  noteEvidence: vi.fn(),
}));

vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  api: fake,
  onCycleAdvanced: async () => () => undefined,
  onOpenRequest: async () => () => undefined,
}));

import { App } from "./App";

it("Settings switches PL/EN immediately and preserves language and UI preferences across a restart", async () => {
  const first = render(<App />);
  const trigger = screen.getByRole("button", { name: "Settings" });
  trigger.focus();
  fireEvent.click(trigger);
  expect(screen.getByRole("dialog", { name: "Settings" })).toBeInTheDocument();
  expect(screen.getByLabelText("Language")).toHaveFocus();
  fireEvent.change(screen.getByLabelText("Language"), {
    target: { value: "pl" },
  });
  expect(screen.getByRole("button", { name: "START" })).toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: "LABORATORIUM" }),
  ).toBeInTheDocument();
  expect(
    screen.getByRole("heading", { name: "Ustawienia" }),
  ).toBeInTheDocument();
  expect(document.documentElement.lang).toBe("pl");
  fireEvent.change(screen.getByLabelText("Gęstość interfejsu"), {
    target: { value: "compact" },
  });
  fireEvent.click(screen.getByLabelText("Objaśnienia dla początkujących"));
  first.unmount();
  render(<App />);
  expect(screen.getByRole("button", { name: "START" })).toBeInTheDocument();
  expect(document.querySelector(".app")).toHaveAttribute(
    "data-density",
    "compact",
  );
  expect(document.querySelector(".app")).toHaveAttribute(
    "data-explanations",
    "false",
  );
  const savedTrigger = screen.getByRole("button", { name: "Ustawienia" });
  savedTrigger.focus();
  fireEvent.click(savedTrigger);
  fireEvent.change(screen.getByLabelText("Język"), { target: { value: "en" } });
  fireEvent.keyDown(window, { key: "Escape" });
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(savedTrigger).toHaveFocus();
  expect(screen.getByRole("button", { name: "HOME" })).toBeInTheDocument();
});

it("Settings reports failed persistence instead of pretending preferences were saved", () => {
  render(<App />);
  fireEvent.click(screen.getByRole("button", { name: "Settings" }));
  const save = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
    throw new Error("Quota");
  });
  fireEvent.change(screen.getByLabelText("Language"), {
    target: { value: "pl" },
  });
  expect(screen.getByRole("alert")).toHaveTextContent("Nie udało się zapisać");
  save.mockRestore();
});

it("Basic Free Lab discloses engineering tools through Advanced while retaining PLC and simulation controls", async () => {
  fake.openWorkspace.mockResolvedValue({ ...workspace(), exercise: null });
  render(<App />);
  fireEvent.click(
    await screen.findByRole("button", { name: /Motor start\/stop/ }),
  );
  await screen.findByRole("tab", { name: "Program" });
  for (const name of [
    "Hardware",
    "Network",
    "Diagnostics",
    "Commissioning",
    "Watch",
    "Waveforms",
    "Events",
    "Properties",
  ])
    expect(screen.queryByRole("tab", { name })).toBeNull();
  expect(screen.getByRole("tab", { name: "Process" })).toBeInTheDocument();
  expect(screen.getByRole("tab", { name: "Variables" })).toBeInTheDocument();
  expect(
    screen.getByRole("tab", { name: "Cause & effect" }),
  ).toBeInTheDocument();
  const plc = screen.getByRole("group", { name: "PLC" });
  expect(
    within(plc).getByRole("button", { name: "RUN" }),
  ).toHaveAccessibleDescription(/PLC mode switch RUN/);
  const simulation = screen.getByRole("group", {
    name: "Simulation",
  });
  expect(
    within(simulation).getByRole("button", { name: "Step" }),
  ).toHaveAccessibleDescription(/exactly one PLC cycle/);
  fireEvent.click(screen.getByRole("button", { name: "ADVANCED" }));
  for (const name of [
    "Hardware",
    "Diagnostics",
    "Commissioning",
    "Watch",
    "Waveforms",
    "Events",
    "Properties",
  ])
    expect(screen.getByRole("tab", { name })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("tab", { name: "Hardware" }));
  fireEvent.click(screen.getByRole("button", { name: "BASIC" }));
  expect(screen.getByRole("tab", { name: "Program" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
});

it("offline variable monitoring offers an explanation and the real Build & Load action", async () => {
  fake.frame.mockResolvedValue(
    frame({ plc: { ...frame().plc, programName: null } }),
  );
  render(<App />);
  fireEvent.click(
    await screen.findByRole("button", { name: /Motor start\/stop/ }),
  );
  const heading = await screen.findByRole("heading", {
    name: "No online data",
  });
  const empty = heading.closest(".empty");
  expect(empty).not.toBeNull();
  fireEvent.click(
    within(empty as HTMLElement).getByRole("button", {
      name: "Build & Load to PLC",
    }),
  );
  await waitFor(() => expect(fake.buildAndLoad).toHaveBeenCalled());
});

beforeEach(() => {
  localStorage.clear();
  fake.coreInfo.mockResolvedValue({
    appVersion: "0.2.0-dev",
    buildContext: "development",
    coreVersion: "0.2.0-dev",
    contractVersion: 8,
    execution: "manual-and-continuous",
    sourceCommit: null,
    sourceDirty: null,
    builtAt: null,
    distribution: "DEVELOPMENT_RUN",
    signed: false,
  });
  fake.knowledgeStatus.mockResolvedValue({
    packs: [],
    searchPaths: [],
    indexedItems: 0,
  });
  fake.library.mockResolvedValue(library);
  fake.projects.mockResolvedValue({
    projects: [
      {
        id: "p-1",
        name: "Motor start/stop",
        kind: "exercise",
        exercise: "ex.motor-start-stop",
        platform: "generic.iec",
        scene: "scene.motor-start-stop",
        updatedAt: "2026-10-27T10:00:00Z",
      },
    ],
    problems: [],
    directory: "projects",
  });
  fake.project.mockResolvedValue(project());
  fake.openWorkspace.mockResolvedValue(workspace());
  fake.closeWorkspace.mockResolvedValue(null);
  fake.frame.mockResolvedValue(frame());
  fake.compile.mockResolvedValue({
    success: true,
    programName: "Main",
    diagnostics: [],
    declarations: [],
    pous: [],
  });
  fake.variables.mockResolvedValue({ total: 0, rows: [] });
  fake.events.mockResolvedValue({
    events: [],
    gap: false,
    dropped: "0",
    latest: null,
  });
  fake.catalog.mockResolvedValue({
    vendors: [],
    protocols: [],
    fundamentals: [],
    instructions: [],
    troubleshooting: [],
  });
  fake.search.mockResolvedValue([]);
  fake.takeOpenRequest.mockResolvedValue(null);
  fake.noteEvidence.mockResolvedValue(null);
  fake.controlLoops.mockResolvedValue({ loops: [] });
  fake.ladderState.mockResolvedValue(null);
  fake.trend.mockResolvedValue({ series: [], oldestUs: null, latestUs: null });
  // Like the real backend, every frame read after a successful load reports the loaded,
  // running simulation; the UI re-reads the frame after each use case.
  fake.buildAndLoad.mockImplementation(() => {
    const running = frame({
      simulation: {
        state: "RUNNING",
        speedPercent: 100,
        runId: "1",
        cycleUs: "100000",
        failure: null,
      },
    });
    fake.frame.mockResolvedValue(running);
    return Promise.resolve({
      outcome: "LOADED",
      diagnostics: [],
      bindingIssues: [],
      message: "Program Main loaded into the PLC.",
      signals: workspace().signals,
      frame: running,
    });
  });
});

it("home offers the primary learning paths without internal jargon", async () => {
  render(<App />);
  const nav = screen.getByRole("navigation", { name: "Main" });
  for (const item of [
    "HOME",
    "LEARN",
    "PRACTICE",
    "COMMISSION",
    "TROUBLESHOOT",
    "FREE LAB",
    "KNOWLEDGE",
    "PROGRESS",
  ])
    expect(within(nav).getByRole("button", { name: item })).toBeInTheDocument();
  expect(
    await screen.findByRole("button", { name: "Continue: lesson 2" }),
  ).toBeInTheDocument();
  expect(document.body.textContent).not.toMatch(
    /M4B|IPC v|FOUNDATION|desktop-session|app-contract/,
  );
});

it("practice lists data-driven exercises by kind", async () => {
  render(<App />);
  fireEvent.click(screen.getByRole("button", { name: "PRACTICE" }));
  expect(
    await screen.findByRole("button", {
      name: /Start Motor start\/stop with seal-in/,
    }),
  ).toBeInTheDocument();
  expect(screen.queryByText("Conveyor will not start")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "TROUBLESHOOT" }));
  expect(
    await screen.findByText("Conveyor will not start"),
  ).toBeInTheDocument();
});

it("commissioning labs have their own section", async () => {
  render(<App />);
  fireEvent.click(screen.getByRole("button", { name: "COMMISSION" }));
  expect(
    await screen.findByRole("heading", { name: "Commissioning labs" }),
  ).toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: /Start Commissioning: no input/ }),
  ).toBeInTheDocument();
  expect(screen.queryByText("Conveyor will not start")).toBeNull();
});

it("the commissioning tab shows the checklist the backend evaluated", async () => {
  fake.commissioning.mockResolvedValue({
    feedbackWaitS: 3,
    categories: [
      {
        id: "power",
        title: "Power",
        state: "FAULT",
        items: [
          {
            id: "io-supply",
            title: "I/O field supply",
            state: "FAULT",
            findings: ["DI 16 at rack 0 slot 1: no 24 V field (load) supply"],
            how: "Hardware view: module status.",
            findingMessages: [
              {
                code: "hw.fieldPower",
                text: "DI 16 at rack 0 slot 1: no 24 V field (load) supply",
                args: [],
              },
            ],
            howMessage: {
              code: "check.how.fieldSupply",
              text: "Hardware view: module status.",
              args: [],
            },
          },
        ],
      },
      {
        id: "drives",
        title: "Drives",
        state: "NOT_APPLICABLE",
        items: [
          {
            id: "ready",
            title: "Ready",
            state: "NOT_APPLICABLE",
            findings: ["This machine has no drive."],
            how: "",
            findingMessages: [
              {
                code: "check.noDrive",
                text: "This machine has no drive.",
                args: [],
              },
            ],
            howMessage: null,
          },
        ],
      },
    ],
  });
  render(<App />);
  fireEvent.click(
    await screen.findByRole("button", { name: /Motor start\/stop/ }),
  );
  fireEvent.click(await screen.findByRole("tab", { name: "Commissioning" }));
  const item = await screen.findByLabelText("I/O field supply: FAULT");
  expect(within(item).getByText(/no 24 V field/)).toBeInTheDocument();
  expect(
    within(item).getByText("Hardware view: module status."),
  ).toBeInTheDocument();
  expect(fake.commissioning).toHaveBeenCalled();
  // A category with nothing to check starts collapsed and opens on request.
  const drives = screen.getByRole("button", { name: /DRIVES/ });
  expect(drives).toHaveAttribute("aria-expanded", "false");
  expect(screen.queryByText("This machine has no drive.")).toBeNull();
  fireEvent.click(drives);
  expect(screen.getByText("This machine has no drive.")).toBeInTheDocument();
});

it("opens a workspace and builds in basic mode through one backend use case", async () => {
  render(<App />);
  fireEvent.click(
    await screen.findByRole("button", { name: /Motor start\/stop/ }),
  );
  const build = await screen.findByRole("button", {
    name: "Build & Load to PLC",
  });
  expect(screen.getByText("PLC RUN")).toBeInTheDocument();
  expect(screen.getByText("Sim paused")).toBeInTheDocument();
  fireEvent.click(build);
  await waitFor(() => expect(fake.buildAndLoad).toHaveBeenCalledTimes(1));
  const [, source, , station, runAfterLoad] =
    fake.buildAndLoad.mock.calls[0] ?? [];
  expect(source).toContain("PROGRAM Main");
  expect(station).toBeNull();
  expect(runAfterLoad).toBe(true);
  expect(await screen.findByText("Sim running")).toBeInTheDocument();
});

it("shows the causal chain of one cycle from backend data", async () => {
  render(<App />);
  fireEvent.click(
    await screen.findByRole("button", { name: /Motor start\/stop/ }),
  );
  fireEvent.click(await screen.findByRole("tab", { name: "Cause & effect" }));
  const ce = await screen.findByLabelText("Cycle 11");
  expect(within(ce).getByText(/PLC read/)).toBeInTheDocument();
  expect(within(ce).getByText(/PLC logic result/)).toBeInTheDocument();
  expect(within(ce).getByText(/Output command/)).toBeInTheDocument();
  expect(within(ce).getByText(/Process/)).toBeInTheDocument();
  expect(within(ce).getByText(/Next PLC input/)).toBeInTheDocument();
  expect(within(ce).getByText("StartPB")).toBeInTheDocument();
  expect(within(ce).getByText("FALSE → TRUE")).toBeInTheDocument();
});

// ---- execution controls: one request at a time, newest frame wins -----------------------

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}
const running = () =>
  frame({
    simulation: {
      state: "RUNNING",
      speedPercent: 100,
      runId: "1",
      cycleUs: "100000",
      failure: null,
    },
  });

async function openMotor() {
  render(<App />);
  fireEvent.click(
    await screen.findByRole("button", { name: /Motor start\/stop/ }),
  );
  return screen.findByRole("button", { name: "Run" });
}

it("Run then Pause: the second command waits for the first to finish", async () => {
  const run = deferred<ReturnType<typeof frame>>();
  fake.simulation.mockReturnValueOnce(run.promise);
  const runButton = await openMotor();
  act(() => {
    runButton.click();
    runButton.click();
  });
  expect(fake.simulation).toHaveBeenCalledTimes(1);
  expect(screen.getByRole("button", { name: "Step" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Stop" })).toBeDisabled();
  const readsBefore = fake.frame.mock.calls.length;
  fake.frame.mockResolvedValue(running());
  await act(async () => run.resolve(running()));
  const pause = await screen.findByRole("button", { name: "Pause" });
  await waitFor(() => expect(pause).toBeEnabled());
  expect(fake.frame.mock.calls.length).toBeGreaterThan(readsBefore);
  fake.simulation.mockResolvedValueOnce(frame());
  fake.frame.mockResolvedValue(frame());
  fireEvent.click(pause);
  await waitFor(() => expect(fake.simulation).toHaveBeenCalledTimes(2));
  expect(fake.simulation.mock.calls.map((c) => c[1])).toEqual(["RUN", "PAUSE"]);
});

it("Run then Stop: Stop is unavailable until Run has settled", async () => {
  const run = deferred<ReturnType<typeof frame>>();
  fake.simulation.mockReturnValueOnce(run.promise);
  fireEvent.click(await openMotor());
  const stop = screen.getByRole("button", { name: "Stop" });
  expect(stop).toBeDisabled();
  fireEvent.click(stop);
  expect(fake.simulation).toHaveBeenCalledTimes(1);
  fake.frame.mockResolvedValue(running());
  await act(async () => run.resolve(running()));
  await waitFor(() => expect(stop).toBeEnabled());
  fake.simulation.mockResolvedValueOnce(frame());
  fireEvent.click(stop);
  await waitFor(() =>
    expect(fake.simulation.mock.calls.map((c) => c[1])).toEqual([
      "RUN",
      "STOP",
    ]),
  );
});

it("a double-clicked Step advances exactly one cycle", async () => {
  const step = deferred<ReturnType<typeof frame>>();
  fake.simulation.mockReturnValueOnce(step.promise);
  await openMotor();
  const stepButton = screen.getByRole("button", { name: "Step" });
  // Both clicks arrive before React re-renders the disabled button.
  act(() => {
    stepButton.click();
    stepButton.click();
  });
  expect(fake.simulation).toHaveBeenCalledTimes(1);
  await act(async () => step.resolve(frame()));
  await waitFor(() => expect(stepButton).toBeEnabled());
  expect(fake.simulation).toHaveBeenCalledTimes(1);
});

it("PLC RUN then PLC STOP: the mode switch sends one request at a time", async () => {
  const mode = deferred<ReturnType<typeof frame>>();
  fake.plcMode.mockReturnValueOnce(mode.promise);
  await openMotor();
  const group = screen.getByRole("group", { name: "PLC mode switch" });
  const plcRun = within(group).getByRole("button", { name: "RUN" });
  const plcStop = within(group).getByRole("button", { name: "STOP" });
  act(() => {
    plcStop.click();
    plcRun.click();
  });
  expect(fake.plcMode).toHaveBeenCalledTimes(1);
  expect(plcRun).toBeDisabled();
  await act(async () => mode.resolve(frame()));
  await waitFor(() => expect(plcRun).toBeEnabled());
  expect(fake.plcMode.mock.calls.map((c) => c[1])).toEqual(["STOP"]);
});

it("a failed command re-enables the controls and re-reads the backend", async () => {
  fake.simulation.mockRejectedValueOnce(new Error("Simulation refused"));
  const runButton = await openMotor();
  const readsBefore = fake.frame.mock.calls.length;
  fireEvent.click(runButton);
  await waitFor(() => expect(runButton).toBeEnabled());
  await waitFor(() =>
    expect(fake.frame.mock.calls.length).toBeGreaterThan(readsBefore),
  );
  fireEvent.click(screen.getByRole("tab", { name: /Messages/ }));
  expect(await screen.findByText(/Simulation refused/)).toBeInTheDocument();
});

it("a slow frame read never overwrites the newer frame of a command", async () => {
  const firstRead = deferred<ReturnType<typeof frame>>();
  fake.frame.mockReturnValueOnce(firstRead.promise);
  const runButton = await openMotor();
  await waitFor(() => expect(fake.frame).toHaveBeenCalled());
  // Reads after Run stay pending: only the two responses below can change the frame.
  fake.frame.mockImplementation(() => new Promise(() => undefined));
  fake.simulation.mockResolvedValueOnce(running());
  fireEvent.click(runButton);
  expect(await screen.findByText("Sim running")).toBeInTheDocument();
  // The read sent before Run answers late with the old paused state.
  await act(async () => firstRead.resolve(frame()));
  await waitFor(() => expect(fake.frame.mock.calls.length).toBeGreaterThan(1));
  expect(screen.getByText("Sim running")).toBeInTheDocument();
  expect(screen.queryByText("Sim paused")).not.toBeInTheDocument();
});

// ---- operator gestures: one ordered lane, every press gets its release (ADR-0048) --------

/** A frame whose Start S1 push button reports `pressed`. */
const startPressed = (pressed: boolean) =>
  frame({
    components: [
      {
        id: "start",
        quantities: [
          { name: "pressed", value: { type: "BOOL", value: pressed } },
        ],
      },
      ...frame().components.slice(1),
    ],
  });

/**
 * The backend side of the Start S1 push button: an operate request changes the button
 * when the backend answers it, and every frame (operate answer or read) reports it.
 * `gate()` keeps the next operate request unanswered until the returned function opens
 * it; `refuse()` makes the next one fail without changing anything.
 */
function buttonBackend() {
  const state = { pressed: false };
  const sent: string[] = [];
  const gates: Promise<void>[] = [];
  const refusals: Error[] = [];
  fake.operate.mockReset();
  fake.operate.mockImplementation(
    async (_ws: unknown, component: string, action: { type: string }) => {
      sent.push(`${component} ${action.type}`);
      const gate = gates.shift();
      if (gate) await gate;
      const refusal = refusals.shift();
      if (refusal) throw refusal;
      if (action.type === "press") state.pressed = true;
      if (action.type === "release") state.pressed = false;
      return startPressed(state.pressed);
    },
  );
  fake.frame.mockImplementation(async () => startPressed(state.pressed));
  // A workspace opens with a fresh process: every button released.
  fake.openWorkspace.mockImplementation(async () => {
    state.pressed = false;
    return workspace();
  });
  return {
    state,
    sent,
    gate() {
      const open = deferred<void>();
      gates.push(open.promise);
      return () => act(async () => open.resolve());
    },
    refuse(message: string) {
      refusals.push(new Error(message));
    },
  };
}

async function openStartButton() {
  await openMotor();
  fireEvent.click(screen.getByRole("tab", { name: "Process" }));
  return screen.findByRole("button", { name: "Start S1 (hold to press)" });
}
/** The button state the backend reported in the latest applied frame. */
const reported = (button: HTMLElement, pressed: boolean) =>
  waitFor(() =>
    expect(button).toHaveAttribute("aria-pressed", String(pressed)),
  );

it("press then release reaches the backend in order and leaves the button released", async () => {
  const backend = buttonBackend();
  const button = await openStartButton();
  fireEvent.pointerDown(button, { pointerId: 1 });
  await reported(button, true);
  expect(backend.state.pressed).toBe(true);
  fireEvent.pointerUp(button, { pointerId: 1 });
  await reported(button, false);
  expect(backend.sent).toEqual(["start press", "start release"]);
  expect(backend.state.pressed).toBe(false);
});

it("a release made while the press is pending is sent after it, not dropped", async () => {
  const backend = buttonBackend();
  const button = await openStartButton();
  const answerPress = backend.gate();
  fireEvent.pointerDown(button, { pointerId: 1 });
  fireEvent.pointerUp(button, { pointerId: 1 });
  // The release waits for the press: the backend runs concurrent requests on separate
  // workers, so sending both at once could reverse them.
  await act(async () => undefined);
  expect(backend.sent).toEqual(["start press"]);
  await answerPress();
  await waitFor(() =>
    expect(backend.sent).toEqual(["start press", "start release"]),
  );
  await reported(button, false);
  expect(backend.state.pressed).toBe(false);
});

it("pointercancel releases a held button", async () => {
  const backend = buttonBackend();
  const button = await openStartButton();
  fireEvent.pointerDown(button, { pointerId: 3 });
  await reported(button, true);
  fireEvent.pointerCancel(button, { pointerId: 3 });
  await reported(button, false);
  expect(backend.sent).toEqual(["start press", "start release"]);
  expect(backend.state.pressed).toBe(false);
});

it("a capture loss without pointerup releases a held button", async () => {
  const backend = buttonBackend();
  const button = await openStartButton();
  fireEvent.pointerDown(button, { pointerId: 1 });
  await reported(button, true);
  fireEvent.lostPointerCapture(button, { pointerId: 1 });
  await reported(button, false);
  expect(backend.sent).toEqual(["start press", "start release"]);
  expect(backend.state.pressed).toBe(false);
});

it("a window that loses focus releases a held button", async () => {
  const backend = buttonBackend();
  const button = await openStartButton();
  fireEvent.pointerDown(button, { pointerId: 1 });
  await reported(button, true);
  fireEvent.blur(window);
  await reported(button, false);
  // The pointerup arriving later sends nothing more.
  fireEvent.pointerUp(button, { pointerId: 1 });
  await act(async () => undefined);
  expect(backend.sent).toEqual(["start press", "start release"]);
  expect(backend.state.pressed).toBe(false);
});

it("closing the process view releases a held button; the workspace stays open", async () => {
  const backend = buttonBackend();
  const button = await openStartButton();
  fireEvent.pointerDown(button, { pointerId: 1 });
  await reported(button, true);
  fireEvent.click(
    within(screen.getByRole("tablist", { name: "Workspace views" })).getByRole(
      "tab",
      { name: "Program" },
    ),
  );
  await waitFor(() =>
    expect(backend.sent).toEqual(["start press", "start release"]),
  );
  await waitFor(() => expect(backend.state.pressed).toBe(false));
  expect(fake.closeWorkspace).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("tab", { name: "Process" }));
  await reported(
    await screen.findByRole("button", { name: "Start S1 (hold to press)" }),
    false,
  );
});

it("leaving the workspace releases a held button before the workspace closes", async () => {
  const backend = buttonBackend();
  const button = await openStartButton();
  fireEvent.pointerDown(button, { pointerId: 1 });
  await reported(button, true);
  fireEvent.click(screen.getByRole("button", { name: "HOME" }));
  await waitFor(() => expect(fake.closeWorkspace).toHaveBeenCalledTimes(1));
  expect(backend.sent).toEqual(["start press", "start release"]);
  expect(backend.state.pressed).toBe(false);
  // Same workspace, release first: closing saves the project.
  const release = fake.operate.mock.calls[1] ?? [];
  expect(release[0]).toEqual({ workspaceId: "1" });
  expect(fake.closeWorkspace).toHaveBeenCalledWith({ workspaceId: "1" });
  expect(fake.operate.mock.invocationCallOrder[1]).toBeLessThan(
    fake.closeWorkspace.mock.invocationCallOrder[0] ?? 0,
  );
});

it("a release that fails while the workspace closes is recorded and shown when a workspace opens", async () => {
  const backend = buttonBackend();
  const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
  const button = await openStartButton();
  fireEvent.pointerDown(button, { pointerId: 1 });
  await reported(button, true);
  backend.refuse("The core refused the release.");
  fireEvent.click(screen.getByRole("button", { name: "HOME" }));
  await waitFor(() => expect(fake.closeWorkspace).toHaveBeenCalledTimes(1));
  expect(backend.sent).toEqual(["start press", "start release"]);
  expect(warn).toHaveBeenCalledWith(
    expect.stringContaining(
      "Releasing Start S1 while closing Motor start/stop failed: The core refused the release.",
    ),
  );
  // The next workspace reports it, and its process starts with the button released.
  fireEvent.click(
    await screen.findByRole("button", { name: /Motor start\/stop/ }),
  );
  fireEvent.click(await screen.findByRole("tab", { name: "Process" }));
  await reported(
    await screen.findByRole("button", { name: "Start S1 (hold to press)" }),
    false,
  );
  fireEvent.click(screen.getByRole("tab", { name: /Messages/ }));
  expect(
    await screen.findByText(
      "When Motor start/stop was closed, the release of Start S1 was not confirmed. The reopened project starts with every button released. Reason: The core refused the release.",
    ),
  ).toBeInTheDocument();
  warn.mockRestore();
});

it("a failed press is reported, its release is still sent and later gestures work", async () => {
  const backend = buttonBackend();
  const button = await openStartButton();
  backend.refuse("Operation refused");
  fireEvent.pointerDown(button, { pointerId: 1 });
  fireEvent.pointerUp(button, { pointerId: 1 });
  await waitFor(() =>
    expect(backend.sent).toEqual(["start press", "start release"]),
  );
  expect(backend.state.pressed).toBe(false);
  fireEvent.pointerDown(button, { pointerId: 1 });
  await reported(button, true);
  fireEvent.pointerUp(button, { pointerId: 1 });
  await reported(button, false);
  expect(backend.sent).toEqual([
    "start press",
    "start release",
    "start press",
    "start release",
  ]);
  fireEvent.click(screen.getByRole("tab", { name: /Messages/ }));
  expect(
    await screen.findByText(
      "Start S1: the press was not applied. Operation refused",
    ),
  ).toBeInTheDocument();
});

it("two rapid press-release gestures reach the backend in order", async () => {
  const backend = buttonBackend();
  const button = await openStartButton();
  const answerFirst = backend.gate();
  fireEvent.pointerDown(button, { pointerId: 1 });
  fireEvent.pointerUp(button, { pointerId: 1 });
  fireEvent.pointerDown(button, { pointerId: 1 });
  fireEvent.pointerUp(button, { pointerId: 1 });
  await answerFirst();
  await waitFor(() =>
    expect(backend.sent).toEqual([
      "start press",
      "start release",
      "start press",
      "start release",
    ]),
  );
  await reported(button, false);
  expect(backend.state.pressed).toBe(false);
});

it("overlapping pointers and keys: only the holder that pressed releases", async () => {
  const backend = buttonBackend();
  const button = await openStartButton();
  fireEvent.pointerDown(button, { pointerId: 1 });
  await reported(button, true);
  // A second finger, a key and their releases neither press again nor release.
  fireEvent.pointerDown(button, { pointerId: 2 });
  fireEvent.keyDown(button, { key: " " });
  fireEvent.pointerUp(button, { pointerId: 2 });
  fireEvent.keyUp(button, { key: " " });
  // A right button press is not a press either.
  fireEvent.pointerDown(button, { pointerId: 1, button: 2 });
  await act(async () => undefined);
  expect(backend.sent).toEqual(["start press"]);
  expect(backend.state.pressed).toBe(true);
  fireEvent.pointerUp(button, { pointerId: 1 });
  await reported(button, false);
  expect(backend.sent).toEqual(["start press", "start release"]);
});

it("a repeated pointerup and the capture loss after it send one release", async () => {
  const backend = buttonBackend();
  const button = await openStartButton();
  fireEvent.pointerDown(button, { pointerId: 1 });
  await reported(button, true);
  fireEvent.pointerUp(button, { pointerId: 1 });
  fireEvent.pointerUp(button, { pointerId: 1 });
  fireEvent.lostPointerCapture(button, { pointerId: 1 });
  await reported(button, false);
  await act(async () => undefined);
  expect(backend.sent).toEqual(["start press", "start release"]);
});

it("Space and Enter hold a focused button; repeats are ignored and focus loss releases", async () => {
  const backend = buttonBackend();
  const button = await openStartButton();
  fireEvent.keyDown(button, { key: " " });
  await reported(button, true);
  fireEvent.keyDown(button, { key: " ", repeat: true });
  fireEvent.keyDown(button, { key: "Enter" });
  fireEvent.keyUp(button, { key: "Enter" });
  await act(async () => undefined);
  expect(backend.sent).toEqual(["start press"]);
  fireEvent.keyUp(button, { key: " " });
  await reported(button, false);
  fireEvent.keyDown(button, { key: "Enter" });
  await reported(button, true);
  fireEvent.focusOut(button);
  await reported(button, false);
  expect(backend.sent).toEqual([
    "start press",
    "start release",
    "start press",
    "start release",
  ]);
  expect(backend.state.pressed).toBe(false);
});

it("a Step clicked while a press is pending is sent after the press", async () => {
  const backend = buttonBackend();
  const button = await openStartButton();
  const answerPress = backend.gate();
  fireEvent.pointerDown(button, { pointerId: 1 });
  const stepButton = screen.getByRole("button", { name: "Step" });
  fireEvent.click(stepButton);
  // Accepted (the toolbar waits), but not sent before the press it follows.
  expect(stepButton).toBeDisabled();
  await act(async () => undefined);
  expect(fake.simulation).not.toHaveBeenCalled();
  fake.simulation.mockImplementationOnce(async () =>
    startPressed(backend.state.pressed),
  );
  await answerPress();
  await waitFor(() => expect(fake.simulation).toHaveBeenCalledTimes(1));
  expect(fake.operate.mock.invocationCallOrder[0]).toBeLessThan(
    fake.simulation.mock.invocationCallOrder[0] ?? 0,
  );
  await waitFor(() => expect(stepButton).toBeEnabled());
  await reported(button, true);
  fireEvent.pointerUp(button, { pointerId: 1 });
  await reported(button, false);
  expect(backend.state.pressed).toBe(false);
});

it("a release made during a pending Step is sent after it; the older Step frame does not win", async () => {
  const backend = buttonBackend();
  const step = deferred<ReturnType<typeof frame>>();
  fake.simulation.mockReturnValueOnce(step.promise);
  const button = await openStartButton();
  fireEvent.pointerDown(button, { pointerId: 1 });
  await reported(button, true);
  const stepButton = screen.getByRole("button", { name: "Step" });
  // Toolbar commands keep their one-at-a-time rule: the second click is ignored.
  act(() => {
    stepButton.click();
    stepButton.click();
  });
  expect(fake.simulation).toHaveBeenCalledTimes(1);
  fireEvent.pointerUp(button, { pointerId: 1 });
  await act(async () => undefined);
  expect(backend.sent).toEqual(["start press"]);
  // The Step answers with the button still pressed; the release follows it.
  await act(async () => step.resolve(startPressed(true)));
  await waitFor(() =>
    expect(backend.sent).toEqual(["start press", "start release"]),
  );
  await reported(button, false);
  await waitFor(() => expect(stepButton).toBeEnabled());
  expect(button).toHaveAttribute("aria-pressed", "false");
  expect(backend.state.pressed).toBe(false);
  expect(fake.simulation).toHaveBeenCalledTimes(1);
});

// ---- PID tuning (ADR-0049): backend values, online writes, operator controls -------------

/** The heated vessel scene's controls, and a PID loop the backend reports. */
function tuningBackend(
  values: Record<string, SignalValueDto>,
  signalsRevision = 2,
) {
  const ids: Record<string, number> = {
    MV: 40,
    E: 41,
    P: 42,
    I: 43,
    D: 44,
    SAT: 45,
    SP: 46,
    PV: 47,
    KP: 48,
    TI: 49,
    TD: 50,
    MV_MIN: 51,
    MV_MAX: 52,
    MANUAL: 53,
    MV_MAN: 54,
  };
  const bound: Record<string, number> = { SP: 60, KP: 61, TI: 62, MV_MAX: 63 };
  const base = workspace();
  fake.openWorkspace.mockResolvedValue(
    workspace({
      scene: {
        ...base.scene,
        components: [
          ...base.scene.components,
          {
            id: "sel",
            label: "Selector S1",
            kind: "selectorSwitch",
            color: null,
            positions: ["MAN", "AUTO"],
            operable: true,
            config: [],
          },
          {
            id: "oven",
            label: "Vessel E-1",
            kind: "heatedVessel",
            color: null,
            positions: [],
            operable: true,
            config: [],
          },
        ],
      },
      frame: frame({ signalsRevision }),
    }),
  );
  const shown = frame({
    signalsRevision,
    components: [
      ...frame().components,
      {
        id: "sel",
        quantities: [{ name: "position", value: { type: "COUNT", value: 1 } }],
      },
      {
        id: "oven",
        quantities: [
          { name: "doorOpen", value: { type: "BOOL", value: false } },
        ],
      },
    ],
  });
  fake.frame.mockResolvedValue(shown);
  fake.controlLoops.mockResolvedValue({
    loops: [
      {
        instance: "Loop",
        signals: Object.entries(ids).map(([parameter, signal]) => ({
          parameter,
          signal,
        })),
        bindings: Object.entries(bound).map(([parameter, signal]) => ({
          parameter,
          signal,
        })),
      },
    ],
  });
  const byId = new Map(
    Object.entries(values).map(([name, value]) => [ids[name], value]),
  );
  fake.watch.mockImplementation(async (_ws: unknown, signals: number[]) => ({
    worldTimeUs: "1200000",
    cycle: "12",
    rows: signals.map((signal) => ({
      signal,
      value: byId.get(signal) ?? null,
    })),
  }));
  const point = (t: string, value: number) => ({
    t,
    value: { type: "REAL", value },
  });
  fake.trend.mockResolvedValue({
    series: [
      {
        signal: 46,
        layer: "COMMITTED",
        decimated: false,
        points: [point("0", 60)],
      },
      {
        signal: 47,
        layer: "COMMITTED",
        decimated: false,
        points: [point("0", 20), point("600000", 35), point("1100000", 50.5)],
      },
      {
        signal: 40,
        layer: "COMMITTED",
        decimated: false,
        points: [point("0", 100), point("900000", 38)],
      },
    ],
    oldestUs: "0",
    latestUs: "1200000",
  });
  fake.modify.mockResolvedValue(shown);
  fake.operate.mockReset();
  fake.operate.mockResolvedValue(shown);
  return { ids, bound };
}
const REAL = (value: number): SignalValueDto => ({ type: "REAL", value });
const automatic = {
  SP: REAL(60),
  PV: REAL(50.5),
  E: REAL(9.5),
  MV: REAL(38),
  P: REAL(38),
  I: REAL(0),
  D: REAL(0),
  SAT: { type: "BOOL", value: false },
  KP: REAL(4),
  TI: { type: "TIME", value: "0" },
  TD: { type: "TIME", value: "0" },
  MV_MIN: REAL(0),
  MV_MAX: REAL(100),
  MANUAL: { type: "BOOL", value: false },
  MV_MAN: REAL(20),
} satisfies Record<string, SignalValueDto>;

async function openTuning() {
  await openMotor();
  fireEvent.click(await screen.findByRole("tab", { name: "Tuning" }));
  return screen.findByRole("img", { name: /^Trend of Loop/ });
}
const row = (region: string, label: string) =>
  within(screen.getByRole("region", { name: region }))
    .getByRole("rowheader", { name: label })
    .closest("tr") as HTMLElement;

it("a program without a PID block has no Tuning tab", async () => {
  await openMotor();
  await waitFor(() => expect(fake.controlLoops).toHaveBeenCalled());
  expect(screen.queryByRole("tab", { name: "Tuning" })).not.toBeInTheDocument();
});

it("the Tuning tab shows the controller's trend, state and diagnostics from the backend", async () => {
  tuningBackend(automatic);
  const chart = await openTuning();
  await waitFor(() => expect(chart).toHaveAttribute("data-points", "1,3,2"));
  expect(fake.trend.mock.calls.at(-1)?.[1]).toEqual([46, 47, 40]);
  await waitFor(() =>
    expect(
      within(row("Controller state", "Output MV")).getByText("38.0"),
    ).toBeInTheDocument(),
  );
  expect(
    within(row("Controller state", "Error E = SP - PV")).getByText("9.5"),
  ).toBeInTheDocument();
  // The mode badge, next to the loop name (the selector has an AUTO button too).
  expect(document.querySelector(".tuning .badge.auto")).toHaveTextContent(
    "AUTO",
  );
  const diagnostics = within(
    screen.getByRole("region", { name: "Diagnostics" }),
  );
  expect(
    diagnostics.getByText("AUTO: MV = P + I + D, from the error E = 9.5."),
  ).toBeInTheDocument();
  expect(
    diagnostics.getByText(
      "TI = 0 s: no integral action. A P-only loop keeps a steady error.",
    ),
  ).toBeInTheDocument();
  expect(
    diagnostics.getByText(
      "The simulation is not running: the values change only while it runs.",
    ),
  ).toBeInTheDocument();
});

it("manual mode and a saturated output are explained from the backend's values", async () => {
  tuningBackend({
    ...automatic,
    MANUAL: { type: "BOOL", value: true },
    MV: REAL(100),
    SAT: { type: "BOOL", value: true },
    TI: { type: "TIME", value: "20000000" },
  });
  await openTuning();
  const diagnostics = within(
    screen.getByRole("region", { name: "Diagnostics" }),
  );
  expect(
    await diagnostics.findByText(
      "MAN: the output follows MV_MAN (20.0). The integral tracks it (I = MV - P), so AUTO continues without a bump.",
    ),
  ).toBeInTheDocument();
  expect(
    diagnostics.getByText(
      "The output is at its high limit MV_MAX (100.0): the integral does not grow further (anti-windup).",
    ),
  ).toBeInTheDocument();
  expect(diagnostics.queryByText(/TI = 0 s/)).not.toBeInTheDocument();
  expect(screen.getByText("Output at a limit")).toBeInTheDocument();
  expect(
    within(row("Parameters", "Integral time TI")).getByText("20 s"),
  ).toBeInTheDocument();
});

it("parameters bound to variables are written online; literals are read-only", async () => {
  const { bound } = tuningBackend(automatic);
  await openTuning();
  const kp = row("Parameters", "Gain KP");
  fireEvent.change(
    within(kp).getByRole("textbox", { name: "New value for Gain KP" }),
    {
      target: { value: "2,5" },
    },
  );
  fireEvent.click(within(kp).getByRole("button", { name: "Write" }));
  await waitFor(() =>
    expect(fake.modify).toHaveBeenCalledWith({ workspaceId: "1" }, bound.KP, {
      type: "REAL",
      value: 2.5,
    }),
  );
  const ti = row("Parameters", "Integral time TI");
  fireEvent.change(within(ti).getByRole("textbox"), {
    target: { value: "20" },
  });
  fireEvent.click(within(ti).getByRole("button", { name: "Write" }));
  await waitFor(() =>
    expect(fake.modify).toHaveBeenLastCalledWith(
      { workspaceId: "1" },
      bound.TI,
      { type: "TIME", value: "20000000" },
    ),
  );
  // Not a number: nothing is written.
  const calls = fake.modify.mock.calls.length;
  const max = row("Parameters", "Output high limit MV_MAX");
  const input = within(max).getByRole("textbox");
  fireEvent.change(input, { target: { value: "lots" } });
  fireEvent.click(within(max).getByRole("button", { name: "Write" }));
  expect(input).toHaveAttribute("aria-invalid", "true");
  expect(fake.modify).toHaveBeenCalledTimes(calls);
  // MV_MIN is a literal in the program: no input.
  const min = row("Parameters", "Output low limit MV_MIN");
  expect(within(min).getByText("set in the program")).toBeInTheDocument();
  expect(within(min).queryByRole("textbox")).not.toBeInTheDocument();
  // A setpoint step writes SP relative to its current value.
  fireEvent.click(screen.getByRole("button", { name: "+5" }));
  await waitFor(() =>
    expect(fake.modify).toHaveBeenLastCalledWith(
      { workspaceId: "1" },
      bound.SP,
      { type: "REAL", value: 65 },
    ),
  );
});

it("the Tuning tab operates the MAN/AUTO selector and opens the vessel door as a disturbance", async () => {
  tuningBackend(automatic);
  await openTuning();
  const selector = screen.getByRole("group", { name: "Selector S1" });
  expect(
    within(selector).getByRole("button", { name: "AUTO" }),
  ).toHaveAttribute("aria-pressed", "true");
  fireEvent.click(within(selector).getByRole("button", { name: "MAN" }));
  await waitFor(() =>
    expect(fake.operate).toHaveBeenCalledWith({ workspaceId: "1" }, "sel", {
      type: "selector",
      position: 0,
    }),
  );
  fireEvent.click(
    screen.getByRole("button", { name: "Open the door of Vessel E-1" }),
  );
  await waitFor(() =>
    expect(fake.operate).toHaveBeenLastCalledWith(
      { workspaceId: "1" },
      "oven",
      {
        type: "door",
        open: true,
      },
    ),
  );
});

it("a new download reads the control loops again", async () => {
  tuningBackend(automatic);
  await openTuning();
  const before = fake.controlLoops.mock.calls.length;
  fake.controlLoops.mockResolvedValue({ loops: [] });
  fake.buildAndLoad.mockImplementation(() =>
    Promise.resolve({
      outcome: "LOADED",
      diagnostics: [],
      bindingIssues: [],
      message: "Program Main loaded into the PLC.",
      signals: workspace().signals,
      frame: frame({ signalsRevision: 3 }),
    }),
  );
  fake.frame.mockResolvedValue(frame({ signalsRevision: 3 }));
  fireEvent.click(screen.getByRole("button", { name: "Build & Load to PLC" }));
  await waitFor(() =>
    expect(fake.controlLoops.mock.calls.length).toBeGreaterThan(before),
  );
  await waitFor(() =>
    expect(
      screen.queryByRole("tab", { name: "Tuning" }),
    ).not.toBeInTheDocument(),
  );
});

// ---- Ladder Diagram (ADR-0050, ADR-0051): edited graphically, compiled and observed in Rust

function freeLabBackend() {
  fake.project.mockResolvedValue(project({ kind: "freeLab", exercise: null }));
  fake.openWorkspace.mockResolvedValue(workspace({ exercise: null }));
  let saved: ReturnType<typeof project> | null = null;
  fake.saveProject.mockImplementation(async (p: ReturnType<typeof project>) => {
    saved = p;
    return p;
  });
  return { saved: () => saved };
}
type LadJson = {
  format: string;
  version: number;
  variables: { name: string; type: string; direction: string }[];
  rungs: {
    id: number;
    network: {
      type: string;
      id: number;
      items: Record<string, unknown>[];
    };
    coils: { id: number; variable: string; kind?: string }[];
  }[];
};
const ladSource = () =>
  JSON.parse(
    (fake.buildAndLoad.mock.calls.at(-1)?.[1] as string | undefined) ??
      (fake.compile.mock.calls.at(-1)?.[0] as string),
  ) as LadJson;
const lad = () => within(screen.getByTestId("lad-editor"));
const palette = () =>
  within(screen.getByRole("toolbar", { name: "Ladder tools" }));
const choose = (name: string, value: string) =>
  fireEvent.change(screen.getByRole("combobox", { name }), {
    target: { value },
  });
const declare = (name: string, type: string, direction: string) => {
  fireEvent.change(screen.getByRole("textbox", { name: "New variable name" }), {
    target: { value: name },
  });
  choose("Type of the new variable", type);
  choose("Kind of the new variable", direction);
  fireEvent.click(screen.getByRole("button", { name: "Add variable" }));
};

async function openLadder() {
  const backend = freeLabBackend();
  await openMotor();
  const languages = await screen.findByRole("group", {
    name: "Program language",
  });
  fireEvent.click(
    within(languages).getByRole("button", { name: "Ladder Diagram (LAD)" }),
  );
  fireEvent.click(
    screen.getByRole("button", { name: "Replace with a new LAD program" }),
  );
  await screen.findByTestId("lad-editor");
  return backend;
}

it("a Free Lab program switches to Ladder Diagram with the scene's signals declared", async () => {
  const backend = await openLadder();
  const main = backend.saved()?.programs[0];
  expect(main?.language).toBe("LAD");
  const document = JSON.parse(main?.source ?? "{}") as LadJson;
  expect(document.format).toBe("plclab-lad");
  expect(document.version).toBe(2);
  expect(document.variables).toEqual([
    {
      name: "StartPB",
      type: "BOOL",
      direction: "input",
      comment: "Start button",
    },
  ]);
  // The switch sits in the view tabs, so the editor keeps its full height.
  const tabs = screen.getByRole("tablist", { name: "Workspace views" });
  expect(
    within(tabs).getByRole("group", { name: "Program language" }),
  ).toBeInTheDocument();
  expect(
    within(tabs).getByRole("button", { name: "Ladder Diagram (LAD)" }),
  ).toHaveAttribute("aria-pressed", "true");
});

it("a rung is built from the palette; compiler errors are shown on their element", async () => {
  await openLadder();
  fake.compile.mockResolvedValue({
    success: false,
    programName: null,
    diagnostics: [
      {
        code: "PLC-LAD-003",
        severity: "error",
        message: "Rung 1: an element has no variable.",
        span: null,
        category: "compiler",
        document: null,
        element: 3,
      },
    ],
    declarations: [],
    pous: [],
  });
  fireEvent.click(screen.getByRole("button", { name: "+ Rung" }));
  const problem = await screen.findByRole("button", {
    name: /PLC-LAD-003.*Rung 1: an element has no variable\./,
  });
  // The diagnostic marks its element (id 3, the rung's first contact).
  const contact = lad().getByRole("button", {
    name: "Normally open contact (variable?)",
  });
  await waitFor(() => expect(contact).toHaveAttribute("aria-invalid", "true"));
  expect(contact).toHaveAttribute(
    "title",
    "Rung 1: an element has no variable.",
  );
  // Choosing the problem selects the element.
  fireEvent.click(lad().getByRole("button", { name: "Coil (variable?)" }));
  fireEvent.click(problem);
  await waitFor(() => expect(contact).toHaveAttribute("aria-pressed", "true"));
  declare("MotorContactor", "BOOL", "output");
  choose("Variable of the contact", "StartPB");
  // A parallel branch for the self-holding contact; the new contact is selected.
  fireEvent.click(
    within(screen.getByRole("group", { name: "Insert position" })).getByRole(
      "button",
      { name: "Parallel" },
    ),
  );
  fireEvent.click(
    palette().getByRole("button", { name: "Insert Normally open contact" }),
  );
  choose("Variable of the contact", "MotorContactor");
  fireEvent.click(lad().getByRole("button", { name: "Coil (variable?)" }));
  const coil = screen.getByRole("combobox", { name: "Variable of the coil" });
  // A coil cannot write an input: only outputs and internal variables are offered.
  expect(
    within(coil)
      .getAllByRole("option")
      .map((o) => o.textContent),
  ).toEqual(["(variable?)", "MotorContactor"]);
  fireEvent.change(coil, { target: { value: "MotorContactor" } });
  await waitFor(() =>
    expect(ladSource().rungs[0]?.coils).toEqual([
      expect.objectContaining({ variable: "MotorContactor" }),
    ]),
  );
  expect(ladSource().rungs[0]?.network).toMatchObject({
    type: "series",
    items: [
      {
        type: "parallel",
        branches: [{ variable: "StartPB" }, { variable: "MotorContactor" }],
      },
    ],
  });
});

it("a TON box is placed and configured, and a SET coil latches", async () => {
  await openLadder();
  declare("Waited", "TIME", "local");
  declare("Lamp", "BOOL", "output");
  fireEvent.click(screen.getByRole("button", { name: "+ Rung" }));
  fireEvent.click(
    lad().getByRole("button", { name: "Normally open contact (variable?)" }),
  );
  choose("Variable of the contact", "StartPB");
  fireEvent.click(
    palette().getByRole("button", { name: "Insert TON on-delay timer" }),
  );
  // The new box is selected: its instance, preset and ET output are set next.
  const instance = screen.getByRole("textbox", { name: "Instance name" });
  expect(instance).toHaveValue("T1");
  fireEvent.change(instance, { target: { value: "StartDelay" } });
  fireEvent.blur(instance);
  const preset = screen.getByRole("textbox", {
    name: "Preset time PT: literal",
  });
  fireEvent.change(preset, { target: { value: "T#3s" } });
  fireEvent.keyDown(preset, { key: "Enter" });
  choose("Elapsed time ET to", "Waited");
  fireEvent.click(palette().getByRole("button", { name: "Add SET coil" }));
  choose("Variable of the coil", "Lamp");
  await waitFor(() =>
    expect(ladSource().rungs[0]?.network.items).toEqual([
      expect.objectContaining({ type: "contact", variable: "StartPB" }),
      expect.objectContaining({
        type: "block",
        block: "TON",
        instance: "StartDelay",
        preset: { constant: "T#3s" },
        output: "Waited",
      }),
    ]),
  );
  expect(ladSource().rungs[0]?.coils.at(-1)).toMatchObject({
    variable: "Lamp",
    kind: "set",
  });
  expect(
    lad().getByRole("button", { name: /^TON StartDelay: preset T#3s/ }),
  ).toBeInTheDocument();
  expect(
    lad().getByRole("button", { name: "SET coil Lamp" }),
  ).toBeInTheDocument();
});

it("undo, redo and keyboard editing change only the document", async () => {
  await openLadder();
  fireEvent.click(screen.getByRole("button", { name: "+ Rung" }));
  const contact = lad().getByRole("button", {
    name: "Normally open contact (variable?)",
  });
  fireEvent.click(contact);
  choose("Variable of the contact", "StartPB");
  const named = () =>
    lad().queryByRole("button", { name: "Normally open contact StartPB" });
  expect(named()).toBeInTheDocument();
  // Delete removes the selected element; Ctrl+Z brings it back; Redo removes it again.
  fireEvent.keyDown(named()!, { key: "Delete" });
  expect(named()).not.toBeInTheDocument();
  fireEvent.keyDown(lad().getByRole("button", { name: /^Empty network/ }), {
    key: "z",
    ctrlKey: true,
  });
  expect(named()).toBeInTheDocument();
  fireEvent.click(palette().getByRole("button", { name: "Redo" }));
  expect(named()).not.toBeInTheDocument();
  fireEvent.click(palette().getByRole("button", { name: "Undo" }));
  // Arrow keys move the selection: from the contact to the coil.
  fireEvent.click(named()!);
  fireEvent.keyDown(named()!, { key: "ArrowRight" });
  expect(
    lad().getByRole("button", { name: "Coil (variable?)" }),
  ).toHaveAttribute("aria-pressed", "true");
  // Alt+Arrow moves an element in its series.
  fireEvent.click(named()!);
  fireEvent.click(
    palette().getByRole("button", { name: "Insert Normally closed contact" }),
  );
  choose("Variable of the contact", "StartPB");
  fireEvent.keyDown(
    lad().getByRole("button", { name: "Normally closed contact StartPB" }),
    {
      key: "ArrowLeft",
      altKey: true,
    },
  );
  await waitFor(() =>
    expect(ladSource().rungs[0]?.network.items.map((i) => i.contact)).toEqual([
      "normallyClosed",
      "normallyOpen",
    ]),
  );
});

it("online, the rungs show the power flow, values and timer state the runtime reports", async () => {
  await openLadder();
  fireEvent.click(screen.getByRole("button", { name: "+ Rung" }));
  fireEvent.click(
    lad().getByRole("button", { name: "Normally open contact (variable?)" }),
  );
  choose("Variable of the contact", "StartPB");
  fireEvent.click(
    palette().getByRole("button", { name: "Insert TON on-delay timer" }),
  );
  fireEvent.click(lad().getByRole("button", { name: "Coil (variable?)" }));
  declare("Motor", "BOOL", "output");
  fireEvent.click(lad().getByRole("button", { name: "Coil (variable?)" }));
  choose("Variable of the coil", "Motor");
  expect(
    screen.getByText(
      "Offline: load this program into the PLC and run it to see power flow and values.",
    ),
  ).toBeInTheDocument();
  // Ids: rung 1, series 2, contact 3, coil 4, the TON box 5 (the next free id).
  const signals = [
    ...workspace().signals,
    {
      id: 40,
      path: "plc.T1.ET",
      label: "T1.ET",
      kind: "BLOCK_OUTPUT" as const,
      valueType: "TIME" as const,
      unit: null,
      active: true,
    },
    {
      id: 41,
      path: "plc.Motor",
      label: "Motor",
      kind: "PLC_OUTPUT" as const,
      valueType: "BOOL" as const,
      unit: null,
      active: true,
    },
  ];
  fake.buildAndLoad.mockImplementation(() =>
    Promise.resolve({
      outcome: "LOADED",
      diagnostics: [],
      bindingIssues: [],
      message: "Program Main loaded into the PLC.",
      signals,
      frame: frame(),
    }),
  );
  fake.ladderState.mockResolvedValue({
    cycle: "12",
    scanned: true,
    power: [
      { element: 3, powered: true },
      { element: 5, powered: false },
      { element: 2, powered: false },
    ],
    values: [
      { signal: 10, value: { type: "BOOL", value: true } },
      { signal: 40, value: { type: "TIME", value: "600000" } },
      { signal: 41, value: { type: "BOOL", value: false } },
    ],
  });
  fireEvent.click(screen.getByRole("button", { name: "Build & Load to PLC" }));
  await waitFor(() => expect(fake.buildAndLoad).toHaveBeenCalled());
  expect(ladSource().version).toBe(2);
  expect(
    await screen.findByText(
      "Online, scan 12: power flow and values as the PLC computed them in that scan.",
    ),
  ).toBeInTheDocument();
  const start = await lad().findByRole("button", {
    name: "Normally open contact StartPB: TRUE, power flows",
  });
  expect(start).toHaveClass("powered");
  expect(start).toHaveClass("on");
  const box = lad().getByRole("button", {
    name: /^TON T1: preset T#1s, value/,
  });
  expect(box).not.toHaveClass("powered");
  expect(box.getAttribute("aria-label")).toMatch(/no power$/);
  expect(box).toHaveTextContent(/ET T#600ms/);
  expect(
    lad().getByRole("button", { name: "Coil Motor: FALSE, no power" }),
  ).toBeInTheDocument();
  // All of it came from one ladder_state request per frame, none from a watch table.
  expect(fake.ladderState).toHaveBeenCalledWith({ workspaceId: "1" });
  expect(fake.watch).not.toHaveBeenCalled();
});

it("a PLC that does not scan shows no power flow, only values", async () => {
  await openLadder();
  fireEvent.click(screen.getByRole("button", { name: "+ Rung" }));
  fireEvent.click(
    lad().getByRole("button", { name: "Normally open contact (variable?)" }),
  );
  choose("Variable of the contact", "StartPB");
  fake.ladderState.mockResolvedValue({
    cycle: "30",
    scanned: false,
    power: [],
    values: [{ signal: 10, value: { type: "BOOL", value: true } }],
  });
  fake.buildAndLoad.mockImplementation(() =>
    Promise.resolve({
      outcome: "LOADED",
      diagnostics: [],
      bindingIssues: [],
      message: "Program Main loaded into the PLC.",
      signals: workspace().signals,
      frame: frame({ plc: { ...frame().plc, mode: "STOP" } }),
    }),
  );
  fake.frame.mockResolvedValue(
    frame({ plc: { ...frame().plc, mode: "STOP" } }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Build & Load to PLC" }));
  expect(
    await screen.findByText(
      "The PLC is in STOP and does not scan: no power flows. Inputs show the field now, outputs what the last scan wrote.",
    ),
  ).toBeInTheDocument();
  // The contact shows its input, with no power claim either way.
  const start = await lad().findByRole("button", {
    name: "Normally open contact StartPB: TRUE",
  });
  expect(start).not.toHaveClass("powered");
});

it("PLC and simulation controls explain what they do", async () => {
  await openMotor();
  const group = screen.getByRole("group", { name: "PLC mode switch" });
  expect(
    within(group).getByRole("button", { name: "RUN" }),
  ).toHaveAccessibleDescription(/PLC mode switch RUN/);
  expect(
    within(group).getByRole("button", { name: "STOP" }),
  ).toHaveAccessibleDescription(/simulated machine keeps running/);
  expect(
    screen.getByRole("button", { name: "Run" }),
  ).toHaveAccessibleDescription(/Does not change the PLC mode switch/);
  expect(
    screen.getByRole("button", { name: "Step" }),
  ).toHaveAccessibleDescription(/exactly one PLC cycle/);
  expect(
    screen.getByRole("button", { name: "Stop" }),
  ).toHaveAccessibleDescription(/not the PLC STOP switch/);
});

// ---- troubleshooting: nothing names the cause before the diagnosis ----------------------

const GUIDE = "Stop button contact replaced by a normally open block";
const REPAIR = "Fit a normally closed contact block to the Stop button";
const ROOT_CAUSE_TEXT = "received a normally open contact block";

function troubleshootingWorkspace() {
  const w = workspace();
  if (!w.exercise) throw new Error("fixture has no exercise");
  return workspace({
    exercise: {
      ...w.exercise,
      id: "ts.motor-wont-start",
      title: "Conveyor will not start after panel maintenance",
      kind: "troubleshooting",
      knowledgeRefs: [
        {
          id: "fundamentals.no-nc-contacts",
          title: "Normally open and normally closed contacts",
        },
      ],
      troubleshooting: {
        symptom: "Pressing Start does nothing.",
        reportedBy: "Shift operator",
        diagnosisOptions: [
          {
            id: "start-contact-broken",
            label: "The Start contact does not close",
          },
          {
            id: "stop-contact-no",
            label: "The Stop contact block was replaced",
          },
        ],
        diagnosed: false,
        repairs: [],
      },
    },
  });
}

it("an open troubleshooting lab shows only general concepts and no repair", async () => {
  fake.openWorkspace.mockResolvedValue(troubleshootingWorkspace());
  fake.diagnose.mockResolvedValue({
    correct: true,
    message: "Your diagnosis is consistent with the evidence.",
    attempts: 1,
    repairs: [{ id: "replace-stop-contact", label: REPAIR }],
  });
  render(<App />);
  fireEvent.click(
    await screen.findByRole("button", { name: /Motor start\/stop/ }),
  );
  expect(await screen.findByText("Helpful concepts")).toBeInTheDocument();
  expect(
    screen.getByRole("button", {
      name: "Normally open and normally closed contacts",
    }),
  ).toBeInTheDocument();
  expect(screen.getByText(/unlock the repair step/)).toBeInTheDocument();
  const body = document.body.textContent ?? "";
  expect(body).not.toContain(GUIDE);
  expect(body).not.toContain(REPAIR);
  expect(body).not.toContain(ROOT_CAUSE_TEXT);

  fireEvent.change(screen.getByLabelText("Root cause"), {
    target: { value: "stop-contact-no" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Submit diagnosis" }));
  expect(await screen.findByRole("button", { name: REPAIR })).toBeEnabled();
  expect(document.body.textContent).not.toContain(GUIDE);
});

it("project library documents compile with the program and are added, renamed and deleted", async () => {
  const BLOCKS = "FUNCTION_BLOCK Toggle\nEND_FUNCTION_BLOCK\n";
  const MAIN = "PROGRAM Main\nEND_PROGRAM\n";
  const p = project({
    programs: [
      ...project().programs,
      { id: "lib-1", name: "Blocks", language: "ST", source: BLOCKS },
    ],
  });
  fake.project.mockResolvedValue(p);
  fake.saveProject.mockImplementation((next: typeof p) =>
    Promise.resolve(next),
  );
  fake.compile.mockResolvedValue({
    success: false,
    programName: null,
    diagnostics: [
      {
        code: "PLC-ST-007",
        severity: "error",
        message: "Type mismatch.",
        span: {
          start: { line: 3, column: 1 },
          end: { line: 3, column: 5 },
        },
        category: "compiler",
        document: "lib-1",
        element: null,
      },
    ],
    declarations: [],
    pous: [],
  });
  render(<App />);
  fireEvent.click(
    await screen.findByRole("button", { name: /Motor start\/stop/ }),
  );
  expect(await screen.findByText("Project library")).toBeInTheDocument();
  const blocks = screen.getByRole("button", { name: /Blocks/ });
  // The live compile sends the library with the program.
  await waitFor(() =>
    expect(fake.compile).toHaveBeenCalledWith(MAIN, [
      { id: "lib-1", source: BLOCKS },
    ]),
  );
  // A diagnostic names its document; following it opens that document.
  const problem = await screen.findByRole("button", {
    name: /Blocks · 3: PLC-ST-007/,
  });
  expect(blocks).not.toHaveAttribute("aria-current", "true");
  fireEvent.click(problem);
  expect(blocks).toHaveAttribute("aria-current", "true");
  expect(screen.getByTestId("document-bar")).toHaveTextContent("Blocks");
  // Build & Load sends the library too.
  fireEvent.click(screen.getByRole("button", { name: "Build & Load to PLC" }));
  await waitFor(() => expect(fake.buildAndLoad).toHaveBeenCalledTimes(1));
  expect(fake.buildAndLoad.mock.calls[0]?.[5]).toEqual([
    { id: "lib-1", source: BLOCKS },
  ]);
  // A new document is saved in the project and opened.
  fireEvent.click(
    screen.getByRole("button", { name: "+ New library document" }),
  );
  await waitFor(() => expect(fake.saveProject).toHaveBeenCalledTimes(1));
  const added = fake.saveProject.mock.calls[0]?.[0] as typeof p;
  expect(added.programs.map((d) => [d.id, d.name, d.source])).toEqual([
    ["main", "Main", MAIN],
    ["lib-1", "Blocks", BLOCKS],
    ["lib-2", "Library 2", ""],
  ]);
  await waitFor(() =>
    expect(screen.getByTestId("document-bar")).toHaveTextContent("Library 2"),
  );
  // Rename it.
  fireEvent.click(screen.getByRole("button", { name: "Rename" }));
  fireEvent.change(screen.getByLabelText("Document name"), {
    target: { value: "Functions" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Save" }));
  await waitFor(() => expect(fake.saveProject).toHaveBeenCalledTimes(2));
  expect(
    (fake.saveProject.mock.calls[1]?.[0] as typeof p).programs[2]?.name,
  ).toBe("Functions");
  // Deleting asks once more, then removes only this document.
  fireEvent.click(
    await screen.findByRole("button", { name: "Delete document" }),
  );
  expect(fake.saveProject).toHaveBeenCalledTimes(2);
  fireEvent.click(
    screen.getByRole("button", { name: "Delete Functions permanently" }),
  );
  await waitFor(() => expect(fake.saveProject).toHaveBeenCalledTimes(3));
  expect(
    (fake.saveProject.mock.calls[2]?.[0] as typeof p).programs.map((d) => d.id),
  ).toEqual(["main", "lib-1"]);
  // The main program is shown again.
  await waitFor(() =>
    expect(screen.getByRole("button", { name: /^▤ Main/ })).toHaveAttribute(
      "aria-current",
      "true",
    ),
  );
});

it("the task brief lists the functions and function blocks the exercise requires", async () => {
  const w = workspace();
  if (!w.exercise) throw new Error("fixture has no exercise");
  fake.openWorkspace.mockResolvedValue(
    workspace({
      exercise: {
        ...w.exercise,
        requiredPous: [
          { name: "Toggle", kind: "FUNCTION_BLOCK" },
          { name: "Both", kind: "FUNCTION" },
        ],
      },
    }),
  );
  render(<App />);
  fireEvent.click(
    await screen.findByRole("button", { name: /Motor start\/stop/ }),
  );
  expect(
    await screen.findByText("Required functions and function blocks"),
  ).toBeInTheDocument();
  const rows = within(screen.getByTestId("required-pous")).getAllByRole("row");
  expect(rows.map((r) => r.textContent)).toEqual([
    "ToggleFUNCTION BLOCK",
    "BothFUNCTION",
  ]);
});

it("the debrief names the cause and links its guide only after verification", async () => {
  fake.openWorkspace.mockResolvedValue(troubleshootingWorkspace());
  fake.check.mockResolvedValue({
    outcome: "GRADED",
    score: 100,
    maxScore: 100,
    passed: true,
    criteria: [],
    diagnostics: [],
    problems: [],
    debrief: {
      rootCause: `The Stop button ${ROOT_CAUSE_TEXT} during the service.`,
      evidence: ["StopPB was FALSE at rest."],
      repair: REPAIR,
      reasoning: "Compare every input with its documentation.",
      references: [
        { id: "troubleshooting.nc-contact-replaced-by-no", title: GUIDE },
      ],
    },
  });
  render(<App />);
  fireEvent.click(
    await screen.findByRole("button", { name: /Motor start\/stop/ }),
  );
  expect(document.body.textContent).not.toContain(GUIDE);
  fireEvent.click(await screen.findByRole("button", { name: "Verify repair" }));
  const debrief = await screen.findByLabelText("Debrief");
  expect(
    within(debrief).getByText(/received a normally open/),
  ).toBeInTheDocument();
  expect(
    within(debrief).getByRole("button", { name: GUIDE }),
  ).toBeInTheDocument();
});

// ---- projects requested by a launch -----------------------------------------------------

it("a project requested by the launcher opens its workspace", async () => {
  fake.takeOpenRequest.mockResolvedValueOnce({
    projectId: "p-1",
    name: "Motor start/stop",
    problem: null,
  });
  render(<App />);
  expect(
    await screen.findByRole("button", { name: "Build & Load to PLC" }),
  ).toBeInTheDocument();
  expect(fake.openWorkspace).toHaveBeenCalledWith("p-1");
});

it("a project file that cannot be opened is explained", async () => {
  fake.takeOpenRequest.mockResolvedValueOnce({
    projectId: null,
    name: null,
    problem: "notes.txt is not a PLC Lab project (.plclab) file.",
  });
  render(<App />);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "is not a PLC Lab project",
  );
});
