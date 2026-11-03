import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import path from "node:path";
import test from "node:test";
import {
  DEVELOPMENT,
  PRODUCTION,
  assertDevelopmentMayReplace,
  assertSigningRecord,
  fileHash,
  hasCertificateTable,
} from "../../scripts/release-trust.mjs";
import {
  installPreparedDistribution,
  installSignedRelease,
} from "../../scripts/install-signed-release.mjs";
import { stageArtifactSet } from "../../scripts/stage-release.mjs";
import { root } from "../../scripts/toolchain.mjs";

const read = (file) => readFileSync(path.join(root, file), "utf8");
async function fixture() {
  const base = path.join(root, "artifacts/release-trust-tests");
  await mkdir(base, { recursive: true });
  return mkdtemp(path.join(base, "product-"));
}
async function cleanup(directory) {
  assert.ok(
    directory.startsWith(
      `${path.join(root, "artifacts/release-trust-tests")}${path.sep}`,
    ),
  );
  await rm(directory, { recursive: true, force: true });
}

test("production commands require signing and ordinary CI explicitly stages unsigned development", () => {
  const { scripts } = JSON.parse(read("package.json"));
  assert.match(
    scripts["build:release"],
    /^npm run signing:preflight && npm run validate && npm run build:release:native && npm run stage:release$/,
  );
  assert.match(scripts["stage:release"], /--production$/);
  assert.match(scripts["stage:release:unsigned"], /--development$/);
  assert.match(scripts["check:distribution"], /--production$/);
  assert.match(read("scripts/ci.mjs"), /"stage:release:unsigned"/);
  assert.match(read("BUILD_PLC_LAB_RELEASE.bat"), /SIGNED PRODUCTION RELEASE/);
});

test("missing signing configuration fails closed without changing the existing root executable", async () => {
  const exe = path.join(root, "PLC Lab.exe");
  let before;
  try {
    before = await fileHash(exe);
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  const result = spawnSync(
    process.execPath,
    ["scripts/windows-signing.mjs", "preflight"],
    {
      cwd: root,
      env: { ...process.env, PLC_LAB_SIGNING_CONFIG: "" },
      encoding: "utf8",
    },
  );
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /SIGNING NOT CONFIGURED/);
  if (before) assert.equal(await fileHash(exe), before);
});

test("manifest verification binds post-signing bytes and verified publisher evidence", async () => {
  const directory = await fixture();
  try {
    const file = path.join(directory, "PLC Lab.exe");
    // This tests manifest binding only; Windows cryptographic verification is separate.
    await writeFile(file, "pre-signing bytes");
    const oldHash = await fileHash(file);
    await writeFile(file, "final bytes including signature");
    const evidence = {
      sha256: await fileHash(file),
      publisherSubject: "CN=Validated Publisher",
      signerThumbprint: "A".repeat(40),
      rootThumbprint: "B".repeat(40),
      timestampSubject: "CN=Timestamp Authority",
    };
    const manifest = {
      releaseKind: PRODUCTION,
      application: { sha256: evidence.sha256 },
      signing: { ...evidence },
    };
    assert.doesNotThrow(() => assertSigningRecord(manifest, evidence));
    for (const field of [
      "publisherSubject",
      "signerThumbprint",
      "rootThumbprint",
      "timestampSubject",
      "sha256",
    ]) {
      assert.throws(
        () =>
          assertSigningRecord(
            { ...manifest, signing: { ...evidence, [field]: "changed" } },
            evidence,
          ),
        /SIGNATURE VERIFICATION FAILED/,
      );
    }
    assert.throws(
      () =>
        assertSigningRecord(
          { ...manifest, application: { sha256: oldHash } },
          evidence,
        ),
      /SHA-256 disagree/,
    );
    assert.throws(
      () =>
        assertSigningRecord(
          { ...manifest, releaseKind: DEVELOPMENT },
          evidence,
        ),
      /not labeled/,
    );
    await writeFile(file, "changed after signing");
    assert.throws(
      () =>
        assertSigningRecord(manifest, {
          ...evidence,
          sha256: readFileSync(file).toString("hex"),
        }),
      /SHA-256 disagree/,
    );
  } finally {
    await cleanup(directory);
  }
});

