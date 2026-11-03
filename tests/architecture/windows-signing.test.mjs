import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import {
  loadSigningConfig,
  preflightSigning,
  validateSigningConfig,
  verifyExecutable,
} from "../../scripts/windows-signing.mjs";

const rootFingerprint = "1234567890ABCDEF1234567890ABCDEF12345678";
const configuration = () => ({
  provider: "azure-artifact-signing",
  publisherSubject: "CN=PLC Lab Test Publisher, O=PLC Lab Test Publisher, C=PL",
  trustedRootThumbprints: [rootFingerprint],
  azure: {
    endpoint: "https://plc.codesigning.azure.net",
    accountName: "plc-lab-test",
    certificateProfileName: "public-trust",
  },
});

test("missing configuration, placeholders, secrets and unpinned roots fail closed", async () => {
  await assert.rejects(loadSigningConfig(""), /SIGNING NOT CONFIGURED/);
  await assert.rejects(
    loadSigningConfig(path.join(tmpdir(), "plc-lab-no-provider-config.json")),
    /SIGNING NOT CONFIGURED/,
  );
  for (const config of [
    {},
    { ...configuration(), publisherSubject: "CN=REPLACE_PUBLISHER" },
    { ...configuration(), trustedRootThumbprints: [] },
    { ...configuration(), trustedRootThumbprints: ["0".repeat(40)] },
    { ...configuration(), pfxPassword: "not-a-real-password" },
    { ...configuration(), provider: "self-signed" },
    {
      ...configuration(),
      azure: { ...configuration().azure, clientSecret: "not-a-real-secret" },
    },
  ])
    assert.throws(
      () => validateSigningConfig(config),
      /SIGNING NOT CONFIGURED/,
    );
});

test("Artifact Signing endpoints must be the official regional service, with no URL credentials", () => {
  assert.equal(
    validateSigningConfig(configuration()).azure.endpoint,
    "https://plc.codesigning.azure.net",
  );
  for (const endpoint of [
    123,
    null,
    "https://plc.codesigning.azure.net.attacker.test",
    "http://plc.codesigning.azure.net",
    "https://user:password@plc.codesigning.azure.net",
    "https://plc.codesigning.azure.net/?secret=value",
  ]) {
    assert.throws(
      () =>
        validateSigningConfig({
          ...configuration(),
          azure: { ...configuration().azure, endpoint },
        }),
      /SIGNING NOT CONFIGURED/,
    );
  }
});

