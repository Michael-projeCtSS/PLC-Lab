/**
 * The visual ladder editor (ADR-0052) driven like a student drives it: pointer drags from
 * the palette and on the diagram, typing on elements, completion lists, the instruction
 * picker, keyboard commands and the context menu. Every assertion is on the saved ladder
 * document. jsdom has no layout, so the canvas reports the size the editor gave it and the
 * pointer positions come from the same layout model the editor hit-tests against.
 */
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { library } from "../../../../tests/frontend/fixtures";
import { Context } from "../locale/context";
import { defaults } from "../locale/preferences";
import type { LadDocument, LadNode } from "./ladModel";
import { coilColumn, layoutRung } from "./lad/layout";
import { expression, type Target } from "./lad/placement";
import type { Lab } from "./useLab";

const fake = vi.hoisted(() => ({ library: vi.fn(), ladderState: vi.fn() }));
vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  api: fake,
}));

import { LadEditor } from "./LadEditor";

const NO = (id: number, variable: string): LadNode => ({
  type: "contact",
  id,
  contact: "normallyOpen",
  variable,
});
function doc(items: LadNode[]): LadDocument {
  return {
    format: "plclab-lad",
    version: 2,
    name: "Main",
    variables: [
      {
        name: "StartPB",
        type: "BOOL",
        direction: "input",
        comment: "Start S1",
      },
      { name: "StopPB", type: "BOOL", direction: "input" },
      { name: "Motor", type: "BOOL", direction: "output" },
      { name: "Lamp", type: "BOOL", direction: "output" },
      { name: "Delay", type: "TIME", direction: "local" },
    ],
    rungs: [
      {
        id: 1,
        network: { type: "series", id: 2, items },
        coils: [{ id: 9, variable: "Motor" }],
      },
    ],
  };
}

let saved: string[] = [];
const latest = (): LadDocument => JSON.parse(saved.at(-1) ?? "null");
const logic = () => expression(latest().rungs[0]!.network);

function Harness({ initial }: { initial: LadDocument }) {
  const [source, setSource] = useState(JSON.stringify(initial, null, 2));
  const lab = {
    source,
    setSource: (text: string) => {
      saved.push(text);
      setSource(text);
    },
    diagnostics: [],
    frame: null,
    signals: [],
    loadedSource: null,
    ws: null,
    workspace: null,
    project: {
      station: {
        ioMap: [{ tag: "StartPB", channel: { rack: 0, slot: 1, channel: 0 } }],
      },
    },
  } as unknown as Lab;
  return (
    <Context.Provider
      value={{
        preferences: defaults("en"),
        update: vi.fn(),
        problem: null,
        theme: "dark",
      }}
    >
      <LadEditor lab={lab} />
    </Context.Provider>
  );
}

/** The center of the drop zone for `target` in the (single) rung of `d`. */
function spot(d: LadDocument, target: Target) {
  const layout = layoutRung(d.rungs[0]!, coilColumn(d));
  const zone = layout.zones.find(
    (z) => JSON.stringify(z.target) === JSON.stringify(target),
  );
  if (!zone) throw new Error(`no zone for ${JSON.stringify(target)}`);
  return {
    clientX: zone.box.x + zone.box.w / 2,
    clientY: zone.box.y + zone.box.h / 2,
  };
}
function drag(
  from: HTMLElement,
  to: { clientX: number; clientY: number },
  extra: { ctrlKey?: boolean } = {},
) {
  fireEvent.pointerDown(from, { button: 0, clientX: 900, clientY: 900 });
  fireEvent.pointerMove(window, {
    clientX: to.clientX + 20,
    clientY: to.clientY + 20,
  });
  fireEvent.pointerMove(window, { ...to, ...extra });
  fireEvent.pointerUp(window, to);
}
const lad = () => within(screen.getByTestId("lad-editor"));
const tool = (name: string) =>
  within(screen.getByRole("toolbar", { name: "Ladder tools" })).getByRole(
    "button",
    { name },
  );

