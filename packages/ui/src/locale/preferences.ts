/** Presentation preferences only; PLC/project semantics remain in Rust. */
export type Preferences = {
  version: 1;
  language: "pl" | "en";
  theme: "dark";
  density: "comfortable" | "compact";
  explanations: boolean;
};
export const PREFERENCES_KEY = "plc-lab.interface.v1";
export function defaults(language: string): Preferences {
  return {
    version: 1,
    language: /^pl(?:-|$)/i.test(language) ? "pl" : "en",
    theme: "dark",
    density: "comfortable",
    explanations: true,
  };
}
export function parsePreferences(text: string, language: string): Preferences {
  const value: unknown = JSON.parse(text);
  if (!value || typeof value !== "object")
    throw new Error("Invalid preferences");
  const p = value as Record<string, unknown>;
  if (
    p.version !== 1 ||
    !["pl", "en"].includes(String(p.language)) ||
    p.theme !== "dark" ||
    !["comfortable", "compact"].includes(String(p.density)) ||
    typeof p.explanations !== "boolean"
  )
    throw new Error("Unsupported interface preferences");
  return {
    ...defaults(language),
    language: p.language as Preferences["language"],
    density: p.density as Preferences["density"],
    explanations: p.explanations,
  };
}
export function readPreferences(language: string): {
  preferences: Preferences;
  problem: boolean;
} {
  try {
    const text = localStorage.getItem(PREFERENCES_KEY);
    return {
      preferences: text ? parsePreferences(text, language) : defaults(language),
      problem: false,
    };
  } catch {
    return { preferences: defaults(language), problem: true };
  }
}
