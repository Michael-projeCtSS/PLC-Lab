import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import {
  frame,
  library,
  project,
  workspace,
} from "../../../tests/frontend/fixtures";

const fake = vi.hoisted(() => ({
  available: () => true,
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
  variables: vi.fn(),
  events: vi.fn(),
  check: vi.fn(),
  diagnose: vi.fn(),
  repair: vi.fn(),
  createProject: vi.fn(),
  updateSource: vi.fn(),
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

beforeEach(() => {
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
