import {
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
  createProject: vi.fn(),
  updateSource: vi.fn(),
  catalog: vi.fn(),
  search: vi.fn(),
}));

vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  api: fake,
  onCycleAdvanced: async () => () => undefined,
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
  fake.buildAndLoad.mockResolvedValue({
    outcome: "LOADED",
    diagnostics: [],
    bindingIssues: [],
    message: "Program Main loaded into the PLC.",
    signals: workspace().signals,
    frame: frame({
      simulation: {
        state: "RUNNING",
        speedPercent: 100,
        runId: "1",
        cycleUs: "100000",
        failure: null,
      },
    }),
  });
});

it("home offers the primary learning paths without internal jargon", async () => {
  render(<App />);
  const nav = screen.getByRole("navigation", { name: "Main" });
  for (const item of [
    "HOME",
    "LEARN",
    "PRACTICE",
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
