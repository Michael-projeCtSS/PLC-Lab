/** Values of the PID tuning screen (ADR-0049); presentation only, no control action. */
import type { SignalValueDto } from "@plc-lab/ipc";

export type ParameterKind = "REAL" | "TIME" | "BOOL";

export function real(value: SignalValueDto | null | undefined): number | null {
  return value?.type === "REAL" ? value.value : null;
}

/** A TIME value in seconds. */
export function timeSeconds(
  value: SignalValueDto | null | undefined,
): number | null {
  return value?.type === "TIME" ? Number(BigInt(value.value)) / 1e6 : null;
}

export function flag(value: SignalValueDto | null | undefined): boolean | null {
  return value?.type === "BOOL" ? value.value : null;
}

/** Longest integral or derivative time the screen accepts: one day. */
const MAX_SECONDS = 86_400;

/**
 * The typed value an engineer entered for a REAL or TIME parameter (TIME in seconds, a
 * decimal comma allowed), or `null` when it is not one. The backend checks the type
 * again when it writes the variable.
 */
export function parseParameter(
  kind: ParameterKind,
  text: string,
): SignalValueDto | null {
  const trimmed = text.trim().replace(",", ".");
  if (trimmed === "" || !/^[-+]?\d*\.?\d+(?:[eE][-+]?\d+)?$/.test(trimmed))
    return null;
  const number = Number(trimmed);
  if (!Number.isFinite(number)) return null;
  if (kind === "REAL") return { type: "REAL", value: number };
  if (kind === "TIME")
    return number < 0 || number > MAX_SECONDS
      ? null
      : { type: "TIME", value: String(Math.round(number * 1e6)) };
  return null;
}
