import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  copyFile,
  mkdir,
  mkdtemp,
  readFile,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import {
  createManifestCatalog,
  manifestMembers,
  verifyManifestCatalog,
} from "../../scripts/windows-manifest.mjs";
import { verifyCatalog } from "../../scripts/windows-signing.mjs";

const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");
const config = () => ({
  provider: "azure-artifact-signing",
  publisherSubject: "CN=PLC Lab Test Publisher, C=PL",
  trustedRootThumbprints: ["1234567890ABCDEF1234567890ABCDEF12345678"],
  azure: {
    endpoint: "https://plc.codesigning.azure.net",
    accountName: "plc-lab-test",
    certificateProfileName: "public-trust",
  },
});

async function product() {
  const directory = await mkdtemp(
    path.join(tmpdir(), "PLC śruba's catalog test "),
  );
  await mkdir(path.join(directory, "resources/knowledge-packs/test"), {
    recursive: true,
  });
  const application =
    process.platform === "win32"
      ? await readFile(process.execPath)
      : Buffer.from("application");
  const knowledge = Buffer.from('{"id":"test"}');
  const bat = Buffer.from('@echo off\r\nstart "" "%~dp0PLC Lab.exe"\r\n');
  await writeFile(path.join(directory, "PLC Lab.exe"), application);
  await writeFile(path.join(directory, "RUN_PLC_LAB.bat"), bat);
  await writeFile(
    path.join(directory, "resources/knowledge-packs/test/pack.json"),
    knowledge,
  );
  await writeFile(
    path.join(directory, "resources/RUNTIME_REQUIREMENTS.txt"),
    "Windows x64",
  );
  const manifest = {
    application: { name: "PLC Lab.exe", sha256: sha(application) },
    dependencies: [],
    files: [{ name: "knowledge-packs/test/pack.json", sha256: sha(knowledge) }],
    fallback: { name: "RUN_PLC_LAB.bat", sha256: sha(bat) },
  };
  await writeFile(
    path.join(directory, "resources/product.json"),
    JSON.stringify(manifest),
  );
  return { directory, manifest };
}

