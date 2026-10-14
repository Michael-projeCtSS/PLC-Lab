import type {
  CompileResponse,
  CompilerDiagnostic,
  CompilerSpan,
  CompilerSymbol,
} from "./generated";
import { CoreInfoError } from "./transport";
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
function compileResponseError(): never {
  throw new CoreInfoError(
    "APP-IPC-002",
    "The desktop returned an incompatible Structured Text compilation response.",
  );
}
function keys(value: Record<string, unknown>, expected: string[]) {
  if (Object.keys(value).sort().join(",") !== expected.sort().join(","))
    compileResponseError();
}
function position(value: unknown): { line: number; column: number } {
  if (
    !isRecord(value) ||
    !Number.isSafeInteger(value.line) ||
    !Number.isSafeInteger(value.column) ||
    Number(value.line) < 1 ||
    Number(value.column) < 1
  )
    compileResponseError();
  keys(value, ["line", "column"]);
  return { line: Number(value.line), column: Number(value.column) };
}
export function parseCompilerSpan(value: unknown): CompilerSpan | null {
  if (value === null) return null;
  if (!isRecord(value)) compileResponseError();
  keys(value, ["start", "end"]);
  const start = position(value.start),
    end = position(value.end);
  if (
    end.line < start.line ||
    (end.line === start.line && end.column < start.column)
  )
    compileResponseError();
  return { start, end };
}
export function parseCompileResponse(value: unknown): CompileResponse {
  if (
    !isRecord(value) ||
    typeof value.success !== "boolean" ||
    !Array.isArray(value.diagnostics)
  )
    compileResponseError();
  const diagnostics: CompilerDiagnostic[] = value.diagnostics.map(
    (d: unknown) => {
      if (
        !isRecord(d) ||
        typeof d.code !== "string" ||
        !/^[A-Z0-9]+(?:-[A-Z0-9]+)+$/.test(d.code) ||
        typeof d.message !== "string" ||
        !d.message.trim() ||
        !["error", "warning", "info"].includes(String(d.severity))
      )
        compileResponseError();
      keys(d, ["code", "severity", "message", "span"]);
      return {
        code: d.code,
        severity: d.severity as CompilerDiagnostic["severity"],
        message: d.message,
        span: parseCompilerSpan(d.span),
      };
    },
  );
  if (!value.success) {
    keys(value, ["success", "diagnostics"]);
    if (!diagnostics.some((d) => d.severity === "error"))
      compileResponseError();
    return { success: false, diagnostics };
  }
  keys(value, ["success", "programName", "symbols", "diagnostics"]);
  if (
    typeof value.programName !== "string" ||
    !value.programName ||
    !Array.isArray(value.symbols) ||
    diagnostics.some((d) => d.severity === "error")
  )
    compileResponseError();
  const symbols: CompilerSymbol[] = value.symbols.map((s: unknown) => {
    if (
      !isRecord(s) ||
      typeof s.name !== "string" ||
      !s.name ||
      !["BOOL", "INT", "REAL", "TIME", "TON"].includes(String(s.typeName)) ||
      !["Input", "Output", "Local", "FB Instance"].includes(String(s.role))
    )
      compileResponseError();
    keys(s, ["name", "typeName", "role"]);
    return {
      name: s.name,
      typeName: s.typeName as CompilerSymbol["typeName"],
      role: s.role as CompilerSymbol["role"],
    };
  });
  return {
    success: true,
    programName: value.programName,
    symbols,
    diagnostics,
  };
}