const original = HTMLElement.prototype.getBoundingClientRect;
beforeEach(() => {
  saved = [];
  fake.library.mockResolvedValue(library);
  fake.ladderState.mockResolvedValue(null);
  if (!("PointerEvent" in window)) {
    class PointerEventPolyfill extends MouseEvent {
      pointerId: number;
      constructor(type: string, init: PointerEventInit = {}) {
        super(type, init);
        this.pointerId = init.pointerId ?? 1;
      }
    }
    Object.assign(window, { PointerEvent: PointerEventPolyfill });
  }
  HTMLElement.prototype.getBoundingClientRect = function (this: HTMLElement) {
    if (this.classList.contains("lad-canvas")) {
      const width = parseFloat(this.style.width);
      const height = parseFloat(this.style.height);
      return {
        left: 0,
        top: 0,
        right: width,
        bottom: height,
        width,
        height,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      } as DOMRect;
    }
    return original.call(this);
  };
});
afterEach(() => {
  HTMLElement.prototype.getBoundingClientRect = original;
});

it("A: a contact dragged to the right of A is in series and its variable is typed in place", async () => {
  const d = doc([NO(3, "StartPB")]);
  render(<Harness initial={d} />);
  drag(tool("Insert Normally open contact"), spot(d, { zone: "after", id: 3 }));
  expect(logic()).toBe("StartPB AND ?");
  // The new contact is selected and its operand editor is open.
  const input = await screen.findByRole("combobox", {
    name: "Variable of the contact of Normally open contact (variable?)",
  });
  expect(input).toHaveFocus();
  fireEvent.change(input, { target: { value: "sto" } });
  const options = within(screen.getByRole("listbox", { name: "Suggestions" }))
    .getAllByRole("option")
    .map((o) => o.textContent);
  expect(options[0]).toContain("StopPB");
  // TIME variables are not offered to a contact.
  expect(options.join()).not.toContain("Delay");
  fireEvent.keyDown(input, { key: "Enter" });
  expect(logic()).toBe("StartPB AND StopPB");
  // One drop and one accepted edit: two undo steps.
  expect(saved).toHaveLength(2);
});

it("B: dropped below A of A AND B it branches only A: (A OR C) AND B", () => {
  const d = doc([NO(3, "StartPB"), NO(4, "StopPB")]);
  render(<Harness initial={d} />);
  drag(tool("Insert Normally open contact"), spot(d, { zone: "below", id: 3 }));
  expect(logic()).toBe("(StartPB OR ?) AND StopPB");
  expect(
    screen.getByRole("button", { name: "Parallel block of 2 branches" }),
  ).toBeInTheDocument();
});

it("E: an existing contact moved below another keeps its identity; undo and redo are one step", () => {
  const d = doc([NO(3, "StartPB"), NO(4, "StopPB")]);
  render(<Harness initial={d} />);
  const stop = lad().getByRole("button", {
    name: "Normally open contact StopPB",
  });
  drag(stop, spot(d, { zone: "below", id: 3 }));
  expect(logic()).toBe("(StartPB OR StopPB)");
  const block = latest().rungs[0]!.network;
  expect(block.type === "series" && block.items[0]).toMatchObject({
    type: "parallel",
    branches: [{ id: 3 }, { id: 4, variable: "StopPB" }],
  });
  expect(saved).toHaveLength(1);
  const editor = screen.getByTestId("lad-editor");
  fireEvent.keyDown(editor, { key: "z", ctrlKey: true });
  expect(logic()).toBe("StartPB AND StopPB");
  fireEvent.keyDown(editor, { key: "y", ctrlKey: true });
  expect(logic()).toBe("(StartPB OR StopPB)");
});

it("Ctrl+drag copies an element with a new id", () => {
  const d = doc([NO(3, "StartPB")]);
  render(<Harness initial={d} />);
  const start = lad().getByRole("button", {
    name: "Normally open contact StartPB",
  });
  drag(start, spot(d, { zone: "below", id: 3 }), { ctrlKey: true });
  expect(logic()).toBe("(StartPB OR StartPB)");
  // Rung, series, the new parallel block, both contacts and the coil.
  const ids = JSON.stringify(latest()).match(/"id":\d+/g) ?? [];
  expect(ids).toHaveLength(6);
  expect(new Set(ids).size).toBe(ids.length);
});

