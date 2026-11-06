// Both themes come from one set of semantic tokens that meets WCAG AA contrast.
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { report, themes } from "../../scripts/theme-contrast.mjs";
import { root } from "../../scripts/toolchain.mjs";

const UI = path.join(root, "packages/ui/src");
const css = readFileSync(path.join(UI, "styles.css"), "utf8");
const HEX = /#[0-9a-fA-F]{3,8}\b/g;
function sources(dir = UI) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const file = path.join(dir, e.name);
    if (e.isDirectory()) return sources(file);
    return /\.(tsx?|css)$/.test(e.name) && !e.name.includes(".test.")
      ? [file]
      : [];
  });
}

test("every text, status and graphic colour pair meets its WCAG AA minimum in both themes", () => {
  const rows = report(css);
  assert.ok(rows.length >= 100, `${rows.length} pairs`);
  const failing = rows.filter((r) => r.ratio < r.min);
  assert.deepEqual(
    failing.map(
      (r) => `${r.theme} --${r.fg} on --${r.bg}: ${r.ratio.toFixed(2)}`,
    ),
    [],
  );
});

test("colours are tokens: no hard-coded colour outside the two token blocks", () => {
  const light = css.indexOf(':root[data-theme="light"] {');
  const rules = css.slice(css.indexOf("\n}", light));
  assert.deepEqual(rules.match(HEX) ?? [], []);
  for (const file of sources().filter((f) => !f.endsWith("styles.css"))) {
    const text = readFileSync(file, "utf8");
    // Monaco themes are defined in its own colour format, next to each other.
    if (file.endsWith("monaco-host.ts")) continue;
    assert.deepEqual(text.match(HEX) ?? [], [], path.relative(root, file));
  }
});

test("every token the interface uses is defined, and the light theme defines a value for each", () => {
  const { dark, light } = themes(css);
  const used = new Set(
    sources()
      .flatMap((f) => [
        ...readFileSync(f, "utf8").matchAll(/var\(--([\w-]+)\)/g),
      ])
      .map((m) => m[1]),
  );
  for (const token of used) assert.ok(dark[token], `--${token} undefined`);
  const lightBlock = css.slice(css.indexOf(':root[data-theme="light"] {'));
  for (const token of Object.keys(dark).filter((t) =>
    /^(bg|text|line|st|accent)/.test(t),
  ))
    assert.ok(lightBlock.includes(`--${token}:`), `light --${token}`);
  assert.equal(light["bg-1"], "#ffffff");
});

test("the editor has a light Monaco theme and switches without recreating the editor", () => {
  const host = readFileSync(path.join(UI, "monaco-host.ts"), "utf8");
  assert.match(host, /defineTheme\("plc-lab-light"/);
  assert.match(host, /monaco\.editor\.setTheme\(/);
  const editor = readFileSync(
    path.join(UI, "StructuredTextEditor.tsx"),
    "utf8",
  );
  assert.match(editor, /setEditorTheme\(theme\)/);
});
