import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { auditUi } from "../../scripts/ui-locale-audit.mjs";
import { root } from "../../scripts/toolchain.mjs";
import {
  canonicalItems,
  LOCALES,
  textFields,
  validateLocale,
} from "../../scripts/content-locales.mjs";

const locale = JSON.parse(readFileSync(path.join(LOCALES, "pl.json"), "utf8"));
test("the Polish overlay validates all declared complete categories and reports all gaps", () => {
  const report = validateLocale(locale);
  assert.deepEqual(report.errors, []);
  assert.deepEqual(
    report.coverage.map(({ group, total, catalogue, complete }) => [
      group,
      total,
      catalogue,
      complete,
    ]),
    [
      ["exercises", 46, 46, 17],
      ["lessons", 22, 22, 22],
      ["fundamentals", 23, 0, 0],
      ["instructions", 7, 7, 7],
      ["protocols", 9, 0, 0],
      ["troubleshooting", 29, 0, 0],
      ["vendors", 24, 0, 0],
    ],
  );
  assert.ok(report.missing.length > 0, "do not claim full Polish coverage");
  assert.ok(report.missing.some((field) => field.startsWith("fundamentals.")));
  for (const { data, group } of canonicalItems().filter(
    (x) => x.group === "exercises" || x.group === "lessons",
  )) {
    const item = locale.items.find((item) => item.id === data.id);
    assert.ok(item?.fields.some((f) => f.path === "title"));
    assert.ok(item.fields.some((f) => f.path === "summary"));
    if (group === "lessons")
      for (const f of textFields(data))
        assert.ok(
          item.fields.some((entry) => entry.path === f.path),
          `${data.id}.${f.path}`,
        );
  }
});
test("strict overlays reject semantic fields, unknown keys, stale originals, numbers, code and duplicate IDs", () => {
  const bads = [
    (x) =>
      x.items[0].fields.push({
        path: "starterSource",
        source: "PROGRAM Main",
        text: "PROGRAM Zmieniony",
      }),
    (x) => (x.items[0].arbitrary = true),
    (x) => (x.items[0].fields[0].source = "Changed original"),
    (x) => (x.items[0].fields[0].text = "   "),
    (x) => {
      const f = x.items
        .flatMap((i) => i.fields)
        .find((f) => f.source.includes("192.168.0.20"));
      f.text = f.text.replace("192.168.0.20", "192.168.0.21");
    },
    (x) => {
      const f = x.items
        .flatMap((i) => i.fields)
        .find((f) => f.source.includes("```"));
      f.text = f.text.replace("```", "```changed");
    },
    (x) => x.items.push(x.items[0]),
    (x) => x.items[0].fields.push(x.items[0].fields[0]),
    (x) => {
      const f = x.items
        .flatMap((i) => i.fields)
        .find((f) => f.source.includes("StartPB"));
      f.text = f.text.replaceAll("StartPB", "PrzyciskStart");
    },
    (x) => x.items.find((i) => i.id === "instruction.ton").fields.pop(),
    (x) => (x.packVersion = "0.0.0"),
    (x) => (x.items[0].file = "../pack.json"),
  ];
  for (const mutate of bads) {
    const copy = structuredClone(locale);
    mutate(copy);
    assert.ok(validateLocale(copy).errors.length > 0, mutate.toString());
  }
});
test("coverage includes article-specific instructions, evidence, fidelity and facts rather than just titles", () => {
  const fields = canonicalItems().flatMap(({ data }) =>
    textFields(data).map((f) => f.path),
  );
  for (const prefix of [
    "facts.",
    "instruction.inputs.",
    "troubleshooting.diagnosticMethod.",
    "fidelity.omitted.",
    "simulation.fidelity.",
  ])
    assert.ok(
      fields.some((field) => field.startsWith(prefix)),
      prefix,
    );
});

test("the literal audit reports actionable engineering gaps without treating dictionary keys as prose", () => {
  const rows = auditUi();
  assert.ok(rows.some((row) => row.file.endsWith("HardwareTab.tsx")));
  for (const row of rows) {
    assert.ok(row.line > 0 && row.text.trim());
    assert.ok(!/^[a-z]+\.[a-zA-Z.]+$/.test(row.text));
    assert.ok(
      !["PLC", "RUN", "Modbus TCP", "PROFINET", "TON"].includes(row.text),
    );
    assert.ok(!row.file.includes(".test."));
  }
});
test("knowledge verification and native UI builds cannot skip content-locale validation", () => {
  const scripts = JSON.parse(
    readFileSync(path.join(root, "package.json"), "utf8"),
  ).scripts;
  for (const command of ["knowledge:validate", "knowledge:verify"])
    assert.match(scripts[command], /content:validate/);
  const build = JSON.parse(
    readFileSync(path.join(root, "apps/desktop/package.json"), "utf8"),
  ).scripts.build;
  assert.match(build, /content-locales\.mjs && vite build/);
});