it("F: an invalid drop explains why and changes nothing; Esc cancels a drag", () => {
  const d = doc([NO(3, "StartPB")]);
  render(<Harness initial={d} />);
  const coil = tool("Add coil");
  const target = spot(d, { zone: "before", id: 3 });
  fireEvent.pointerDown(coil, { button: 0, clientX: 900, clientY: 900 });
  fireEvent.pointerMove(window, {
    clientX: target.clientX + 20,
    clientY: target.clientY,
  });
  fireEvent.pointerMove(window, target);
  expect(
    screen.getAllByText(
      "Coils go in the coil column at the right end of a rung.",
    ).length,
  ).toBeGreaterThan(0);
  fireEvent.pointerUp(window, target);
  expect(saved).toHaveLength(0);
  // A valid target, then Esc: nothing changes either.
  fireEvent.pointerDown(tool("Insert Normally open contact"), {
    button: 0,
    clientX: 900,
    clientY: 900,
  });
  const after = spot(d, { zone: "after", id: 3 });
  fireEvent.pointerMove(window, {
    clientX: after.clientX + 20,
    clientY: after.clientY,
  });
  fireEvent.pointerMove(window, after);
  fireEvent.keyDown(window, { key: "Escape" });
  fireEvent.pointerUp(window, after);
  expect(saved).toHaveLength(0);
});

it("C: typing on a selected contact searches compatible variables; an unknown name needs confirmation", () => {
  const d = doc([NO(3, "")]);
  render(<Harness initial={d} />);
  const contact = lad().getByRole("button", {
    name: "Normally open contact (variable?)",
  });
  fireEvent.click(contact);
  fireEvent.keyDown(contact, { key: "L" });
  const input = screen.getByRole("combobox", {
    name: "Variable of the contact of Normally open contact (variable?)",
  });
  expect(input).toHaveValue("L");
  fireEvent.change(input, { target: { value: "Pump1" } });
  expect(
    within(screen.getByRole("listbox", { name: "Suggestions" }))
      .getAllByRole("option")
      .map((o) => o.textContent),
  ).toEqual([expect.stringContaining("Create variable “Pump1”")]);
  fireEvent.keyDown(input, { key: "Enter" });
  // Nothing is declared before the student confirms type and kind.
  expect(saved).toHaveLength(0);
  const form = screen.getByRole("form", { name: "Declare “Pump1”" });
  expect(within(form).getByRole("combobox", { name: "Type" })).toHaveValue(
    "BOOL",
  );
  expect(within(form).getByRole("combobox", { name: "Kind" })).toHaveValue(
    "local",
  );
  fireEvent.click(within(form).getByRole("button", { name: "Create and use" }));
  expect(latest().variables.at(-1)).toEqual({
    name: "Pump1",
    type: "BOOL",
    direction: "local",
  });
  expect(logic()).toBe("Pump1");
  // Declaring and binding was one step.
  fireEvent.keyDown(screen.getByTestId("lad-editor"), {
    key: "z",
    ctrlKey: true,
  });
  expect(latest().variables.map((v) => v.name)).not.toContain("Pump1");
});

it("D: an instruction typed at an insertion point becomes a real box with pins edited in place", async () => {
  const d = doc([NO(3, "StartPB")]);
  render(<Harness initial={d} />);
  await act(async () => undefined);
  const point = lad().getByRole("button", {
    name: "Insertion point after Normally open contact StartPB",
  });
  fireEvent.click(point);
  fireEvent.keyDown(point, { key: "T" });
  const search = screen.getByRole("combobox", { name: "Instruction name" });
  expect(search).toHaveValue("T");
  fireEvent.change(search, { target: { value: "TON" } });
  const first = within(
    screen.getByRole("listbox", { name: "Instructions" }),
  ).getAllByRole("option")[0]!;
  expect(first).toHaveTextContent("TON");
  expect(first).toHaveTextContent("On-delay timer");
  expect(first).toHaveTextContent("IN: BOOL, PT: TIME → Q: BOOL, ET: TIME");
  fireEvent.keyDown(search, { key: "Enter" });
  expect(logic()).toBe("StartPB AND TON(T1)");
  // The instance name is edited first; Tab goes on to the preset time.
  const instance = screen.getByRole("combobox", {
    name: "Instance name of TON T1",
  });
  fireEvent.change(instance, { target: { value: "RunDelay" } });
  fireEvent.keyDown(instance, { key: "Tab" });
  const preset = screen.getByRole("combobox", {
    name: "Preset time PT of TON RunDelay",
  });
  fireEvent.change(preset, { target: { value: "5" } });
  const offered = within(screen.getByRole("listbox", { name: "Suggestions" }))
    .getAllByRole("option")
    .map((o) => o.textContent);
  expect(offered[0]).toContain("T#5s");
  fireEvent.keyDown(preset, { key: "Enter" });
  const box = latest().rungs[0]!.network;
  expect(box.type === "series" && box.items[1]).toMatchObject({
    type: "block",
    block: "TON",
    instance: "RunDelay",
    preset: { constant: "T#5s" },
  });
  expect(
    lad().getByRole("button", { name: "Preset time PT of TON RunDelay: T#5s" }),
  ).toBeInTheDocument();
});

