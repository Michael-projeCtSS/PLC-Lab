import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import type { PouDto } from "@plc-lab/ipc";
import { library } from "../../../../tests/frontend/fixtures";
import { Context } from "../locale/context";
import { defaults } from "../locale/preferences";

const fake = vi.hoisted(() => ({ library: vi.fn() }));
vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  api: fake,
}));

import { LibraryTab } from "./LibraryTab";

const pous: PouDto[] = [
  {
    name: "MotorControl",
    kind: "FUNCTION_BLOCK",
    returnType: null,
    parameters: [
      {
        name: "Start",
        dataType: "BOOL",
        direction: "Input",
        description: "Start command",
      },
      { name: "Stop", dataType: "BOOL", direction: "Input", description: null },
      {
        name: "Running",
        dataType: "BOOL",
        direction: "Output",
        description: "Contactor",
      },
    ],
    description: "Start/stop with seal-in.",
    line: 2,
    document: null,
  },
  {
    name: "Clamp",
    kind: "FUNCTION",
    returnType: "INT",
    parameters: [
      { name: "Value", dataType: "INT", direction: "Input", description: null },
      { name: "Low", dataType: "INT", direction: "Input", description: null },
    ],
    description: null,
    line: 20,
    document: "lib-1",
  },
];

function show(language: "en" | "pl", current = true) {
  return render(
    <Context.Provider
      value={{
        preferences: defaults(language),
        update: vi.fn(),
        problem: null,
        theme: "dark",
      }}
    >
      <LibraryTab
        pous={pous}
        current={current}
        documentName={(id) => (id === "lib-1" ? "Utilities" : null)}
      />
    </Context.Provider>,
  );
}

beforeEach(() => fake.library.mockResolvedValue(library));

it("lists the program's blocks and functions with signatures, parameters and examples", async () => {
  show("en");
  const motor = screen.getByTestId("lib-MotorControl");
  expect(motor).toHaveTextContent(
    "MotorControl(Start : BOOL, Stop : BOOL) => Running : BOOL",
  );
  expect(motor).toHaveTextContent("Start/stop with seal-in.");
  expect(within(motor).getByText("Start command")).toBeInTheDocument();
  expect(motor).toHaveTextContent(
    "MotorControl1(Start := StartVar, Stop := StopVar, Running => RunningOut);",
  );
  const clamp = screen.getByTestId("lib-Clamp");
  expect(clamp).toHaveTextContent("Clamp(Value : INT, Low : INT) : INT");
  expect(clamp).toHaveTextContent("Result := Clamp(Value, Low);");
  expect(clamp).toHaveTextContent("No description");
  // Where each block is defined: the program or a project library document.
  expect(motor).toHaveTextContent("Line 2 of the program");
  expect(clamp).toHaveTextContent("Line 20 of the library document Utilities");
  // The standard blocks come from the backend's signatures.
  const ton = await screen.findByTestId("lib-TON");
  expect(ton).toHaveTextContent(
    "TON(IN : BOOL, PT : TIME) => Q : BOOL, ET : TIME",
  );
  expect(ton).toHaveTextContent("Delay(IN := StartPB, PT := T#5s);");
  expect(screen.queryByRole("note")).toBeNull();
});

it("filters by name and says when the shown list is from an older compile", async () => {
  show("en", false);
  expect(screen.getByRole("note")).toHaveTextContent(
    "does not compile right now",
  );
  fireEvent.change(screen.getByRole("searchbox"), {
    target: { value: "clam" },
  });
  expect(screen.queryByTestId("lib-MotorControl")).toBeNull();
  expect(screen.getByTestId("lib-Clamp")).toBeInTheDocument();
  expect(screen.queryByTestId("lib-TON")).toBeNull();
});

it("speaks Polish", async () => {
  show("pl");
  expect(
    screen.getByRole("region", {
      name: "Biblioteka użytkownika (ten program)",
    }),
  ).toBeInTheDocument();
  const ton = await screen.findByTestId("lib-TON");
  expect(ton).toHaveTextContent("Timer opóźnionego załączenia");
  expect(
    within(screen.getByTestId("lib-MotorControl")).getAllByText("wejście"),
  ).toHaveLength(2);
});
