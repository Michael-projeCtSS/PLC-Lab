// Every coded backend message and vocabulary term has Polish wording in the interface.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import {
  backendMessages,
  placeholders,
} from "../../scripts/backend-messages.mjs";
import { root } from "../../scripts/toolchain.mjs";

const read = (file) => readFileSync(path.join(root, file), "utf8");
/** `"key": "text",` entries of a TypeScript object literal. */
function entries(source) {
  return new Map(
    [...source.matchAll(/^\s+"([^"]+)":\s*"((?:[^"\\]|\\.)*)",$/gm)].map(
      (m) => [m[1], JSON.parse(`"${m[2]}"`)],
    ),
  );
}
const backend = read("packages/ui/src/locale/backend-pl.ts");
const polish = entries(
  backend.slice(0, backend.indexOf("export const sameInPolish")),
);
const same = entries(
  backend.slice(backend.indexOf("export const sameInPolish")),
);
const dictionary = {
  en: entries(read("packages/ui/src/locale/en.ts")),
  pl: entries(read("packages/ui/src/locale/pl.ts")),
};
const wording = (id) => polish.get(id) ?? dictionary.pl.get(id);

test("the inventory finds the coded messages of every converted backend area", () => {
  const codes = backendMessages().map((m) => m.id);
  for (const prefix of ["check.", "hw.", "event.", "device.", "q.", "led."])
    assert.ok(
      codes.some((c) => c.startsWith(prefix)),
      prefix,
    );
  assert.ok(codes.length >= 300, `${codes.length} codes`);
});

test("every backend message and term has Polish wording with the same placeholders", () => {
  for (const m of backendMessages()) {
    const text = wording(m.id);
    assert.ok(text, `${m.id}: no Polish wording`);
    if (m.english !== null)
      assert.deepEqual(placeholders(text), placeholders(m.english), m.id);
  }
});

test("a Polish text identical to the English one is a stated exception", () => {
  for (const m of backendMessages()) {
    const text = wording(m.id);
    if (m.english === null || text !== m.english) continue;
    const words = m.english.replace(/\{\w+\}/g, "");
    assert.ok(
      same.has(m.id) || !/[A-Za-z]/.test(words),
      `${m.id}: identical to English without a reason`,
    );
  }
  for (const [id, reason] of same) {
    assert.ok(reason.length > 10, id);
    assert.ok(polish.has(id), `${id}: listed as unchanged but not worded`);
  }
});

test("module type names in the dictionary match the backend's generic names", () => {
  const source = read("crates/lab-model/src/hardware.rs");
  const between = (from, to) =>
    source.slice(
      source.indexOf(from),
      source.indexOf(to, source.indexOf(from)),
    );
  const generic = new Map(
    [
      ...between("pub fn generic_name", "pub fn channel_count").matchAll(
        /Self::(\w+) => "([^"]+)",/g,
      ),
    ].map((m) => [m[1], m[2]]),
  );
  const ids = [
    ...between(
      "pub fn id(self)",
      "/// The generic name as a vocabulary term",
    ).matchAll(/Self::(\w+) => "([^"]+)",/g),
  ];
  assert.equal(ids.length, generic.size);
  for (const [, variant, id] of ids) {
    assert.equal(dictionary.en.get(`module.${id}`), generic.get(variant), id);
    assert.ok(dictionary.pl.get(`module.${id}`), id);
  }
});