it("G: keyboard only: F9 inserts, typing binds, Shift+F9 branches, the menu toggles NO/NC", () => {
  const d = doc([NO(3, "StartPB")]);
  render(<Harness initial={d} />);
  const start = lad().getByRole("button", {
    name: "Normally open contact StartPB",
  });
  fireEvent.click(start);
  fireEvent.keyDown(start, { key: "F9" });
  const input = screen.getByRole("combobox", {
    name: /^Variable of the contact of/,
  });
  fireEvent.change(input, { target: { value: "Stop" } });
  fireEvent.keyDown(input, { key: "Enter" });
  expect(logic()).toBe("StartPB AND StopPB");
  const stop = lad().getByRole("button", {
    name: "Normally open contact StopPB",
  });
  fireEvent.keyDown(stop, { key: "F9", shiftKey: true });
  const branch = screen.getByRole("combobox", {
    name: /^Variable of the contact of/,
  });
  fireEvent.change(branch, { target: { value: "Lamp" } });
  fireEvent.keyDown(branch, { key: "ArrowDown" });
  fireEvent.keyDown(branch, { key: "ArrowUp" });
  fireEvent.keyDown(branch, { key: "Enter" });
  expect(logic()).toBe("StartPB AND (StopPB OR Lamp)");
  // Shift+F10 opens the context menu of the selection.
  fireEvent.keyDown(
    lad().getByRole("button", { name: "Normally open contact Lamp" }),
    { key: "F10", shiftKey: true },
  );
  const menu = screen.getByRole("menu", { name: "Element actions" });
  fireEvent.click(
    within(menu).getByRole("menuitem", { name: "Change to normally closed" }),
  );
  expect(logic()).toBe("StartPB AND (StopPB OR NOT Lamp)");
});

it("a coil gets only writable variables; Esc keeps the document", () => {
  const d = doc([NO(3, "StartPB")]);
  render(<Harness initial={d} />);
  const coil = lad().getByRole("button", { name: "Coil Motor" });
  fireEvent.doubleClick(coil);
  const input = screen.getByRole("combobox", {
    name: "Variable of the coil of Coil Motor",
  });
  fireEvent.change(input, { target: { value: "" } });
  const offered = within(screen.getByRole("listbox", { name: "Suggestions" }))
    .getAllByRole("option")
    .map((o) => o.textContent ?? "");
  expect(offered.some((o) => o.includes("StartPB"))).toBe(false);
  expect(offered.some((o) => o.includes("Lamp"))).toBe(true);
  fireEvent.keyDown(input, { key: "Escape" });
  expect(saved).toHaveLength(0);
  expect(
    screen.queryByRole("combobox", {
      name: "Variable of the coil of Coil Motor",
    }),
  ).toBeNull();
});

it("buttons outside the diagram keep their own keys", () => {
  render(<Harness initial={doc([NO(3, "StartPB")])} />);
  const add = screen.getByRole("button", { name: "+ Rung" });
  add.focus();
  // Not prevented: the browser turns Enter and Space on a button into a click.
  expect(fireEvent.keyDown(add, { key: "Enter" })).toBe(true);
  expect(fireEvent.keyDown(add, { key: "a" })).toBe(true);
  expect(fireEvent.keyDown(tool("Undo"), { key: "Delete" })).toBe(true);
  expect(
    screen.queryByRole("combobox", { name: "Instruction name" }),
  ).toBeNull();
  expect(saved).toHaveLength(0);
});
