import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { readPreviewContent } from "../../apps/desktop/preview-content.ts";
import {
  prepareNativeTest,
  fetchNativeTest,
} from "../../scripts/isolated-native-test.mjs";
import { root } from "../../scripts/toolchain.mjs";

const sha = (value) => createHash("sha256").update(value).digest("hex");
const base = path.join(root, "artifacts/local-testing-fixtures");
async function cleanup(directory, parent = base) {
  assert.ok(
    path.resolve(directory).startsWith(`${path.resolve(parent)}${path.sep}`),
  );
  await rm(directory, { recursive: true, force: true });
}

test("preview projects real public knowledge without runtime or exercise solutions", async () => {
  const content = await readPreviewContent(
    path.join(root, "knowledge/PLC_Lab_Knowledge_Pack_v1"),
  );
  assert.equal(content.lessons.length, 25);
  assert.equal(content.articles.length, 95);
  assert.equal(content.library.exercises.length, 49);
  assert.deepEqual(content.library.scenes, []);
  assert.deepEqual(content.library.platforms, []);
  for (const exercise of content.library.exercises) {
    assert.deepEqual(
      Object.keys(exercise).sort(),
      [
        "id",
        "title",
        "summary",
        "kind",
        "difficulty",
        "minutes",
        "scene",
        "platform",
        "attempts",
        "bestScore",
        "maxScore",
        "passed",
      ].sort(),
    );
    assert.equal(exercise.maxScore, 0);
  }
  const lesson = content.lessons.find((item) => item.number === 1);
  assert.equal(lesson.sections[0].sectionType, "concept");
  const ton = content.articles.find((item) => item.id === "instruction.ton");
  assert.ok(ton?.instruction?.inputs.some((item) => item.dataType === "BOOL"));
  assert.ok(content.catalog.instructions.some((item) => item.id === ton.id));
  assert.equal(JSON.stringify(content).includes('"starterProgram"'), false);
});

test("invalid public document reports its source error rather than returning an empty catalogue", async () => {
  await mkdir(base, { recursive: true });
  const directory = await mkdtemp(path.join(base, "invalid-"));
  try {
    for (const folder of [
      "lessons",
      "exercises",
      "fundamentals",
      "instructions",
      "protocols",
      "troubleshooting",
      "vendors",
      "sources",
    ])
      await mkdir(path.join(directory, folder));
    await writeFile(path.join(directory, "lessons/bad.json"), "invalid JSON");
    await assert.rejects(readPreviewContent(directory), SyntaxError);
  } finally {
    await cleanup(directory);
  }
});

test("isolated preparation rejects tampering and copies a valid unsigned distribution without launching", async () => {
  await mkdir(base, { recursive: true });
  const directory = await mkdtemp(path.join(base, "product with spaces-"));
  let session;
  try {
    const exe = "MZ inert inventory fixture; never executed";
    const pack = '{"id":"fixture"}';
    const bat = '@echo off\r\nstart "" "%~dp0PLC Lab.exe"\r\n';
    await mkdir(path.join(directory, "resources/knowledge-packs/Test"), {
      recursive: true,
    });
    await writeFile(path.join(directory, "PLC Lab.exe"), exe);
    await writeFile(path.join(directory, "RUN_PLC_LAB.bat"), bat);
    await writeFile(
      path.join(directory, "resources/knowledge-packs/Test/pack.json"),
      pack,
    );
    const manifest = {
      releaseKind: "UNSIGNED_DEVELOPMENT_BUILD",
      sourceCommit: "a".repeat(40),
      application: { name: "PLC Lab.exe", sha256: sha(exe) },
      fallback: { name: "RUN_PLC_LAB.bat", sha256: sha(bat) },
      dependencies: [],
      files: [{ name: "knowledge-packs/Test/pack.json", sha256: sha(pack) }],
    };
    await writeFile(
      path.join(directory, "resources/product.json"),
      JSON.stringify(manifest),
    );
    await writeFile(path.join(directory, "PLC Lab.exe"), "changed");
    await assert.rejects(
      prepareNativeTest(directory),
      /differs from the staged build/,
    );
    await writeFile(path.join(directory, "PLC Lab.exe"), exe);
    await writeFile(
      path.join(directory, "RUN_PLC_LAB.bat"),
      bat + "rem changed\r\n",
    );
    await assert.rejects(prepareNativeTest(directory), /fallback BAT hash/);
    await writeFile(path.join(directory, "RUN_PLC_LAB.bat"), bat);
    manifest.releaseKind = "SIGNED_PRODUCTION_RELEASE";
    await writeFile(
      path.join(directory, "resources/product.json"),
      JSON.stringify(manifest),
    );
    await assert.rejects(
      prepareNativeTest(directory),
      /UNSIGNED_DEVELOPMENT_BUILD/,
    );
    manifest.releaseKind = "UNSIGNED_DEVELOPMENT_BUILD";
    await writeFile(
      path.join(directory, "resources/product.json"),
      JSON.stringify(manifest),
    );
    const result = await prepareNativeTest(directory);
    session = result.session;
    assert.equal(result.integrity, "PASS");
    assert.equal(result.hostExecutableLaunched, false);
    assert.equal(
      await readFile(path.join(session, "input/product/PLC Lab.exe"), "utf8"),
      exe,
    );
    const configuration = await readFile(result.sandboxFile, "utf8");
    assert.match(configuration, /<Networking>Disable<\/Networking>/);
    assert.match(configuration, /<ReadOnly>true<\/ReadOnly>/);
    assert.match(configuration, /<Command>explorer.exe /);
    assert.doesNotMatch(
      configuration,
      /ExecutionPolicy|PLC Lab\.exe|powershell/i,
    );
  } finally {
    await cleanup(directory);
    if (session)
      await cleanup(session, path.join(root, "artifacts/native-tests"));
  }
});

test("invalid CI run identity is rejected before download", async () => {
  await assert.rejects(
    fetchNativeTest("not-a-run"),
    /successful Canonical validation run ID/,
  );
});
