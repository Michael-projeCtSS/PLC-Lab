import { expect, it } from "vitest";
import { toEditorRange } from "./source-spans";
it.each(["ASCII", "ą ż", "– —", "😀", "ą ż – 😀"])(
  "maps Unicode scalar columns to UTF-16 for %s",
  (comment) => {
    const prefix = `(* ${comment} *) `,
      source = `${prefix}Missing`;
    const column = Array.from(prefix).length + 1;
    expect(
      toEditorRange(source, {
        start: { line: 1, column },
        end: { line: 1, column: column + 7 },
      }),
    ).toEqual({
      startLineNumber: 1,
      startColumn: prefix.length + 1,
      endLineNumber: 1,
      endColumn: prefix.length + 8,
    });
  },
);
it.each(["\n", "\r\n", "\r"])(
  "handles line ending %j and multiline spans",
  (newline) => {
    expect(
      toEditorRange(`😀${newline}ąA`, {
        start: { line: 1, column: 2 },
        end: { line: 2, column: 3 },
      }),
    ).toEqual({
      startLineNumber: 1,
      startColumn: 3,
      endLineNumber: 2,
      endColumn: 3,
    });
  },
);
it("preserves EOF point locations", () => {
  expect(
    toEditorRange("😀", {
      start: { line: 1, column: 2 },
      end: { line: 1, column: 2 },
    }).startColumn,
  ).toBe(3);
});
it("rejects invalid spans instead of silently moving markers", () => {
  for (const span of [
    { start: { line: 0, column: 1 }, end: { line: 1, column: 1 } },
    { start: { line: 2, column: 1 }, end: { line: 2, column: 1 } },
    { start: { line: 1, column: 3 }, end: { line: 1, column: 3 } },
    { start: { line: 1, column: 2 }, end: { line: 1, column: 1 } },
  ])
    expect(() => toEditorRange("A", span)).toThrow();
});
