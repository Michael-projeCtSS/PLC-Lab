// Inventory of the coded backend messages and vocabulary terms in the Rust sources, so
// the interface can prove that every one of them has a Polish wording.
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { root } from "./toolchain.mjs";

const SOURCES = [
  "crates/hardware-model/src",
  "crates/lab-engine/src",
  "crates/lab-model/src",
  "crates/desktop-session/src",
];

function files(dir) {
  return readdirSync(path.join(root, dir), { withFileTypes: true }).flatMap(
    (entry) => {
      const relative = `${dir}/${entry.name}`;
      if (entry.isDirectory()) return files(relative);
      return entry.name.endsWith(".rs") ? [relative] : [];
    },
  );
}

/** A Rust string literal's value (escapes this code uses: \" and \\). */
const literal = (text) => text.replaceAll('\\"', '"').replaceAll("\\\\", "\\");
const STRING = String.raw`"((?:[^"\\]|\\.)*)"`;
const PATTERNS = [
  // message!("code", "English {name}", ...)
  ["message", new RegExp(String.raw`message!\(\s*${STRING},\s*${STRING}`, "g")],
  // Message::fixed / short / how ("code", "English")
  [
    "message",
    new RegExp(
      String.raw`(?:Message::fixed|short|how)\(\s*${STRING},\s*${STRING}\s*,?\s*\)`,
      "g",
    ),
  ],
  // ("event.diag.X", vec![...]) codes whose English comes from diagnostic_text
  [
    "message",
    new RegExp(String.raw`\(\s*"(event\.diag\.[A-Za-z.]+)",\s*vec!`, "g"),
  ],
  // term("code", "English") with a literal code
  ["term", new RegExp(String.raw`term\(\s*${STRING},\s*${STRING}`, "g")],
  // ("event.process.x", "English") pairs that become terms
  [
    "term",
    new RegExp(String.raw`"(event\.process\.[A-Za-z]+)",\s*${STRING}`, "g"),
  ],
];

export function backendMessages() {
  const found = new Map();
  for (const file of SOURCES.flatMap(files)) {
    const source = readFileSync(path.join(root, file), "utf8");
    // Unit tests build example messages that never reach the interface.
    const code = source.split(/\n#\[cfg\(test\)\]/)[0];
    for (const [kind, pattern] of PATTERNS)
      for (const match of code.matchAll(pattern)) {
        const id = match[1];
        if (id === "code" || id.startsWith("a.") || id.startsWith("x."))
          continue;
        const english = match[2] === undefined ? null : literal(match[2]);
        const previous = found.get(id);
        if (previous && previous.english !== english && english !== null)
          throw new Error(`${id}: two English texts (${file})`);
        found.set(id, {
          id,
          kind,
          english: english ?? previous?.english ?? null,
          file,
        });
      }
  }
  // Runtime codes built with format!: one term per exception and alarm code.
  return [...found.values()].sort((a, b) => a.id.localeCompare(b.id));
}

export const placeholders = (text) =>
  [...(text ?? "").matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
)
  for (const m of backendMessages())
    console.log(`${m.kind}\t${m.id}\t${m.english ?? ""}`);
