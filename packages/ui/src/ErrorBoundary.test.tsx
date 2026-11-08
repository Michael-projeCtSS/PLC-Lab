import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ErrorBoundary } from "./ErrorBoundary";
import { recentProblems, rememberDraft } from "./error-log";
import { defaults, PREFERENCES_KEY } from "./locale/preferences";

let fail = true;
function Fragile() {
  if (fail) throw new Error("Compiler column is outside the source.");
  return <p>editor works</p>;
}

beforeEach(() => {
  localStorage.clear();
  fail = true;
  vi.spyOn(console, "error").mockImplementation(() => undefined);
  vi.spyOn(console, "warn").mockImplementation(() => undefined);
});
afterEach(() => vi.restoreAllMocks());

it("a view that throws shows the error screen with the program instead of a blank window", () => {
  rememberDraft("p1", "PROGRAM Main\nLamp1 := Button1;\nEND_PROGRAM");
  render(
    <div>
      <nav>shell</nav>
      <ErrorBoundary scope="page">
        <Fragile />
      </ErrorBoundary>
    </div>,
  );
  expect(screen.getByText("shell")).toBeInTheDocument();
  expect(
    screen.getByRole("heading", {
      name: "Part of the interface stopped working",
    }),
  ).toBeInTheDocument();
  expect(screen.getByLabelText("Your program")).toHaveValue(
    "PROGRAM Main\nLamp1 := Button1;\nEND_PROGRAM",
  );
  expect(
    recentProblems().some(
      (p) => p.kind === "render" && p.message.includes("outside the source"),
    ),
  ).toBe(true);
  // "Try again" renders the view anew; it does not pretend the old one still works.
  fail = false;
  fireEvent.click(screen.getByRole("button", { name: "Try again" }));
  expect(screen.getByText("editor works")).toBeInTheDocument();
});

it("another page clears the error screen; a healthy page is not remounted", () => {
  const { rerender } = render(
    <ErrorBoundary scope="page" resetKey="workspace">
      <Fragile />
    </ErrorBoundary>,
  );
  expect(screen.getByRole("alert")).toBeInTheDocument();
  fail = false;
  rerender(
    <ErrorBoundary scope="page" resetKey="home">
      <Fragile />
    </ErrorBoundary>,
  );
  expect(screen.getByText("editor works")).toBeInTheDocument();
  // Navigation inside a page (another lesson) keeps the page's own state.
  const input = () => screen.getByLabelText("search") as HTMLInputElement;
  rerender(
    <ErrorBoundary scope="page" resetKey="learn:a">
      <input aria-label="search" defaultValue="" />
    </ErrorBoundary>,
  );
  fireEvent.change(input(), { target: { value: "timer" } });
  rerender(
    <ErrorBoundary scope="page" resetKey="learn:b">
      <input aria-label="search" defaultValue="" />
    </ErrorBoundary>,
  );
  expect(input().value).toBe("timer");
});

it("the error screen speaks the interface language", () => {
  localStorage.setItem(
    PREFERENCES_KEY,
    JSON.stringify({ ...defaults("pl"), language: "pl" }),
  );
  render(
    <ErrorBoundary scope="app">
      <Fragile />
    </ErrorBoundary>,
  );
  expect(
    screen.getByRole("heading", { name: "Część interfejsu przestała działać" }),
  ).toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: "Uruchom interfejs ponownie" }),
  ).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Spróbuj ponownie" })).toBeNull();
});
