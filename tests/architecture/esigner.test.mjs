import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import test from "node:test";
import {
  assertEsignerPackage,
  esignerCredentials,
  ESIGNER_PACKAGE,
} from "../../scripts/esigner.mjs";
import { root } from "../../scripts/toolchain.mjs";

const credentials = {
  ESIGNER_USERNAME: "synthetic-user@example.invalid",
  ESIGNER_PASSWORD: 'synthetic-password-with-"-$()-`-&-characters',
  ESIGNER_CREDENTIAL_ID: "synthetic-credential",
  ESIGNER_TOTP_SECRET: "synthetic-totp-secret",
};

test("eSigner requires every protected credential and never includes values in errors", () => {
  assert.deepEqual(esignerCredentials(credentials), credentials);
  for (const name of Object.keys(credentials)) {
    for (const value of [
      undefined,
      "",
      "synthetic\nsecret",
      "synthetic\0secret",
    ]) {
      assert.throws(
        () => esignerCredentials({ ...credentials, [name]: value }),
        (error) => {
          assert.match(error.message, /SIGNING NOT CONFIGURED/);
          assert.match(error.message, new RegExp(name));
          for (const secret of Object.values(credentials))
            assert.ok(!error.message.includes(secret));
          return true;
        },
      );
    }
  }
});

test("the official client version, archive and extracted executable code are pinned", () => {
  assert.match(
    ESIGNER_PACKAGE.url,
    /^https:\/\/github\.com\/SSLcom\/CodeSignTool\/releases\/download\/v1\.3\.2\/CodeSignTool-v1\.3\.2\.zip$/,
  );
  for (const digest of [
    ESIGNER_PACKAGE.sha256,
    ESIGNER_PACKAGE.jarSha256,
    ESIGNER_PACKAGE.propertiesSha256,
  ])
    assert.match(digest, /^[a-f0-9]{64}$/);
  assert.throws(
    () => assertEsignerPackage(Buffer.from("tampered package")),
    /SHA-256 mismatch/,
  );
});

test("missing signing credentials fail before Java or network calls without leaking other secrets", () => {
  const env = { ...process.env, ...credentials };
  delete env.ESIGNER_CREDENTIAL_ID;
  const result = spawnSync(
    process.execPath,
    ["scripts/esigner.mjs", "preflight"],
    { cwd: root, env, encoding: "utf8" },
  );
  assert.notEqual(result.status, 0);
  assert.match(
    result.stderr,
    /SIGNING NOT CONFIGURED: set protected ESIGNER_CREDENTIAL_ID/,
  );
  assert.equal(result.stdout, "");
  for (const secret of Object.values(credentials))
    assert.ok(!`${result.stdout}\n${result.stderr}`.includes(secret));
});

test("eSigner executes Java with argument arrays and discards provider diagnostics", async () => {
  const source = await readFile(
    new URL("../../scripts/esigner.mjs", import.meta.url),
    "utf8",
  );
  assert.match(source, /await execute\(java, args,/);
  assert.doesNotMatch(source, /shell: true|stdio: ["']inherit/);
  assert.match(source, /JAVA_TOOL_OPTIONS/);
  assert.match(source, /CODE_SIGN_TOOL_PATH: temporary/);
  assert.match(source, /esignerPreflight\(\)/);
  assert.match(source, /no raw credential diagnostics are logged/);
});

test(
  "the credential-free client doctor reports unavailable Java without exposing environment secrets",
  {
    skip:
      process.platform !== "win32" ||
      !existsSync(
        new URL(
          "../../.tools/esigner-v1.3.2/jar/code_sign_tool-1.3.2.jar",
          import.meta.url,
        ),
      ),
  },
  () => {
    const env = {
      ...process.env,
      ...credentials,
      JAVA_HOME_17_X64: "C:/PLC-Lab-missing-Java-runtime",
      JAVA_HOME: "C:/PLC-Lab-missing-Java-runtime",
    };
    const result = spawnSync(
      process.execPath,
      ["scripts/esigner.mjs", "doctor"],
      { cwd: root, env, encoding: "utf8" },
    );
    assert.notEqual(result.status, 0);
    assert.match(
      result.stderr,
      /SIGNING NOT CONFIGURED: Java 17 or later is unavailable or blocked/,
    );
    assert.equal(result.stdout, "");
    for (const secret of Object.values(credentials))
      assert.ok(!`${result.stdout}\n${result.stderr}`.includes(secret));
  },
);
