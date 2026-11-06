// Backend codes that reach the interface must have localized wording in both languages.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { root } from "../../scripts/toolchain.mjs";

const read = (file) => readFileSync(path.join(root, file), "utf8");
const dictionaries = ["en", "pl"].map((language) =>
  read(`packages/ui/src/locale/${language}.ts`),
);
const codes = (file) =>
  [
    ...read(file).matchAll(/pub const [A-Z_]+: &str = "(PLC-[A-Z]+-\d{3})";/g),
  ].map((m) => m[1]);

test("every compiler and runtime diagnostic code has a localized title", () => {
  const all = [
    ...codes("languages/st/src/diagnostics.rs"),
    ...codes("crates/plc-types/src/diagnostics.rs"),
  ];
  assert.ok(all.length >= 42, `found ${all.length} codes`);
  for (const code of all)
    for (const dictionary of dictionaries)
      assert.ok(dictionary.includes(`"code.${code}":`), code);
});

test("every CPU diagnostic kind has localized wording", () => {
  const source = read("crates/hardware-model/src/diagnostics.rs");
  const body = source.slice(source.indexOf("pub enum DiagnosticKind {"));
  const kinds = [
    ...body.slice(0, body.indexOf("\n}")).matchAll(/^ {4}([A-Z][A-Za-z]+)/gm),
  ].map((m) => m[1]);
  assert.ok(kinds.length >= 20, `found ${kinds.length} kinds`);
  for (const kind of kinds)
    for (const dictionary of dictionaries)
      assert.ok(
        dictionary.includes(`"diag.kind.${kind}":`) ||
          dictionary.includes(`"diag.kind.${kind}.warm":`),
        kind,
      );
});
