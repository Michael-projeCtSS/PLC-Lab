import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import {
  DEFAULT_PACK,
  DEFAULT_ZIP,
  PACK_NAME,
  buildZip,
  listFiles,
  readZip,
  validate,
  validatePack,
  verifyZip,
} from "../../scripts/knowledge-pack.mjs";

const read = (rel) => readFileSync(path.join(DEFAULT_PACK, rel), "utf8");
const schema = (name) => JSON.parse(read(`schemas/${name}.schema.json`));

test("the canonical pack satisfies its schemas with unique identifiers", () => {
  const { problems, items } = validatePack(listFiles(DEFAULT_PACK), read);
  assert.deepEqual(problems, []);
  assert.ok(items > 100);
});

test("schema validation rejects unknown fields, missing fields and bad identifiers", () => {
  const article = JSON.parse(read("fundamentals/4-20ma.json"));
  const s = schema("article");
  assert.deepEqual(validate(article, s), []);
  assert.match(
    validate({ ...article, extra: 1 }, s).join(),
    /unknown field extra/,
  );
  const { summary: _drop, ...missing } = article;
  void _drop;
  assert.match(validate(missing, s).join(), /missing summary/);
  assert.match(
    validate({ ...article, id: "Bad Id" }, s).join(),
    /does not match/,
  );
  const exercise = JSON.parse(read("exercises/ex-tank-filling.json"));
  const broken = structuredClone(exercise);
  broken.criteria[0].rule = { type: "sometimes" };
  assert.match(validate(broken, schema("exercise")).join(), /allowed shapes/);
});

test("the archive is deterministic, complete and byte-identical to the pack", () => {
  const a = buildZip(DEFAULT_PACK);
  const b = buildZip(DEFAULT_PACK);
  assert.ok(a.equals(b), "two builds must be identical");
  const entries = readZip(a);
  assert.deepEqual(
    [...entries.keys()].sort(),
    listFiles(DEFAULT_PACK)
      .map((f) => `${PACK_NAME}/${f}`)
      .sort(),
  );
  assert.equal(
    entries.get(`${PACK_NAME}/pack.json`).toString("utf8"),
    read("pack.json"),
  );
});

test("the committed release archive matches the current pack", () => {
  const { problems, files } = verifyZip(DEFAULT_ZIP, DEFAULT_PACK);
  assert.deepEqual(problems, []);
  assert.ok(files > 100);
  assert.ok(
    buildZip(DEFAULT_PACK).equals(readFileSync(DEFAULT_ZIP)),
    "run npm run knowledge:zip",
  );
});

test("tampered archives are detected", () => {
  const zip = Buffer.from(buildZip(DEFAULT_PACK));
  // Flip one byte inside the compressed data of the first entry.
  const dataStart = 30 + zip.readUInt16LE(26) + zip.readUInt16LE(28);
  zip[dataStart + 8] ^= 0xff;
  assert.throws(() => readZip(zip));
});

test("troubleshooting labs show only general references before the diagnosis", () => {
  const files = listFiles(DEFAULT_PACK);
  const items = new Map(
    files
      .filter((f) => f.endsWith(".json") && !f.startsWith("schemas/"))
      .map((f) => JSON.parse(read(f)))
      .filter((d) => typeof d.id === "string")
      .map((d) => [d.id, d]),
  );
  const labs = [...items.values()].filter(
    (d) => d.kind === "troubleshooting" && d.troubleshooting?.symptom,
  );
  assert.equal(labs.length, 7);
  for (const lab of labs) {
    const t = lab.troubleshooting;
    assert.equal(
      lab.knowledgeRefs,
      undefined,
      `${lab.id} uses learningRefs/debriefRefs`,
    );
    assert.ok(t.learningRefs.length > 0 && t.debriefRefs.length > 0, lab.id);
    for (const ref of t.learningRefs) {
      const item = items.get(ref);
      assert.ok(item, `${lab.id} -> ${ref}`);
      assert.notEqual(
        item.kind,
        "troubleshooting",
        `${lab.id} -> ${ref} is a fault guide`,
      );
      for (const related of item.related ?? [])
        assert.ok(
          !t.debriefRefs.includes(related),
          `${lab.id}: ${ref} links to the debrief guide ${related}`,
        );
    }
    assert.ok(
      t.debriefRefs.some((r) => items.get(r)?.kind === "troubleshooting"),
      `${lab.id}: the debrief names its guide`,
    );
  }
});
