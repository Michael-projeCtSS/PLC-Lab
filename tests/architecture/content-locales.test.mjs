import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
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
      ["exercises", 53, 53, 53],
      ["lessons", 26, 26, 26],
      ["fundamentals", 28, 28, 28],
      ["instructions", 7, 7, 7],
      ["protocols", 9, 9, 9],
      ["troubleshooting", 30, 30, 30],
      ["vendors", 24, 24, 24],
    ],
  );
  // Every approved prose field has Polish text or an explicit keep reason.
  assert.deepEqual(report.missing, []);
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
  // A fixture proves detection now that the real screens have no gaps left.
  const fixture = mkdtempSync(path.join(tmpdir(), "plclab-audit-"));
  try {
    writeFileSync(
      path.join(fixture, "Panel.tsx"),
      [
        "export const Panel = () => (",
        '  <div title="Module status">',
        "    <h4>Configured versus actual</h4>",
        '    <span aria-label="hw.status">RUN</span>',
        "    <span>TID</span>",
        "  </div>",
        ");",
        "",
      ].join("\n"),
    );
    assert.deepEqual(
      auditUi(fixture).map((row) => [row.line, row.text]),
      [
        [2, "Module status"],
        [3, "Configured versus actual"],
      ],
    );
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
  const rows = auditUi();
  for (const row of rows) {
    assert.ok(row.line > 0 && row.text.trim());
    assert.ok(!/^[a-z]+\.[a-zA-Z_.]+$/.test(row.text));
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

test("an unchanged English text counts only with an explicit reason", () => {
  const item = locale.items.find((i) => i.fields.some((f) => !f.keep));
  const field = item.fields.find((f) => !f.keep);
  const copy = (change) => ({
    ...locale,
    items: locale.items.map((i) =>
      i === item
        ? {
            ...i,
            fields: i.fields.map((f) => (f === field ? change(f) : f)),
          }
        : i,
    ),
  });
  const untranslated = validateLocale(copy((f) => ({ ...f, text: f.source })));
  assert.ok(
    untranslated.errors.some((e) => e.includes("identical to the English")),
    untranslated.errors.join("\n"),
  );
  const misused = validateLocale(copy((f) => ({ ...f, keep: "code" })));
  assert.ok(
    misused.errors.some((e) => e.includes("keep is only for text identical")),
    misused.errors.join("\n"),
  );
  const kept = locale.items.flatMap((i) => i.fields).filter((f) => f.keep);
  assert.ok(kept.every((f) => f.text === f.source));
});

test("a declared complete group fails when one of its fields is missing", () => {
  for (const [declared, prefix] of [
    ["commissioning", "cm."],
    ["fundamentals", "fundamentals."],
    ["protocols", "protocol."],
    ["troubleshooting", "troubleshooting."],
    ["vendors", "vendor."],
  ]) {
    assert.ok(locale.complete.includes(declared), declared);
    const item = locale.items.find((i) => i.id.startsWith(prefix));
    assert.ok(item, prefix);
    const removed = item.fields.at(-1);
    const report = validateLocale({
      ...locale,
      items: locale.items.map((i) =>
        i === item ? { ...i, fields: i.fields.slice(0, -1) } : i,
      ),
    });
    assert.ok(
      report.errors.includes(
        `${item.id}.${removed.path}: missing in a declared complete category`,
      ),
      `${declared}: ${report.errors.join(" | ")}`,
    );
  }
});
