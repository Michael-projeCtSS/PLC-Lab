import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { PreferencesProvider, usePreferences } from ".";
import { defaults, parsePreferences, PREFERENCES_KEY } from "./preferences";

type Listener = () => void;
let systemLight = false;
const listeners = new Set<Listener>();

beforeEach(() => {
  localStorage.clear();
  systemLight = false;
  listeners.clear();
  vi.stubGlobal("matchMedia", (query: string) => ({
    media: query,
    get matches() {
      return systemLight;
    },
    addEventListener: (_: string, l: Listener) => listeners.add(l),
    removeEventListener: (_: string, l: Listener) => listeners.delete(l),
  }));
});
afterEach(() => {
  vi.unstubAllGlobals();
  delete document.documentElement.dataset.theme;
});

function Switch() {
  const context = usePreferences()!;
  return (
    <>
      <span>applied:{context.theme}</span>
      {(["dark", "light", "system"] as const).map((theme) => (
        <button key={theme} onClick={() => context.update({ theme })}>
          {theme}
        </button>
      ))}
    </>
  );
}

it("dark, light and system themes are stored and older dark records stay valid", () => {
  const stored = { ...defaults("en"), theme: "light" };
  expect(parsePreferences(JSON.stringify(stored), "en").theme).toBe("light");
  expect(
    parsePreferences(JSON.stringify({ ...stored, theme: "system" }), "en")
      .theme,
  ).toBe("system");
  expect(
    parsePreferences(JSON.stringify({ ...stored, theme: "dark" }), "en").theme,
  ).toBe("dark");
  expect(() =>
    parsePreferences(JSON.stringify({ ...stored, theme: "sepia" }), "en"),
  ).toThrow();
  expect(defaults("pl-PL").theme).toBe("dark");
});

it("the chosen theme is applied to the document at once and persisted", () => {
  render(
    <PreferencesProvider>
      <Switch />
    </PreferencesProvider>,
  );
  expect(document.documentElement.dataset.theme).toBe("dark");
  fireEvent.click(screen.getByText("light"));
  expect(document.documentElement.dataset.theme).toBe("light");
  expect(screen.getByText("applied:light")).toBeInTheDocument();
  expect(JSON.parse(localStorage.getItem(PREFERENCES_KEY)!).theme).toBe(
    "light",
  );
});

it("the system theme follows the operating system and its later changes", () => {
  render(
    <PreferencesProvider>
      <Switch />
    </PreferencesProvider>,
  );
  fireEvent.click(screen.getByText("system"));
  expect(document.documentElement.dataset.theme).toBe("dark");
  act(() => {
    systemLight = true;
    for (const l of listeners) l();
  });
  expect(document.documentElement.dataset.theme).toBe("light");
  fireEvent.click(screen.getByText("dark"));
  expect(document.documentElement.dataset.theme).toBe("dark");
});
