import assert from "node:assert/strict";
import {
  copyFile,
  mkdir,
  mkdtemp,
  readFile,
  rm,
  writeFile,
} from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { root } from "../../scripts/toolchain.mjs";
import {
  lockfileMatchesInstalled,
  onlyOverriddenProblems,
  taskListContainsApplication,
} from "../../scripts/human-test.mjs";
import { stageArtifactSet } from "../../scripts/stage-release.mjs";

test("dependency preflight accepts installed platform subset and rejects version/integrity drift", () => {
  const expected = {
    packages: {
      "node_modules/a": { version: "1", integrity: "hash-a" },
      "node_modules/optional-other-platform": { version: "1" },
    },
  };
  assert.equal(
    lockfileMatchesInstalled(expected, {
      packages: { "node_modules/a": { version: "1", integrity: "hash-a" } },
    }),
    true,
  );
  for (const installed of [
    {},
    { packages: { "node_modules/a": { version: "2" } } },
    { packages: { "node_modules/a": { version: "1", integrity: "wrong" } } },
    { packages: { "node_modules/unexpected": { version: "1" } } },
  ]) {
    assert.equal(lockfileMatchesInstalled(expected, installed), false);
  }
});

test("dependency preflight accepts only exact root overrides reported by npm ls", () => {
  const overrides = { dompurify: "3.4.17" };
  assert.equal(
    onlyOverriddenProblems(
      ["invalid: dompurify@3.4.17 /repo/node_modules/dompurify"],
      overrides,
    ),
    true,
  );
  assert.equal(
    onlyOverriddenProblems(
      ["invalid: dompurify@3.4.15 /repo/node_modules/dompurify"],
      overrides,
    ),
    false,
  );
  assert.equal(
    onlyOverriddenProblems(["missing: react@19.3.0, required by x"], overrides),
    false,
  );
  assert.equal(onlyOverriddenProblems([], overrides), false);
});

test("running application preflight recognizes exact names and ignores localized no-task output", () => {
  assert.equal(
    taskListContainsApplication(
      '"PLC Lab.exe","123","Console","1","9 K"\r\n',
      "PLC Lab.exe",
    ),
    true,
  );
  assert.equal(
    taskListContainsApplication(
      '"PLC-LAB-DESKTOP.EXE","123","Console","1","9 K"',
      "plc-lab-desktop.exe",
    ),
    true,
  );
  assert.equal(
    taskListContainsApplication(
      '"plc-lab-desktop-helper.exe","123"',
      "plc-lab-desktop.exe",
    ),
    false,
  );
  assert.equal(
    taskListContainsApplication(
      "INFORMACJA: Brak zadań spełniających podane kryteria.",
      "PLC Lab.exe",
    ),
    false,
  );
});

async function fixture() {
  const directory = path.join(root, "artifacts/human-test-fixtures");
  await mkdir(directory, { recursive: true });
  return mkdtemp(path.join(directory, "test with spaces-"));
}

async function cleanup(directory) {
  const expected = path.resolve(root, "artifacts/human-test-fixtures");
  assert.ok(path.resolve(directory).startsWith(`${expected}${path.sep}`));
  await rm(directory, { recursive: true, force: true });
}

test("release staging preserves exact bytes and replaces the whole previous artifact set", async () => {
  const directory = await fixture();
  try {
    const source = path.join(directory, "source");
    const current = path.join(directory, "releases/current");
    await mkdir(source);
    await mkdir(current, { recursive: true });
    const bytes = Buffer.from([0, 1, 255, 4, 0, 9]);
    await writeFile(path.join(source, "fixture.bin"), bytes);
    await writeFile(path.join(current, "stale.dll"), "old");
    await stageArtifactSet(
      source,
      current,
      [{ source: "fixture.bin", target: "copied.bin" }],
      { profile: "unit-test-fixture" },
    );
    assert.deepEqual(await readFile(path.join(current, "copied.bin")), bytes);
    await assert.rejects(readFile(path.join(current, "stale.dll")), {
      code: "ENOENT",
    });
    const manifest = JSON.parse(
      await readFile(path.join(current, "product.json"), "utf8"),
    );
    assert.equal(manifest.files[0].name, "copied.bin");
    assert.equal(manifest.files[0].sha256.length, 64);
    await assert.rejects(
      stageArtifactSet(
        source,
        current,
        [{ source: "missing.bin", target: "copied.bin" }],
        {},
      ),
      { code: "ENOENT" },
    );
    assert.deepEqual(await readFile(path.join(current, "copied.bin")), bytes);
    await assert.rejects(
      stageArtifactSet(
        source,
        current,
        [{ source: "../fixture.bin", target: "copied.bin" }],
        {},
      ),
      /directory paths/,
    );
  } finally {
    await cleanup(directory);
  }
});

test(
  "fallback BAT from another working directory fails clearly when PLC Lab.exe is missing",
  { skip: process.platform !== "win32" },
  async () => {
    const directory = await fixture();
    try {
      await copyFile(
        path.join(root, "RUN_PLC_LAB.bat"),
        path.join(directory, "RUN_PLC_LAB.bat"),
      );
      const result = spawnSync(
        process.env.ComSpec ?? "C:/Windows/System32/cmd.exe",
        ["/d", "/c", `call "${path.join(directory, "RUN_PLC_LAB.bat")}"`],
        {
          cwd: root,
          env: { ...process.env, PLC_LAB_NO_PAUSE: "1" },
          encoding: "utf8",
          windowsVerbatimArguments: true,
        },
      );
      assert.ifError(result.error);
      assert.notEqual(result.status, 0);
      assert.match(
        result.stdout,
        /PLC Lab\.exe was not found next to this file\. Run BUILD_PLC_LAB_RELEASE\.bat first\./,
      );
    } finally {
      await cleanup(directory);
    }
  },
);

test(
  "actual checks BAT propagates missing-tool failure and never prints success",
  { skip: process.platform !== "win32" },
  () => {
    const pathKey =
      Object.keys(process.env).find((key) => key.toUpperCase() === "PATH") ??
      "PATH";
    const result = spawnSync(
      process.env.ComSpec ?? "C:/Windows/System32/cmd.exe",
      ["/d", "/c", `call "${path.join(root, "RUN_ALL_CHECKS.bat")}"`],
      {
        cwd: path.dirname(root),
        env: { ...process.env, [pathKey]: "", PLC_LAB_NO_PAUSE: "1" },
        encoding: "utf8",
        windowsVerbatimArguments: true,
      },
    );
    assert.ifError(result.error);
    assert.notEqual(result.status, 0);
    assert.match(result.stdout, /CHECKS FAILED/);
    assert.doesNotMatch(result.stdout, /ALL CHECKS PASSED/);
  },
);

test("root BAT helpers resolve every path from their own folder", async () => {
  for (const name of [
    "RUN_PLC_LAB.bat",
    "RUN_PLC_LAB_DEV.bat",
    "RUN_ALL_CHECKS.bat",
    "BUILD_PLC_LAB_RELEASE.bat",
  ]) {
    const text = await readFile(path.join(root, name), "utf8");
    assert.match(text, /%~dp0/, `${name} must locate files from %~dp0`);
    assert.doesNotMatch(
      text,
      /\b[A-Za-z]:[\\/]/,
      `${name} must not contain an absolute drive path`,
    );
    for (const line of text.split(/\r?\n/).filter((l) => l.includes("%~dp0")))
      assert.match(line, /"[^"]*%~dp0/, `${name}: unquoted %~dp0 in ${line}`);
  }
});
