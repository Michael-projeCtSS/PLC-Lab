// Presentation-only overlays for the bundled English pack. No executable fields.
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { DEFAULT_PACK, validate } from "./knowledge-pack.mjs";
import { root } from "./toolchain.mjs";

export const LOCALES = path.join(root, "knowledge/locales");
export const groups = [
  "exercises",
  "lessons",
  "fundamentals",
  "instructions",
  "protocols",
  "troubleshooting",
  "vendors",
];
// A whitelist, not a generic object-patching facility.
const prose =
  /^(?:title|summary|objectives\.\d+|instructions\.\d+|hints\.\d+|criteria\.\d+\.(?:title|onFail)|scenarios\.\d+\.title|sections\.\d+\.(?:heading|body|diagram|question\.(?:question|explanation|options\.\d+))|checkpoint\.\d+\.(?:question|explanation|options\.\d+)|troubleshooting\.(?:symptom|reportedBy|diagnosisOptions\.\d+\.label|repairs\.\d+\.label|debrief\.(?:rootCause|repair|reasoning|evidence\.\d+))|facts\.\d+\.(?:label|value)|instruction\.(?:category|inputs\.\d+\.description|outputs\.\d+\.description|state|retentivity|timing|equivalents\.\d+\.notes)|troubleshooting\.(?:effect|symptoms\.\d+|evidence\.\d+|diagnosticMethod\.\d+|repair\.\d+|explanation)|(?:simulation\.)?fidelity\.(?:modeled|approximated|omitted)\.\d+|simulation\.products\.\d+\.note)$/;
export function textFields(item) {
  const fields = [];
  function visit(value, at) {
    if (typeof value === "string" && value.trim() && prose.test(at))
      fields.push({ path: at, source: value });
    else if (value && typeof value === "object")
      for (const [key, child] of Object.entries(value))
        visit(child, at ? `${at}.${key}` : key);
  }
  visit(item, "");
  return fields;
}
export function canonicalItems() {
  return groups.flatMap((group) =>
    readdirSync(path.join(DEFAULT_PACK, group))
      .filter((f) => f.endsWith(".json"))
      .map((file) => ({
        file: `${group}/${file}`,
        group,
        data: JSON.parse(
          readFileSync(path.join(DEFAULT_PACK, group, file), "utf8"),
        ),
      })),
  );
}
export function validateLocale(locale) {
  const schema = JSON.parse(
    readFileSync(path.join(LOCALES, "content-locale.schema.json"), "utf8"),
  );
  const errors = validate(locale, schema);
  if (errors.length) return { errors, coverage: [], missing: [] };
  const canonical = canonicalItems();
  const pack = JSON.parse(
    readFileSync(path.join(DEFAULT_PACK, "pack.json"), "utf8"),
  );
  if (locale.packId !== pack.id || locale.packVersion !== pack.version)
    errors.push("Locale does not match the canonical pack version");
  const byId = new Map(canonical.map((item) => [item.data.id, item]));
  const seen = new Set();
  const translated = new Map();
  for (const item of locale.items) {
    const original = byId.get(item.id);
    if (!original || original.file !== item.file || seen.has(item.id)) {
      errors.push(
        `${item.id}: unknown, duplicate or mismatched canonical item`,
      );
      continue;
    }
    seen.add(item.id);
    const allowed = new Map(
      textFields(original.data).map((field) => [field.path, field.source]),
    );
    const paths = new Set();
    for (const field of item.fields) {
      if (!field.text.trim())
        errors.push(`${item.id}.${field.path}: empty translation`);
      if (paths.has(field.path) || allowed.get(field.path) !== field.source)
        errors.push(
          `${item.id}.${field.path}: duplicate, stale or forbidden text field`,
        );
      if (
        JSON.stringify(field.source.match(/\b\d+(?:\.\d+)?\b/g) ?? []) !==
        JSON.stringify(field.text.match(/\b\d+(?:\.\d+)?\b/g) ?? [])
      )
        errors.push(
          `${item.id}.${field.path}: numeric engineering values changed`,
        );
      const code = (value) => value.match(/```[\s\S]*?```|`[^`\n]+`/g) ?? [];
      if (
        JSON.stringify(code(field.source)) !== JSON.stringify(code(field.text))
      )
        errors.push(
          `${item.id}.${field.path}: code examples or identifiers changed`,
        );
      const tags = original.data.requiredInterface?.map((tag) => tag.tag) ?? [];
      for (const tag of tags)
        if (field.source.includes(tag) && !field.text.includes(tag))
          errors.push(`${item.id}.${field.path}: missing identifier ${tag}`);
      paths.add(field.path);
    }
    translated.set(item.id, paths);
  }
  const missing = canonical.flatMap(({ data }) =>
    textFields(data)
      .filter((field) => !translated.get(data.id)?.has(field.path))
      .map((field) => `${data.id}.${field.path}`),
  );
  const coverage = groups.map((group) => {
    const items = canonical.filter((item) => item.group === group);
    return {
      group,
      total: items.length,
      catalogue: items.filter((item) =>
        ["title", "summary"].every((key) =>
          translated.get(item.data.id)?.has(key),
        ),
      ).length,
      complete: items.filter((item) =>
        textFields(item.data).every((field) =>
          translated.get(item.data.id)?.has(field.path),
        ),
      ).length,
    };
  });
  for (const { data, group } of canonical) {
    const required =
      locale.complete.includes("exercise-catalogue") && group === "exercises"
        ? ["title", "summary"]
        : [];
    if (
      (locale.complete.includes("practice") &&
        group === "exercises" &&
        data.kind === "practice") ||
      (locale.complete.includes("troubleshooting-cases") &&
        group === "exercises" &&
        data.kind === "troubleshooting") ||
      (locale.complete.includes("lessons") && group === "lessons") ||
      (locale.complete.includes("instructions") && group === "instructions")
    )
      required.push(...textFields(data).map((field) => field.path));
    for (const field of new Set(required))
      if (!translated.get(data.id)?.has(field))
        errors.push(
          `${data.id}.${field}: missing in a declared complete category`,
        );
  }
  return { errors, coverage, missing };
}
if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const report = validateLocale(
    JSON.parse(readFileSync(path.join(LOCALES, "pl.json"), "utf8")),
  );
  if (process.argv.includes("--json")) {
    console.log(JSON.stringify(report, null, 2));
    process.exit(report.errors.length ? 1 : 0);
  }
  console.log(
    "PL CONTENT COVERAGE (complete items; title/summary coverage separately)",
  );
  for (const row of report.coverage)
    console.log(
      `${row.group}: ${row.complete}/${row.total}; catalogue ${row.catalogue}/${row.total}`,
    );
  console.log(`Missing fields: ${report.missing.length}`);
  if (process.argv.includes("--all")) console.log(report.missing.join("\n"));
  if (report.errors.length) {
    console.error(report.errors.join("\n"));
    process.exitCode = 1;
  }
}
