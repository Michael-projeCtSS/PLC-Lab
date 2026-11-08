// UPDATE_PLC_LAB.bat decides from the repository and the installed distribution, never
// replaces a signed release, never installs uncommitted work and keeps its own backups.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { root } from "../../scripts/toolchain.mjs";
import { backupsToPrune, decide } from "../../scripts/update-exe.mjs";

const head = "d950c2f584491f803cb1e45e954576acf897654f";
const development = (sourceCommit) => ({
  releaseKind: "UNSIGNED_DEVELOPMENT_BUILD",
  sourceCommit,
});

test("an installed build of HEAD whose files verify is current", () => {
  const d = decide({
    head,
    dirty: false,
    manifest: development(head),
    problems: [],
  });
  assert.equal(d.action, "current");
  assert.equal(
    decide({
      head,
      dirty: false,
      manifest: development(head),
      problems: [],
      force: true,
    }).action,
    "update",
  );
});

test("another commit, a damaged installation or none at all needs an update", () => {
  for (const [manifest, problems] of [
    [development("0123456789abcdef"), []],
    [development(head), ["resources/product.json: PLC Lab.exe hash differs"]],
    [null, ["no staged distribution"]],
  ])
    assert.equal(
      decide({ head, dirty: false, manifest, problems }).action,
      "update",
    );
});

test("uncommitted work and signed releases are refused with a reason", () => {
  const dirty = decide({ head, dirty: true, manifest: null, problems: [] });
  assert.equal(dirty.action, "refuse");
  assert.match(dirty.reason, /exact source commit[\s\S]*TEST_PLC_LAB\.bat/);
  for (const manifest of [
    { releaseKind: "SIGNED_PRODUCTION_RELEASE", sourceCommit: head },
    { ...development("0123"), signing: { sha256: "x" } },
  ]) {
    const d = decide({
      head,
      dirty: false,
      manifest,
      problems: [],
      force: true,
    });
    assert.equal(d.action, "refuse");
    assert.match(d.reason, /signed production release/);
  }
  assert.equal(
    decide({ head: null, dirty: false, manifest: null, problems: [] }).action,
    "refuse",
  );
});

test("only the updater's own oldest backups are pruned", () => {
  const names = [
    "20261101T100000Z-aaaaaaaaaaaa",
    "20261102T100000Z-bbbbbbbbbbbb",
    "20261103T100000Z-unknown",
    "20261104T100000Z-cccccccccccc",
    "my notes",
    ".restore-1234",
  ];
  assert.deepEqual(backupsToPrune(names, 3), ["20261101T100000Z-aaaaaaaaaaaa"]);
  assert.deepEqual(backupsToPrune(names.slice(0, 2), 3), []);
});

test("UPDATE_PLC_LAB.bat runs the shared launcher in update mode and offers to start only after success", async () => {
  const bat = await readFile(path.join(root, "UPDATE_PLC_LAB.bat"), "utf8");
  assert.match(bat, /scripts\\human-launcher\.bat" "%~dp0" update/);
  assert.match(bat, /UNSIGNED DEVELOPMENT BUILD/);
  const exit = bat.indexOf("exit /b %PLC_LAB_EXIT%");
  const start = bat.indexOf('start "" "%~dp0PLC Lab.exe"');
  assert.ok(exit > 0 && start > exit, "start only after the failure exit");
  const human = await readFile(
    path.join(root, "scripts/human-test.mjs"),
    "utf8",
  );
  assert.match(human, /update: null/);
  const updater = await readFile(
    path.join(root, "scripts/update-exe.mjs"),
    "utf8",
  );
  // The existing pipeline, the native check before any success, no security changes.
  for (const piece of [
    "scripts/stage-release.mjs",
    "--development",
    "verifyDistribution",
    "tests/e2e/desktop-smoke.mjs",
    "codeIntegrityBlocks",
  ])
    assert.ok(updater.includes(piece), piece);
  assert.doesNotMatch(
    updater,
    /Set-MpPreference|Add-MpPreference|VerifiedAndReputablePolicyState|signtool|New-SelfSignedCertificate/i,
  );
  assert.ok(
    updater.indexOf("PLC LAB UPDATED AND VERIFIED") >
      updater.indexOf("desktop-smoke.mjs"),
    "success is reported after the native check",
  );
});
