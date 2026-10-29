// The user distribution is one application executable, `PLC Lab.exe`, in the product
// root with its data in resources/ (ADR-0030). No launcher, no second application EXE.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import {
  APPLICATION,
  distributionEntries,
  verifyDistribution,
} from "../../scripts/distribution.mjs";
import { root } from "../../scripts/toolchain.mjs";

const read = (file) => readFileSync(path.join(root, file), "utf8");
const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");

async function product({ extra = [], tamper = false } = {}) {
  const base = path.join(root, "artifacts/human-test-fixtures");
  await mkdir(base, { recursive: true });
  const dir = await mkdtemp(path.join(base, "product with spaces-"));
  const exe = Buffer.from("MZ fixture application");
  const pack = Buffer.from('{"id":"plclab.core"}');
  await writeFile(path.join(dir, APPLICATION), exe);
  await mkdir(path.join(dir, "resources/knowledge-packs/Pack"), {
    recursive: true,
  });
  await writeFile(
    path.join(dir, "resources/knowledge-packs/Pack/pack.json"),
    pack,
  );
  await writeFile(
    path.join(dir, "resources/product.json"),
    JSON.stringify({
      application: { name: APPLICATION, sha256: sha(tamper ? "other" : exe) },
      dependencies: [],
      files: [{ name: "knowledge-packs/Pack/pack.json", sha256: sha(pack) }],
    }),
  );
  await writeFile(
    path.join(dir, "RUN_PLC_LAB.bat"),
    '@echo off\r\nstart "" "%~dp0PLC Lab.exe"\r\n',
  );
  for (const name of extra) {
    await mkdir(path.dirname(path.join(dir, name)), { recursive: true });
    await writeFile(path.join(dir, name), "x");
  }
  return dir;
}

async function cleanup(dir) {
  const expected = path.resolve(root, "artifacts/human-test-fixtures");
  assert.ok(path.resolve(dir).startsWith(`${expected}${path.sep}`));
  await rm(dir, { recursive: true, force: true });
}

test("a complete distribution has exactly one application executable", async () => {
  const dir = await product();
  try {
    assert.deepEqual(await verifyDistribution(dir), []);
    assert.deepEqual(await distributionEntries(dir), [
      APPLICATION,
      "resources",
      "RUN_PLC_LAB.bat",
    ]);
  } finally {
    await cleanup(dir);
  }
});

test("a second application executable, a launcher or the old release folder fails", async () => {
  for (const extra of [
    "Launcher.exe",
    "PLC Lab Desktop.exe",
    "resources/helper.exe",
    "releases/current/PLC Lab Desktop.exe",
  ]) {
    const dir = await product({ extra: [extra] });
    try {
      const problems = await verifyDistribution(dir);
      assert.ok(problems.length > 0, `${extra} must be refused`);
    } finally {
      await cleanup(dir);
    }
  }
});

test("a modified application or missing resources fail the check", async () => {
  const tampered = await product({ tamper: true });
  try {
    assert.match(
      (await verifyDistribution(tampered)).join("\n"),
      /differs from the staged build/,
    );
  } finally {
    await cleanup(tampered);
  }
  const bare = await product();
  try {
    await rm(path.join(bare, "resources"), { recursive: true });
    assert.match(
      (await verifyDistribution(bare)).join("\n"),
      /product\.json is missing/,
    );
  } finally {
    await cleanup(bare);
  }
});

test("the workspace has no launcher and stages the Tauri application as PLC Lab.exe", () => {
  assert.doesNotMatch(read("Cargo.toml"), /apps\/launcher/);
  assert.doesNotMatch(read("Cargo.lock"), /name = "plc-lab-launcher"/);
  const stage = read("scripts/stage-release.mjs");
  assert.match(stage, /item\.name === "plc-lab-desktop"/);
  assert.match(
    stage,
    /installFile\(sourceExe, path\.join\(productRoot, APPLICATION\)\)/,
  );
  assert.doesNotMatch(
    stage,
    /plc-lab-launcher|ROOT_LAUNCHERS|PLC Lab Desktop\.exe/,
  );
  assert.equal(APPLICATION, "PLC Lab.exe");
  const packageJson = JSON.parse(read("package.json"));
  for (const [name, command] of Object.entries(packageJson.scripts))
    assert.doesNotMatch(command, /launcher|releases\/current/i, name);
  // The application finds its data from its own folder, never the working directory.
  const host = read("apps/desktop/src-tauri/src/resources.rs");
  assert.match(host, /std::env::current_exe\(\)/);
  assert.match(host, /pub const RESOURCES: &str = "resources";/);
  assert.doesNotMatch(
    read("apps/desktop/src-tauri/src/lib.rs"),
    /current_dir\(\)\.unwrap_or_default\(\)\)\s*\.join\("knowledge/,
  );
});

test("the optional BAT only starts PLC Lab.exe next to it", () => {
  const bat = read("RUN_PLC_LAB.bat");
  assert.match(bat, /start "" "%~dp0PLC Lab\.exe"/);
  assert.doesNotMatch(bat, /releases|Launcher|npm|cargo|node /i);
});