test("verification config needs no private key, signing client or credentials", async () => {
  const directory = await mkdtemp(
    path.join(tmpdir(), "plc-signing-config-test-"),
  );
  try {
    const file = path.join(directory, "config.json");
    await writeFile(
      file,
      JSON.stringify({ ...configuration(), signtoolPath: "sdk/signtool.exe" }),
    );
    const loaded = await loadSigningConfig(file);
    assert.equal(loaded.signtoolPath, path.join(directory, "sdk/signtool.exe"));
    assert.equal(loaded.azure.dlibPath, undefined);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("individual eSigner verification uses only public identity and root settings", () => {
  const config = {
    provider: "ssl-esigner",
    publisherSubject: "CN=Test Individual, C=PL",
    trustedRootThumbprints: [rootFingerprint],
  };
  assert.equal(validateSigningConfig(config).provider, "ssl-esigner");
  assert.throws(
    () => validateSigningConfig({ ...config, password: "not-a-real-password" }),
    /SIGNING NOT CONFIGURED/,
  );
});

test("certificate-store route selects an exact certificate and a documented timestamp endpoint", () => {
  const config = {
    provider: "certificate-store",
    publisherSubject: "CN=PLC Lab Test Publisher, C=PL",
    trustedRootThumbprints: [rootFingerprint],
    certificateStore: {
      thumbprint: rootFingerprint,
      storeLocation: "CurrentUser",
      timestampUrl: "http://timestamp.digicert.com",
    },
  };
  assert.equal(
    validateSigningConfig(config).certificateStore.thumbprint,
    rootFingerprint,
  );
  for (const timestampUrl of [
    "file:///private/key",
    "https://user:password@timestamp.digicert.com",
    "https://timestamp.digicert.com/?token=secret",
  ]) {
    assert.throws(
      () =>
        validateSigningConfig({
          ...config,
          certificateStore: { ...config.certificateStore, timestampUrl },
        }),
      /SIGNING NOT CONFIGURED/,
    );
  }
  assert.throws(
    () =>
      validateSigningConfig({
        ...config,
        certificateStore: { ...config.certificateStore, storeLocation: "Root" },
      }),
    /SIGNING NOT CONFIGURED/,
  );
});

test(
  "unsigned x64 PE cannot pass the real Windows Authenticode verifier",
  { skip: process.platform !== "win32" },
  async () => {
    const directory = await mkdtemp(
      path.join(tmpdir(), "plc-signing-pe-test-"),
    );
    try {
      const file = path.join(directory, "unsigned.exe");
      // A minimal PE header exercises the real Windows verification path, with no certificate table.
      const bytes = Buffer.alloc(512);
      bytes.write("MZ");
      bytes.writeUInt32LE(64, 60);
      bytes.writeUInt32LE(0x4550, 64);
      bytes.writeUInt16LE(0x8664, 68);
      bytes.writeUInt16LE(0x020b, 88);
      await writeFile(file, bytes);
      await assert.rejects(
        verifyExecutable(file, { config: configuration() }),
        /SIGNATURE VERIFICATION FAILED/,
      );
      assert.deepEqual(
        await readFile(file),
        bytes,
        "verification must never modify the executable",
      );
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  },
);

test(
  "missing signing client fails before production signing",
  { skip: process.platform !== "win32" },
  async () => {
    await assert.rejects(
      preflightSigning({ config: configuration() }),
      /SIGNING NOT CONFIGURED/,
    );
  },
);

test(
  "real Microsoft-signed SDK executable validates with its exact publisher and rejects tampering",
  { skip: process.platform !== "win32", timeout: 180_000 },
  async (context) => {
    // This is a genuine embedded, timestamped signature, not a mocked trust result.
    // SDK availability is optional on developer hosts and mandatory on Windows release CI.
    const script =
      "Import-Module (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Security/Microsoft.PowerShell.Security.psd1'); $base = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'; $tool = Get-ChildItem -LiteralPath $base -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending | ForEach-Object { Join-Path $_.FullName 'x64/signtool.exe' } | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1; if (!$tool) { exit 0 }; $s = Get-AuthenticodeSignature -LiteralPath $tool; if ($s.Status -ne 'Valid' -or $s.SignatureType.ToString() -ne 'Authenticode') { exit 0 }; $c = New-Object System.Security.Cryptography.X509Certificates.X509Chain; $c.ChainPolicy.VerificationFlags = 'IgnoreNotTimeValid'; if (!$c.Build($s.SignerCertificate)) { exit 0 }; @{file=$tool;publisher=$s.SignerCertificate.Subject;root=$c.ChainElements[$c.ChainElements.Count-1].Certificate.Thumbprint} | ConvertTo-Json -Compress";
    const output = execFileSync(
      "powershell.exe",
      ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", script],
      { encoding: "utf8", windowsHide: true },
    ).trim();
    if (!output) {
      context.skip(
        "No verifiable embedded Microsoft SDK signature on this host.",
      );
      return;
    }
    const evidence = JSON.parse(output);
    assert.ok(existsSync(evidence.file));
    assert.match(evidence.publisher, /O=Microsoft Corporation/);
    const config = {
      ...configuration(),
      publisherSubject: evidence.publisher,
      trustedRootThumbprints: [evidence.root],
      signtoolPath: evidence.file,
    };
    const verified = await verifyExecutable(evidence.file, { config });
    assert.equal(verified.status, "Valid");
    assert.equal(verified.fileDigest, "SHA256");
    assert.equal(verified.timestampDigest, "SHA256");
    assert.equal(verified.timestampType, "RFC3161");
    await assert.rejects(
      verifyExecutable(evidence.file, {
        config: { ...config, publisherSubject: "CN=Wrong Publisher" },
      }),
      /publisher subject/,
    );
    await assert.rejects(
      verifyExecutable(evidence.file, {
        config: { ...config, trustedRootThumbprints: [rootFingerprint] },
      }),
      /configured public CA root/,
    );
    const directory = await mkdtemp(
      path.join(tmpdir(), "plc-signed-tamper-test-"),
    );
    try {
      const bytes = await readFile(evidence.file);
      // Change the COFF timestamp, covered by Authenticode, leaving the signature bytes present.
      bytes[bytes.readUInt32LE(60) + 8] ^= 1;
      const tampered = path.join(directory, "tampered.exe");
      await writeFile(tampered, bytes);
      await assert.rejects(
        verifyExecutable(tampered, { config }),
        /SIGNATURE VERIFICATION FAILED/,
      );
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  },
);