test("unsigned staging cannot overwrite a signed release or certificate-bearing executable", async () => {
  const directory = await fixture();
  try {
    await assertDevelopmentMayReplace(directory);
    await mkdir(path.join(directory, "resources"));
    const manifest = path.join(directory, "resources/product.json");
    await writeFile(manifest, JSON.stringify({ releaseKind: PRODUCTION }));
    await assert.rejects(
      assertDevelopmentMayReplace(directory),
      /cannot be overwritten/,
    );
    await writeFile(manifest, JSON.stringify({ releaseKind: DEVELOPMENT }));
    const bytes = Buffer.alloc(512);
    bytes.write("MZ");
    bytes.writeUInt32LE(128, 60);
    bytes.writeUInt32LE(0x4550, 128);
    bytes.writeUInt16LE(0x20b, 152);
    bytes.writeUInt32LE(32, 152 + 112 + 32 + 4);
    assert.equal(hasCertificateTable(bytes), true);
    await writeFile(path.join(directory, "PLC Lab.exe"), bytes);
    await assert.rejects(
      assertDevelopmentMayReplace(directory),
      /cannot be overwritten/,
    );
    assert.deepEqual(
      await readFile(path.join(directory, "PLC Lab.exe")),
      bytes,
    );
    assert.equal(hasCertificateTable(Buffer.from("not a PE")), false);
    bytes.writeUInt32LE(0, 152 + 112 + 32 + 4);
    assert.equal(hasCertificateTable(bytes), false);
  } finally {
    await cleanup(directory);
  }
});

test("installation refuses missing credentials before touching the current product", async () => {
  const directory = await fixture();
  const previous = process.env.PLC_LAB_SIGNING_CONFIG;
  try {
    process.env.PLC_LAB_SIGNING_CONFIG = "";
    const exe = path.join(directory, "PLC Lab.exe");
    await writeFile(exe, "existing root executable");
    await assert.rejects(
      installSignedRelease(path.join(directory, "download"), directory),
      /SIGNING NOT CONFIGURED/,
    );
    assert.equal(await readFile(exe, "utf8"), "existing root executable");
  } finally {
    if (previous === undefined) delete process.env.PLC_LAB_SIGNING_CONFIG;
    else process.env.PLC_LAB_SIGNING_CONFIG = previous;
    await cleanup(directory);
  }
});

async function unsignedProduct(directory, bytes) {
  await mkdir(directory, { recursive: true });
  await writeFile(path.join(directory, "PLC Lab.exe"), bytes);
  const pack = path.join(directory, "pack-source");
  await mkdir(pack);
  await writeFile(path.join(pack, "pack.json"), '{"id":"test-pack"}');
  await stageArtifactSet(
    directory,
    path.join(directory, "resources"),
    [],
    {
      releaseKind: DEVELOPMENT,
      application: {
        name: "PLC Lab.exe",
        sha256: await fileHash(path.join(directory, "PLC Lab.exe")),
      },
      dependencies: [],
    },
    [{ source: pack, target: "knowledge-packs/Fixture" }],
  );
}

test("prepared distribution installation preserves exact bytes and retires the old launcher", async () => {
  const directory = await fixture();
  try {
    const source = path.join(directory, "download");
    const target = path.join(directory, "installed");
    await unsignedProduct(source, "new application");
    await unsignedProduct(target, "old application");
    await writeFile(path.join(target, "Launcher.exe"), "retired launcher");
    const oldDll = path.join(target, "previous.dll");
    await writeFile(oldDll, "previous build dependency");
    const oldManifestPath = path.join(target, "resources/product.json");
    const oldManifest = JSON.parse(await readFile(oldManifestPath, "utf8"));
    oldManifest.dependencies = [
      { name: "previous.dll", sha256: await fileHash(oldDll) },
    ];
    await writeFile(oldManifestPath, JSON.stringify(oldManifest));
    await installPreparedDistribution(source, target, { production: false });
    assert.equal(
      await readFile(path.join(target, "PLC Lab.exe"), "utf8"),
      "new application",
    );
    await assert.rejects(readFile(path.join(target, "Launcher.exe")), {
      code: "ENOENT",
    });
    await assert.rejects(readFile(oldDll), { code: "ENOENT" });
  } finally {
    await cleanup(directory);
  }
});

test("failed final distribution verification rolls back application, resources and retired files", async () => {
  const directory = await fixture();
  try {
    const source = path.join(directory, "download");
    const target = path.join(directory, "installed");
    await unsignedProduct(source, "new application");
    await unsignedProduct(target, "old application");
    const oldManifest = await readFile(
      path.join(target, "resources/product.json"),
    );
    await writeFile(path.join(target, "Launcher.exe"), "retired launcher");
    await writeFile(
      path.join(target, "unexpected.exe"),
      "unrelated executable",
    );
    await assert.rejects(
      installPreparedDistribution(source, target, { production: false }),
      /unexpected application executable/,
    );
    assert.equal(
      await readFile(path.join(target, "PLC Lab.exe"), "utf8"),
      "old application",
    );
    assert.deepEqual(
      await readFile(path.join(target, "resources/product.json")),
      oldManifest,
    );
    assert.equal(
      await readFile(path.join(target, "Launcher.exe"), "utf8"),
      "retired launcher",
    );
  } finally {
    await cleanup(directory);
  }
});
