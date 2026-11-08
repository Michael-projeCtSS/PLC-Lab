import { fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { expect, it, vi } from "vitest";
import type {
  AlarmDto,
  ExerciseDto,
  SequenceConditionDto,
  SequenceDefinitionDto,
  SequenceEventDto,
  SequenceStatusDto,
} from "@plc-lab/ipc";
import raw from "../../../../knowledge/PLC_Lab_Knowledge_Pack_v1/exercises/ex-tank-sequence.json";
import { frame, workspace } from "../../../../tests/frontend/fixtures";
import { Context } from "../locale/context";
import { localizeContent } from "../locale/content";
import { defaults } from "../locale/preferences";
import { Inspector } from "./Inspector";
import { SequenceTab } from "./SequenceTab";
import type { Lab } from "./useLab";

type Raw = {
  id: string;
  name: string;
  steps: {
    id: string;
    number: number;
    name: string;
    role?: string;
    timeoutMs?: number;
  }[];
  transitions: {
    id: string;
    from: string;
    to: string;
    name: string;
    permissives?: { id: string; name: string }[];
  }[];
  interlocks: {
    id: string;
    name: string;
    reaction: string;
    steps: string[];
    reactionMs?: number;
  }[];
};
const seq = raw.sequence as unknown as Raw;
/** The shipped tank batch definition as the backend sends it. */
const definition: SequenceDefinitionDto = {
  id: seq.id,
  name: seq.name,
  steps: seq.steps.map((s) => ({
    id: s.id,
    number: s.number,
    name: s.name,
    role: (s.role ??
      "normal") as SequenceDefinitionDto["steps"][number]["role"],
    timeoutMs: s.timeoutMs ?? null,
  })),
  transitions: seq.transitions.map((t) => ({
    id: t.id,
    from: t.from,
    to: t.to,
    name: t.name,
    permissives: (t.permissives ?? []).map((p) => ({ id: p.id, name: p.name })),
  })),
  interlocks: seq.interlocks.map((i) => ({
    id: i.id,
    name: i.name,
    reaction:
      i.reaction as SequenceDefinitionDto["interlocks"][number]["reaction"],
    steps: i.steps,
    reactionMs: i.reactionMs ?? null,
  })),
};

const condition = (
  rows: [string, boolean, boolean | null][],
): SequenceConditionDto => ({
  satisfied: rows.every(([, e, a]) => a === e),
  rows: rows.map(([source, expected, actual]) => ({
    signal: source.toLowerCase(),
    source,
    layer: "inputs",
    op: "eq",
    expected: { type: "bool", value: expected },
    actual: actual === null ? null : { type: "bool", value: actual },
    satisfied: actual === null ? null : actual === expected,
    negated: false,
  })),
});

function status(overrides: Partial<SequenceStatusDto> = {}): SequenceStatusDto {
  return {
    sequenceId: "tank-batch",
    state: "RUNNING",
    activeStep: "fill",
    previousStep: "open-inlet",
    stepValue: 2,
    elapsedUs: "2000000",
    timeoutUs: "30000000",
    fault: null,
    faultStep: null,
    steps: definition.steps.map((s) => ({
      id: s.id,
      state:
        s.id === "fill"
          ? "ACTIVE"
          : s.id === "idle" || s.id === "open-inlet"
            ? "COMPLETED"
            : "INACTIVE",
    })),
    transitions: [
      {
        id: "full",
        fromActive: true,
        condition: condition([["LevelHigh", true, false]]),
        permissives: [],
        blockedBy: [],
      },
    ],
    interlocks: definition.interlocks.map((i) => ({
      id: i.id,
      applies: i.id === "stop-pressed",
      active: false,
      condition: condition([["StopPB", false, true]]),
    })),
    history: [],
    totalEvents: "0",
    ...overrides,
  };
}

const event = (overrides: Partial<SequenceEventDto>): SequenceEventDto => ({
  seq: "1",
  timeUs: "2500000",
  cycle: "25",
  code: "transitionCompleted",
  fault: false,
  step: "fill",
  from: "open-inlet",
  transition: "opened",
  interlock: null,
  permissives: [],
  interlocks: [],
  stepValue: null,
  start: null,
  values: [
    {
      signal: "open",
      source: "InletOpen",
      layer: "inputs",
      value: { type: "bool", value: true },
    },
  ],
  ...overrides,
});

function inLanguage(
  language: "en" | "pl",
  children: ReactNode,
  theme: "dark" | "light" = "dark",
) {
  return (
    <Context.Provider
      value={{
        preferences: defaults(language),
        update: vi.fn(),
        problem: null,
        theme,
      }}
    >
      {children}
    </Context.Provider>
  );
}

function show(
  s: SequenceStatusDto,
  {
    advanced = false,
    alarms = [],
    language = "en",
  }: { advanced?: boolean; alarms?: AlarmDto[]; language?: "en" | "pl" } = {},
) {
  const exercise = {
    ...workspace().exercise,
    id: "ex.tank-sequence",
    sequence: definition,
  } as ExerciseDto;
  const localized = localizeContent(exercise, language).sequence!;
  return render(
    inLanguage(
      language,
      <SequenceTab
        definition={localized}
        status={s}
        alarms={alarms}
        advanced={advanced}
      />,
    ),
  );
}

it("shows the active step, its timer and the condition it waits for from the backend", () => {
  show(status());
  expect(screen.getByTestId("sequence-active")).toHaveTextContent(
    "S2 Fill the tank",
  );
  expect(screen.getByText("RUNNING")).toBeInTheDocument();
  expect(screen.getByTestId("sequence-timer")).toHaveTextContent(
    "2.0 s of 30.0 s, 28.0 s left",
  );
  const waiting = screen.getByRole("region", { name: "Waiting for" });
  expect(within(waiting).getByText("High level reached")).toBeInTheDocument();
  expect(within(waiting).getByText("LevelHigh = TRUE")).toBeInTheDocument();
  expect(within(waiting).getByText(/now FALSE/)).toBeInTheDocument();
  expect(
    within(waiting).getAllByRole("img", { name: "not satisfied" }).length,
  ).toBeGreaterThan(0);
  // Basic mode: no evaluation layers and no history table.
  expect(screen.queryByText(/input image/)).toBeNull();
  expect(screen.queryByRole("region", { name: "Sequence history" })).toBeNull();
});

it("a refused start names the unsatisfied permissive and the blocking interlock", () => {
  show(
    status({
      state: "IDLE",
      activeStep: "idle",
      timeoutUs: null,
      elapsedUs: "0",
      transitions: [
        {
          id: "start",
          fromActive: true,
          condition: condition([["StartPB", true, true]]),
          permissives: [
            {
              id: "stop-released",
              condition: condition([["StopPB", true, true]]),
            },
            {
              id: "hold-released",
              condition: condition([["HoldPB", false, true]]),
            },
          ],
          blockedBy: ["inlet-not-closed"],
        },
      ],
    }),
  );
  expect(screen.getByRole("note")).toHaveTextContent(
    "Not permitted: Hold button released, Inlet valve not closed",
  );
  expect(screen.getByTestId("sequence-explain")).toHaveTextContent(
    "waits in its initial step",
  );
  expect(screen.getByTestId("sequence-timer")).toHaveTextContent(
    "0.0 s (no timeout)",
  );
});

it("a step timeout is a fault with its reason, an expired timer and the related alarm", () => {
  const timeout: AlarmDto = {
    component: "sequence.tank-batch",
    kind: "SEQUENCE_TIMEOUT",
    text: "Tank batch: step timeout",
    id: "sequence.tank-batch.step-timeout",
    sourceLabel: "Tank batch",
    category: "PROCESS",
    priority: "ALARM",
    state: "ACTIVE_UNACKNOWLEDGED",
    condition: true,
    activatedUs: "5500000",
    changedUs: "5500000",
    acknowledgments: 0,
  };
  const { container } = show(
    status({
      state: "FAULTED",
      activeStep: "open-inlet",
      elapsedUs: "6000000",
      timeoutUs: "5000000",
      fault: "stepTimeout",
      faultStep: "open-inlet",
      steps: definition.steps.map((s) => ({
        id: s.id,
        state: s.id === "open-inlet" ? "FAULTED" : "INACTIVE",
      })),
      transitions: [
        {
          id: "opened",
          fromActive: true,
          condition: condition([["InletOpen", true, false]]),
          permissives: [],
          blockedBy: [],
        },
      ],
    }),
    { alarms: [timeout] },
  );
  expect(screen.getByText("FAULTED")).toBeInTheDocument();
  expect(screen.getByTestId("sequence-explain")).toHaveTextContent(
    "Fault in S1 Open the inlet valve: step timeout.",
  );
  expect(screen.getByTestId("sequence-timer")).toHaveTextContent(
    "6.0 s of 5.0 s, 0.0 s left",
  );
  expect(container.querySelector("progress.seq-timer.expired")).not.toBeNull();
  const related = screen.getByRole("region", { name: "Related alarms" });
  expect(related).toHaveTextContent("Tank batch: step timeout");
});

it("advanced mode adds actual values, evidence layers, interlock reactions and the history", () => {
  show(
    status({
      history: [
        event({ seq: "1" }),
        event({
          seq: "2",
          code: "stepTimeout",
          fault: true,
          step: "fill",
          from: null,
          transition: null,
          values: [],
          timeUs: "32500000",
        }),
      ],
      totalEvents: "2",
    }),
    { advanced: true },
  );
  expect(screen.getAllByText(/input image/).length).toBeGreaterThan(0);
  expect(screen.getByText("Previous step")).toBeInTheDocument();
  expect(
    screen.getByText(
      "Aborts: the program must return to the initial step within 200 ms",
    ),
  ).toBeInTheDocument();
  const history = screen.getByRole("region", { name: "Sequence history" });
  const rows = within(history).getAllByRole("row");
  // Newest first; the fault is marked.
  expect(rows[1]).toHaveTextContent("Step timeout in S2 Fill the tank");
  expect(rows[1]).toHaveClass("seq-fault");
  expect(rows[2]).toHaveTextContent(
    "S1 Open the inlet valve → S2 Fill the tank: Inlet valve reports open",
  );
  expect(rows[2]).toHaveTextContent("InletOpen = TRUE");
  expect(history).toHaveTextContent("(2 events)");
});

it("speaks Polish: content names from the overlay and interface labels", () => {
  show(status({ state: "HELD" }), { language: "pl" });
  expect(screen.getByText("WSTRZYMANA")).toBeInTheDocument();
  expect(screen.getByTestId("sequence-active")).toHaveTextContent(
    "S2 Napełnianie zbiornika",
  );
  expect(screen.getByTestId("sequence-explain")).toHaveTextContent(
    "Program wstrzymał S2 Napełnianie zbiornika",
  );
  const waiting = screen.getByRole("region", { name: "Oczekiwanie na" });
  expect(
    within(waiting).getByText("Osiągnięty poziom górny"),
  ).toBeInTheDocument();
  expect(screen.getByText("Wciśnięty Stop")).toBeInTheDocument();
  expect(screen.queryByText("Fill the tank")).toBeNull();
});

it("step states are told by words and shape, not by colour alone, in both themes", () => {
  for (const theme of ["dark", "light"] as const) {
    const { container, unmount } = render(
      inLanguage(
        "en",
        <SequenceTab
          definition={definition}
          status={status({
            steps: [
              { id: "idle", state: "COMPLETED" },
              { id: "open-inlet", state: "COMPLETED" },
              { id: "fill", state: "HELD" },
              { id: "close-inlet", state: "INACTIVE" },
              { id: "drain", state: "INACTIVE" },
              { id: "complete", state: "INACTIVE" },
            ],
          })}
          alarms={[]}
          advanced={false}
        />,
        theme,
      ),
    );
    const chart = screen.getByRole("region", { name: "Sequence chart" });
    const steps = within(chart).getAllByRole("listitem");
    expect(steps[0]).toHaveClass("completed");
    expect(steps[0]).toHaveTextContent("completed");
    expect(steps[2]).toHaveClass("held");
    expect(steps[2]).toHaveTextContent("held");
    expect(container.querySelector('[aria-current="step"]')).toHaveTextContent(
      "S2",
    );
    unmount();
  }
});

it("the Sequence tab is a keyboard-focusable tab that opens the inspector", () => {
  const exercise = {
    ...workspace().exercise,
    id: "ex.tank-sequence",
    sequence: definition,
  } as ExerciseDto;
  const lab = {
    workspace: { ...workspace(), exercise },
    frame: frame({ sequence: status() }),
    report: null,
    signals: [],
  } as unknown as Lab;
  render(
    inLanguage(
      "en",
      <Inspector lab={lab} selection={null} navigate={vi.fn()} advanced />,
    ),
  );
  const tab = screen.getByRole("tab", { name: "Sequence" });
  expect(tab.tagName).toBe("BUTTON");
  tab.focus();
  expect(tab).toHaveFocus();
  fireEvent.click(tab);
  expect(tab).toHaveAttribute("aria-selected", "true");
  expect(screen.getByTestId("sequence-active")).toHaveTextContent(
    "S2 Fill the tank",
  );
  expect(
    screen.getByRole("region", { name: "Sequence history" }),
  ).toBeVisible();
});

it("shows nothing to evaluate before the first frame and explains an unbound program", () => {
  const { rerender } = render(
    inLanguage(
      "en",
      <SequenceTab
        definition={definition}
        status={null}
        alarms={[]}
        advanced={false}
      />,
    ),
  );
  expect(
    screen.getByText(/appears once the workspace is running/),
  ).toBeVisible();
  rerender(
    inLanguage(
      "en",
      <SequenceTab
        definition={definition}
        status={status({
          state: "UNBOUND",
          activeStep: null,
          transitions: [],
          steps: definition.steps.map((s) => ({ id: s.id, state: "INACTIVE" })),
        })}
        alarms={[]}
        advanced={false}
      />,
    ),
  );
  expect(screen.getByText("NO STEP VARIABLE")).toBeVisible();
  expect(screen.getByTestId("sequence-explain")).toHaveTextContent(
    "does not provide the step variable",
  );
});
