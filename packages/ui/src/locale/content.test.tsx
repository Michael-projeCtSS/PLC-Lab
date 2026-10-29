import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import raw from "../../../../knowledge/locales/pl.json";
import { PreferencesProvider, usePreferences } from ".";
import {
  localizeContent,
  searchPolishContent,
  englishSearchAlias,
} from "./content";
import { useLoad } from "../useLoad";
import { defaults, PREFERENCES_KEY } from "./preferences";

const canonical = (item: (typeof raw.items)[number]): Record<string, unknown> =>
  JSON.parse(
    readFileSync(
      join(process.cwd(), "knowledge/PLC_Lab_Knowledge_Pack_v1", item.file),
      "utf8",
    ),
  );
beforeEach(() => localStorage.clear());
it("preserves every non-display value, key, array index, source example and English original", () => {
  for (const item of raw.items) {
    const data = canonical(item);
    const before = JSON.stringify(data);
    const localized = localizeContent(data, "pl");
    const paths = new Map(item.fields.map((f) => [f.path, f]));
    function compare(a: unknown, b: unknown, at: string) {
      if (typeof a === "string")
        expect(b, `${item.id}.${at}`).toBe(paths.get(at)?.text ?? a);
      else if (a && typeof a === "object") {
        expect(Object.keys(b as object)).toEqual(Object.keys(a));
        for (const [key, value] of Object.entries(a))
          compare(
            value,
            (b as Record<string, unknown>)[key],
            at ? `${at}.${key}` : key,
          );
      } else expect(b).toEqual(a);
    }
    compare(data, localized, "");
    expect(JSON.stringify(data)).toBe(before);
    expect(localizeContent(data, "en")).toBe(data);
  }
});
it("missing and stale translations fall back field by field without adding hidden keys", () => {
  const unknown = {
    id: "custom.lesson",
    title: "New content",
    source: "PROGRAM Main",
  };
  expect(localizeContent(unknown, "pl")).toEqual(unknown);
  expect(
    localizeContent({ id: "instruction.ton", title: "New TON title" }, "pl")
      .title,
  ).toBe("New TON title");
  expect(localizeContent(null, "pl")).toBeNull();
});
it("all seven Polish investigations retain the backend repair and debrief gates", () => {
  const cases = raw.items.filter((item) => item.id.startsWith("ts."));
  expect(cases).toHaveLength(7);
  for (const item of cases) {
    const source = canonical(item);
    const brief = source.troubleshooting as Record<string, unknown>;
    const before = {
      id: item.id,
      title: source.title,
      troubleshooting: {
        symptom: brief.symptom,
        repairs: [],
        debrief: null,
        diagnosed: false,
      },
      knowledgeRefs: [
        { id: "fundamentals.digital-io", title: "Digital inputs and outputs" },
      ],
    };
    const shown = localizeContent(before, "pl");
    expect(shown.troubleshooting.repairs).toEqual([]);
    expect(shown.troubleshooting.debrief).toBeNull();
    expect(shown.knowledgeRefs).toEqual(before.knowledgeRefs);
    expect(JSON.stringify(shown)).not.toContain(
      item.fields.find((f) => f.path === "troubleshooting.debrief.rootCause")!
        .text,
    );
    expect(shown).not.toHaveProperty("rootCauses");
    const after = localizeContent(
      { debrief: brief.debrief, criteria: [] },
      "pl",
      item.id,
    );
    expect((after.debrief as { rootCause: string }).rootCause).toBe(
      item.fields.find((f) => f.path === "troubleshooting.debrief.rootCause")!
        .text,
    );
    const unlocked = localizeContent(
      { id: item.id, troubleshooting: { repairs: brief.repairs } },
      "pl",
    );
    const options = unlocked.troubleshooting.repairs as {
      id: string;
      label: string;
    }[];
    for (const [index, option] of options.entries())
      expect(option.label).toBe(
        item.fields.find(
          (f) => f.path === `troubleshooting.repairs.${index}.label`,
        )!.text,
      );
  }
});
it("Polish search includes installed public prose and aliases with deterministic category filtering", () => {
  const available = raw.items.map((item) => ({
    id: item.id,
    title: item.fields.find((f) => f.path === "title")!.source,
    summary: item.fields.find((f) => f.path === "summary")!.source,
  }));
  expect(
    searchPolishContent("opoznione zalaczenie", available, null).some(
      (hit) => hit.id === "instruction.ton",
    ),
  ).toBe(true);
  expect(
    searchPolishContent("opóźnione załączenie", available, "TROUBLESHOOTING"),
  ).toEqual([]);
  expect(searchPolishContent("opóźnione załączenie", [], null)).toEqual([]);
  expect(englishSearchAlias("falownik PLC")).toBe("VFD PLC");
  const query = "żywe zero";
  expect(searchPolishContent(query, available, null)).toEqual(
    searchPolishContent(query, available, null),
  );
  for (const item of raw.items.filter((item) => item.id.startsWith("ts."))) {
    const secret = item.fields.find(
      (f) => f.path === "troubleshooting.debrief.rootCause",
    )!.text;
    expect(searchPolishContent(secret, available, null)).toEqual([]);
  }
});
it("an open lesson, article and exercise change language immediately without fetching or editing content", async () => {
  localStorage.setItem(PREFERENCES_KEY, JSON.stringify(defaults("en")));
  const selected = [
    "lesson.4-20ma",
    "instruction.ton",
    "ex.lamp-pushbutton",
  ].map((id) => raw.items.find((item) => item.id === id)!);
  const data = selected.map(canonical);
  const load = vi.fn(async () => data);
  function OpenContent() {
    const context = usePreferences();
    const loaded = useLoad(load, []);
    return (
      <>
        <button onClick={() => context?.update({ language: "pl" })}>PL</button>
        <button onClick={() => context?.update({ language: "en" })}>EN</button>
        {loaded.data?.map((item) => (
          <h2 key={String(item.id)}>{String(item.title)}</h2>
        ))}
      </>
    );
  }
  render(
    <PreferencesProvider>
      <OpenContent />
    </PreferencesProvider>,
  );
  await screen.findByRole("heading", { name: String(data[0]!.title) });
  fireEvent.click(screen.getByText("PL"));
  for (const item of selected)
    expect(
      screen.getByRole("heading", {
        name: item.fields.find((f) => f.path === "title")!.text,
      }),
    ).toBeInTheDocument();
  fireEvent.click(screen.getByText("EN"));
  for (const item of data)
    expect(
      screen.getByRole("heading", { name: String(item.title) }),
    ).toBeInTheDocument();
  expect(load).toHaveBeenCalledTimes(1);
});
