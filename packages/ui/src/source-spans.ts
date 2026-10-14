import type { CompilerSpan } from "@plc-lab/ipc";

export interface EditorRange {
  startLineNumber: number;
  startColumn: number;
  endLineNumber: number;
  endColumn: number;
}

/** Backend one-based Unicode scalar columns -> Monaco one-based UTF-16 columns. */
export function toEditorRange(source: string, span: CompilerSpan): EditorRange {
  const lines = source.split(/\r\n|\n|\r/);
  const convert = (line: number, column: number): number => {
    if (
      !Number.isSafeInteger(line) ||
      !Number.isSafeInteger(column) ||
      line < 1 ||
      column < 1
    )
      throw new Error("Invalid compiler position.");
    const text = lines[line - 1];
    if (text === undefined)
      throw new Error("Compiler line is outside the source.");
    const scalars = Array.from(text);
    if (column > scalars.length + 1)
      throw new Error("Compiler column is outside the source.");
    return scalars.slice(0, column - 1).join("").length + 1;
  };
  if (
    span.end.line < span.start.line ||
    (span.end.line === span.start.line && span.end.column < span.start.column)
  )
    throw new Error("Reversed compiler range.");
  return {
    startLineNumber: span.start.line,
    startColumn: convert(span.start.line, span.start.column),
    endLineNumber: span.end.line,
    endColumn: convert(span.end.line, span.end.column),
  };
}
