import { beforeEach, expect, it, vi } from "vitest";
import { en } from "./en";
import { pl } from "./pl";
import { translate } from ".";
import {
  defaults,
  parsePreferences,
  readPreferences,
  PREFERENCES_KEY,
} from "./preferences";

beforeEach(() => {
  localStorage.clear();
  vi.restoreAllMocks();
});
it("English and Polish have identical stable keys and interpolation parameters", () => {
  expect(Object.keys(pl).sort()).toEqual(Object.keys(en).sort());
  for (const key of Object.keys(en) as (keyof typeof en)[]) {
    expect(pl[key].trim()).not.toBe("");
    expect(pl[key].match(/\{\w+\}/g) ?? []).toEqual(
      en[key].match(/\{\w+\}/g) ?? [],
    );
    expect(translate("pl", key)).not.toBe(key);
  }
});
it("a missing Polish entry falls back to English and interpolates values", () => {
  expect(translate("pl", "home.continue", { number: 3 }, {})).toBe(
    "Continue: lesson 3",
  );
  expect(translate("pl", "home.continue", { number: 3 })).toBe(
    "Kontynuuj: lekcja 3",
  );
});
it("locale detection is bounded to Polish or English without overriding saved choices", () => {
  expect(defaults("pl-PL").language).toBe("pl");
  expect(defaults("de-DE").language).toBe("en");
  const saved = { ...defaults("en"), density: "compact", explanations: false };
  expect(parsePreferences(JSON.stringify(saved), "pl-PL")).toEqual(saved);
});
it("corrupt or future settings do not prevent startup and produce a visible problem state", () => {
  for (const text of [
    "broken",
    "null",
    JSON.stringify({ ...defaults("en"), version: 2 }),
    JSON.stringify({ ...defaults("en"), language: "xx" }),
  ]) {
    localStorage.setItem(PREFERENCES_KEY, text);
    expect(readPreferences("pl-PL")).toEqual({
      preferences: defaults("pl-PL"),
      problem: true,
    });
  }
});
it("unavailable browser storage is reported without blocking startup", () => {
  vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
    throw new Error("Storage denied");
  });
  expect(readPreferences("en").problem).toBe(true);
});
