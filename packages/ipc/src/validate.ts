/**
 * Strict runtime validation combinators for native responses.
 *
 * Every object validator checks the exact key set (mirroring Rust `deny_unknown_fields`),
 * and its output type is tied to the generated contract type, so a drift between Rust
 * DTOs and these validators fails TypeScript compilation as well as at runtime.
 */
import { IpcClientError } from "./transport";

export type Validator<T> = (value: unknown, path: string) => T;

function fail(path: string, expected: string): never {
  throw new IpcClientError(
    "APP-IPC-RESPONSE",
    `The desktop returned an incompatible response (${path}: expected ${expected}).`,
  );
}

export const str: Validator<string> = (v, p) =>
  typeof v === "string" ? v : fail(p, "string");

export const bool: Validator<boolean> = (v, p) =>
  typeof v === "boolean" ? v : fail(p, "boolean");

/** Finite number; REAL values are representation-checked only, never computed. */
export const num: Validator<number> = (v, p) =>
  typeof v === "number" && Number.isFinite(v) ? v : fail(p, "finite number");

export function int(min: number, max: number): Validator<number> {
  return (v, p) =>
    typeof v === "number" && Number.isInteger(v) && v >= min && v <= max
      ? v
      : fail(p, `integer ${min}..${max}`);
}

export const u8 = int(0, 255);
export const u16 = int(0, 65_535);
export const u32 = int(0, 4_294_967_295);
export const i16 = int(-32_768, 32_767);
export const safeInt = int(Number.MIN_SAFE_INTEGER, Number.MAX_SAFE_INTEGER);

const U64_MAX = 18446744073709551615n;
/** Canonical unsigned decimal text for exact 64-bit values. */
export const u64: Validator<string> = (v, p) => {
  if (
    typeof v !== "string" ||
    v.length > 20 ||
    !/^(0|[1-9][0-9]*)$/.test(v) ||
    BigInt(v) > U64_MAX
  )
    fail(p, "canonical u64 decimal string");
  return v;
};

export function literal<const T extends string>(...values: T[]): Validator<T> {
  return (v, p) =>
    typeof v === "string" && (values as string[]).includes(v)
      ? (v as T)
      : fail(p, values.join(" | "));
}

export function nullable<T>(inner: Validator<T>): Validator<T | null> {
  return (v, p) => (v === null ? null : inner(v, p));
}

export function arr<T>(inner: Validator<T>, max = 100_000): Validator<T[]> {
  return (v, p) => {
    if (!Array.isArray(v) || v.length > max) fail(p, `array (max ${max})`);
    return v.map((item, i) => inner(item, `${p}[${i}]`));
  };
}

function record(v: unknown, p: string): Record<string, unknown> {
  if (typeof v !== "object" || v === null || Array.isArray(v))
    fail(p, "object");
  return v as Record<string, unknown>;
}

type Shape<T> = { [K in keyof T]-?: Validator<T[K]> };

export function obj<T>(shape: Shape<T>): Validator<T> {
  const keys = Object.keys(shape).sort();
  return (v, p) => {
    const value = record(v, p);
    const actual = Object.keys(value).sort();
    if (actual.length !== keys.length || actual.some((k, i) => k !== keys[i]))
      fail(p, `fields {${keys.join(", ")}} but got {${actual.join(", ")}}`);
    const out: Record<string, unknown> = {};
    const validators = shape as Record<string, Validator<unknown>>;
    for (const key of keys) {
      const validate = validators[key];
      if (!validate) fail(p, `known field ${key}`);
      out[key] = validate(value[key], `${p}.${key}`);
    }
    return out as T;
  };
}

/** Internally tagged union (`#[serde(tag = "type")]`). */
export function tagged<T extends { type: string }>(variants: {
  [K in T["type"]]: Validator<Extract<T, { type: K }>>;
}): Validator<T> {
  return (v, p) => {
    const tag = record(v, p).type;
    if (typeof tag !== "string" || !(tag in variants))
      fail(`${p}.type`, Object.keys(variants).join(" | "));
    const validate = (variants as Record<string, Validator<T> | undefined>)[
      tag
    ];
    if (!validate) fail(`${p}.type`, Object.keys(variants).join(" | "));
    return validate(v, p);
  };
}

export const unit: Validator<null> = (v, p) =>
  v === null || v === undefined ? null : fail(p, "no value");
