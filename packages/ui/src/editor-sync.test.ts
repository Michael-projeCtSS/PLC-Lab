import { expect, it } from "vitest";
import { EditorSync } from "./editor-sync";
import { toEditorRange, tryEditorRange } from "./source-spans";

it("a lagging React value is an echo: the newer keystrokes in the model stay", () => {
  const sync = new EditorSync();
  // Enter, then Backspace, typed faster than React re-renders.
  sync.emitted("A\n");
  sync.emitted("A");
  // React's effect runs with the first change while the model already shows the second.
  expect(sync.reconcile("A\n", "A")).toBe("echo");
  expect(sync.reconcile("A", "A")).toBe("in-sync");
});

it("returning to an earlier text keeps the echoes still on their way", () => {
  const sync = new EditorSync();
  // "x", a typed "a", then Backspace: the model is back at "x" while React still has to
  // deliver all three values.
  sync.emitted("x");
  sync.emitted("xa");
  sync.emitted("x");
  expect(sync.reconcile("x", "x")).toBe("in-sync");
  // The deleted "a" must not come back into the model.
  expect(sync.reconcile("xa", "x")).toBe("echo");
  expect(sync.reconcile("x", "x")).toBe("in-sync");
  // React may also skip the intermediate value.
  sync.emitted("xb");
  sync.emitted("x");
  expect(sync.reconcile("x", "x")).toBe("in-sync");
});

it("a program that was opened or replaced is an external change", () => {
  const sync = new EditorSync();
  sync.emitted("PROGRAM Main\nEND_PROGRAM");
  expect(
    sync.reconcile("PROGRAM Other\nEND_PROGRAM", "PROGRAM Main\nEND_PROGRAM"),
  ).toBe("external");
  // After an external change no earlier keystroke counts as an echo any more.
  expect(sync.reconcile("PROGRAM Main\nEND_PROGRAM", "x")).toBe("external");
});

it("echo bookkeeping stays bounded during a long burst of keystrokes", () => {
  const sync = new EditorSync();
  for (let i = 0; i < 1000; i++) sync.emitted(`text ${i}`);
  expect(sync.reconcile("text 999", "text 999")).toBe("in-sync");
  for (let i = 0; i < 1000; i++) sync.emitted(`text ${i}`);
  // Only the most recent keystrokes are remembered.
  expect(sync.reconcile("text 950", "text 999")).toBe("echo");
  expect(sync.reconcile("text 10", "text 999")).toBe("external");
});

it("a diagnostic of an earlier text is skipped instead of throwing", () => {
  const before = "Lamp2 := AutoMode;\nLamp3 := Button2 AND ;";
  const span = { start: { line: 2, column: 22 }, end: { line: 2, column: 23 } };
  expect(tryEditorRange(before, span)).not.toBeNull();
  // Backspace at the start of line 2 joined the lines: line 2 no longer exists.
  const after = "Lamp2 := AutoMode;Lamp3 := Button2 AND ;";
  expect(() => toEditorRange(after, span)).toThrow();
  expect(tryEditorRange(after, span)).toBeNull();
  // Enter before the error shortened the line: the column is past its end.
  const split = "Lamp2 := AutoMode;\nLamp3 := \nButton2 AND ;";
  expect(tryEditorRange(split, span)).toBeNull();
});