test("catalog inventory binds manifest, application, every resource and fallback BAT", async () => {
  const { directory } = await product();
  try {
    assert.deepEqual(
      (await manifestMembers(directory)).map((entry) => entry.name),
      [
        "PLC Lab.exe",
        "resources/knowledge-packs/test/pack.json",
        "resources/product.json",
        "resources/RUNTIME_REQUIREMENTS.txt",
        "RUN_PLC_LAB.bat",
      ].sort((a, b) => a.localeCompare(b, "en")),
    );
    await writeFile(
      path.join(directory, "resources/product.cat"),
      "catalog excluded from its own inventory",
    );
    assert.equal((await manifestMembers(directory)).length, 5);
    await writeFile(
      path.join(directory, "resources/new-resource.json"),
      "new resource",
    );
    assert.equal(
      (await manifestMembers(directory)).length,
      6,
      "additional resources require catalog membership too",
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("catalog inventory rejects final executable tampering, unsafe paths and unrecorded BAT", async () => {
  const { directory, manifest } = await product();
  try {
    await writeFile(
      path.join(directory, "PLC Lab.exe"),
      "changed after signing",
    );
    await assert.rejects(manifestMembers(directory), /manifest hash disagrees/);
    manifest.application.sha256 = sha("changed after signing");
    manifest.files[0].name = "../../outside.json";
    await writeFile(
      path.join(directory, "resources/product.json"),
      JSON.stringify(manifest),
    );
    await assert.rejects(manifestMembers(directory), /unsafe or recursive/);
    manifest.files = [];
    delete manifest.fallback;
    await writeFile(
      path.join(directory, "resources/product.json"),
      JSON.stringify(manifest),
    );
    await assert.rejects(
      manifestMembers(directory),
      /fallback BAT must be recorded/,
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test(
  "real MakeCat creates SHA-256 catalog in Unicode paths; unsigned catalog fails closed",
  { skip: process.platform !== "win32", timeout: 180_000 },
  async () => {
    const { directory } = await product();
    try {
      const catalog = await createManifestCatalog(directory);
      assert.ok((await readFile(catalog)).length > 100);
      await assert.rejects(createManifestCatalog(directory), /already exists/);
      await assert.rejects(
        verifyManifestCatalog(directory, { config: config() }),
        /SIGNATURE VERIFICATION FAILED/,
      );
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  },
);

test(
  "missing MakeCat fails with an actionable configuration error",
  { skip: process.platform !== "win32" },
  async () => {
    const { directory } = await product();
    try {
      await assert.rejects(
        createManifestCatalog(directory, {
          makecatPath: path.join(directory, "missing-makecat.exe"),
        }),
        /SIGNING NOT CONFIGURED.*makecat/,
      );
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  },
);

test(
  "genuine Microsoft-signed catalog passes real publisher/root/timestamp verification",
  { skip: process.platform !== "win32", timeout: 180_000 },
  async (context) => {
    const script =
      "Import-Module (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Security/Microsoft.PowerShell.Security.psd1'); $base=Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'; $catalog=Get-ChildItem -LiteralPath $base -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending | ForEach-Object { Join-Path $_.FullName 'x64/virtualstoragevss.cat' } | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1; if(!$catalog){exit 0}; $s=Get-AuthenticodeSignature -LiteralPath $catalog; if($s.Status -ne 'Valid'){exit 0}; $chain=New-Object System.Security.Cryptography.X509Certificates.X509Chain; $chain.ChainPolicy.VerificationFlags='IgnoreNotTimeValid'; if(!$chain.Build($s.SignerCertificate)){exit 0}; @{file=$catalog;publisher=$s.SignerCertificate.Subject;root=$chain.ChainElements[$chain.ChainElements.Count-1].Certificate.Thumbprint} | ConvertTo-Json -Compress";
    const output = execFileSync(
      "powershell.exe",
      ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", script],
      { encoding: "utf8", windowsHide: true },
    ).trim();
    if (!output) {
      context.skip("No signed SDK catalog on this host.");
      return;
    }
    const fixture = JSON.parse(output);
    assert.match(fixture.publisher, /O=Microsoft Corporation/);
    const settings = {
      ...config(),
      publisherSubject: fixture.publisher,
      trustedRootThumbprints: [fixture.root],
    };
    const evidence = await verifyCatalog(fixture.file, { config: settings });
    assert.equal(evidence.status, "Valid");
    assert.equal(evidence.timestampType, "RFC3161");
    const directory = await mkdtemp(
      path.join(tmpdir(), "plc-real-catalog-tamper-"),
    );
    try {
      const tampered = path.join(directory, "tampered.cat");
      await copyFile(fixture.file, tampered);
      const bytes = await readFile(tampered);
      bytes[Math.floor(bytes.length / 2)] ^= 1;
      await writeFile(tampered, bytes);
      await assert.rejects(
        verifyCatalog(tampered, { config: settings }),
        /SIGNATURE VERIFICATION FAILED/,
      );
      const sys = path.join(
        path.dirname(fixture.file),
        "virtualstoragevss.sys",
      );
      const inf = path.join(
        path.dirname(fixture.file),
        "virtualstoragevss.inf",
      );
      const members = [];
      for (const source of [sys, inf]) {
        const destination = path.join(directory, path.basename(source));
        await copyFile(source, destination);
        members.push({
          name: path.basename(source),
          file: destination,
          sha256: sha(await readFile(destination)),
        });
      }
      const inventory = path.join(directory, "members.json");
      const implementation = path.resolve("scripts/windows-manifest.ps1");
      await writeFile(inventory, JSON.stringify(members));
      const verifyMembers = () =>
        execFileSync(
          "powershell.exe",
          [
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-File",
            implementation,
            "-Action",
            "VerifyMembers",
            "-InventoryPath",
            inventory,
            "-CatalogPath",
            fixture.file,
          ],
          { encoding: "utf8", windowsHide: true, stdio: "pipe" },
        );
      assert.equal(
        JSON.parse(verifyMembers()).members,
        2,
        "genuine signed catalog authenticates both actual covered files",
      );
      await writeFile(members[1].file, "tampered catalog member");
      members[1].sha256 = sha("tampered catalog member");
      await writeFile(inventory, JSON.stringify(members));
      assert.throws(
        verifyMembers,
        /signed manifest catalog does not authenticate member/,
      );
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  },
);
