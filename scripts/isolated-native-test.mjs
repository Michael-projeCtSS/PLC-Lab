// Prepares an unsigned development distribution for a separate Windows guest.
// Never starts PLC Lab.exe on the host and never changes host/guest security policy.
import { execFileSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { cp, lstat, mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { distributionEntries, verifyDistribution } from "./distribution.mjs";
import { DEVELOPMENT } from "./release-trust.mjs";
import { root } from "./toolchain.mjs";

const repository = "Michael-projeCtSS/PLC-Lab";
const branch = "claude/product-development-v2";
const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");
const xml = (value) =>
  value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
function powershell(script, args = []) {
  return execFileSync(
    "powershell.exe",
    [
      "-NoLogo",
      "-NoProfile",
      "-NonInteractive",
      "-File",
      path.join(root, "scripts", script),
      ...args,
    ],
    { encoding: "utf8", windowsHide: true, timeout: 120000 },
  );
}
export function nativeTestStatus() {
  if (process.platform !== "win32")
    return {
      sandbox: "NEEDS SETUP",
      edition: process.platform,
      hostSecurityModified: false,
    };
  return JSON.parse(powershell("native-test-status.ps1"));
}
function github(endpoint, binary = false) {
  return execFileSync("gh", ["api", `repos/${repository}/${endpoint}`], {
    ...(binary ? {} : { encoding: "utf8" }),
    windowsHide: true,
    timeout: 120000,
    maxBuffer: 128 * 1024 * 1024,
  });
}
async function verifyDevelopmentDistribution(directory) {
  const problems = await verifyDistribution(directory);
  if (problems.length) throw new Error(problems.join("; "));
  const manifest = JSON.parse(
    await readFile(path.join(directory, "resources/product.json"), "utf8"),
  );
  if (manifest.releaseKind !== DEVELOPMENT)
    throw new Error(
      "An explicitly labeled UNSIGNED_DEVELOPMENT_BUILD is required.",
    );
  if (
    manifest.fallback?.name !== "RUN_PLC_LAB.bat" ||
    manifest.fallback.sha256 !==
      sha(await readFile(path.join(directory, "RUN_PLC_LAB.bat")))
  )
    throw new Error(
      "Development fallback BAT hash differs from the inventory.",
    );
  return manifest;
}
export async function fetchNativeTest(runId) {
  if (!/^\d+$/.test(runId))
    throw new Error(
      "Provide a successful Canonical validation run ID from the trusted development branch.",
    );
  const run = JSON.parse(github(`actions/runs/${runId}`));
  if (
    run.conclusion !== "success" ||
    run.name !== "Canonical validation" ||
    run.head_branch !== branch ||
    !["push", "workflow_dispatch"].includes(run.event)
  )
    throw new Error(
      "The selected run is not successful trusted-branch canonical validation.",
    );
  const artifacts = JSON.parse(
    github(`actions/runs/${runId}/artifacts`),
  ).artifacts.filter(
    (artifact) =>
      /^plc-lab-windows-UNSIGNED-development-\d+$/.test(artifact.name) &&
      !artifact.expired,
  );
  if (artifacts.length !== 1)
    throw new Error(
      "Exactly one unexpired validated UNSIGNED development distribution is required.",
    );
  const artifact = artifacts[0];
  const metadata = JSON.parse(github(`actions/artifacts/${artifact.id}`));
  if (
    String(metadata.workflow_run?.id) !== runId ||
    metadata.workflow_run?.head_sha !== run.head_sha ||
    !/^sha256:[a-f0-9]{64}$/i.test(metadata.digest ?? "")
  )
    throw new Error(
      "Artifact source/run identity or archive SHA-256 is missing.",
    );
  const directory = path.join(
    root,
    "artifacts/native-test-downloads",
    `${runId}-${randomUUID()}`,
  );
  await mkdir(directory, { recursive: true });
  const bytes = github(`actions/artifacts/${artifact.id}/zip`, true);
  if (`sha256:${sha(bytes)}` !== metadata.digest.toLowerCase())
    throw new Error(
      "Unsigned developer archive SHA-256 mismatch; nothing was launched.",
    );
  const archive = path.join(directory, "validated.zip");
  await writeFile(archive, bytes, { flag: "wx" });
  const distribution = path.join(directory, "distribution");
  powershell("extract-native-test.ps1", [
    "-Archive",
    archive,
    "-Destination",
    distribution,
  ]);
  const manifest = await verifyDevelopmentDistribution(distribution);
  if (
    manifest.releaseKind !== DEVELOPMENT ||
    manifest.sourceCommit !== run.head_sha
  )
    throw new Error(
      "Distribution is not the explicitly unsigned build of the validated source commit.",
    );
  await writeFile(
    path.join(directory, "receipt.json"),
    JSON.stringify(
      {
        repository,
        runId,
        artifactId: artifact.id,
        archiveSha256: sha(bytes),
        sourceCommit: run.head_sha,
        classification: DEVELOPMENT,
        distribution,
      },
      null,
      2,
    ),
  );
  return {
    distribution,
    receipt: path.join(directory, "receipt.json"),
    sourceCommit: run.head_sha,
    classification: DEVELOPMENT,
  };
}
export async function prepareNativeTest(distribution) {
  distribution = path.resolve(distribution);
  const manifest = await verifyDevelopmentDistribution(distribution);
  const session = path.join(root, "artifacts/native-tests", randomUUID());
  const input = path.join(session, "input");
  const output = path.join(session, "output");
  const product = path.join(input, "product");
  await mkdir(product, { recursive: true });
  await mkdir(output);
  for (const entry of await distributionEntries(distribution))
    await cp(path.join(distribution, entry), path.join(product, entry), {
      recursive: true,
      errorOnExist: true,
      force: false,
      filter: async (file) => {
        if ((await lstat(file)).isSymbolicLink())
          throw new Error(
            "Test distributions must not contain symbolic links.",
          );
        return true;
      },
    });
  await verifyDevelopmentDistribution(product);
  await cp(
    path.join(root, "scripts/collect-native-guest.ps1"),
    path.join(input, "collect-native-guest.ps1"),
  );
  await writeFile(
    path.join(input, "README.txt"),
    "UNSIGNED NATIVE DEVELOPMENT BUILD\r\nOpen product\\PLC Lab.exe inside the guest only, where guest policy permits it.\r\nIf blocked, retain the warning and report BLOCKED. Do not alter guest policy.\r\nSee docs/LOCAL_TESTING.md for WebView2, test steps and evidence collection.\r\n",
  );
  const configuration = `<Configuration>\n<Networking>Disable</Networking>\n<ClipboardRedirection>Disable</ClipboardRedirection>\n<AudioInput>Disable</AudioInput>\n<VideoInput>Disable</VideoInput>\n<PrinterRedirection>Disable</PrinterRedirection>\n<MemoryInMB>4096</MemoryInMB>\n<MappedFolders>\n<MappedFolder><HostFolder>${xml(input)}</HostFolder><SandboxFolder>C:\\PLC-Lab-Test\\Input</SandboxFolder><ReadOnly>true</ReadOnly></MappedFolder>\n<MappedFolder><HostFolder>${xml(output)}</HostFolder><SandboxFolder>C:\\PLC-Lab-Test\\Results</SandboxFolder><ReadOnly>false</ReadOnly></MappedFolder>\n</MappedFolders>\n<LogonCommand><Command>explorer.exe C:\\PLC-Lab-Test\\Input</Command></LogonCommand>\n</Configuration>\n`;
  const sandboxFile = path.join(session, "PLC Lab native test.wsb");
  await writeFile(sandboxFile, configuration);
  const result = {
    session,
    sandboxFile,
    output,
    classification: DEVELOPMENT,
    sourceCommit: manifest.sourceCommit ?? null,
    integrity: "PASS",
    hostExecutableLaunched: false,
    environment: nativeTestStatus(),
  };
  await writeFile(
    path.join(session, "preparation.json"),
    JSON.stringify(result, null, 2),
  );
  return result;
}
if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  try {
    const [command, input] = process.argv.slice(2);
    if (command === "status")
      console.log(JSON.stringify(nativeTestStatus(), null, 2));
    else if (command === "fetch" && input)
      console.log(JSON.stringify(await fetchNativeTest(input), null, 2));
    else if (command === "prepare" && input)
      console.log(JSON.stringify(await prepareNativeTest(input), null, 2));
    else
      throw new Error(
        "Usage: node scripts/isolated-native-test.mjs status|fetch <validated-run-id>|prepare <unsigned-distribution-directory>",
      );
  } catch (error) {
    console.error(`ISOLATED NATIVE TEST: ${error.message}`);
    process.exitCode = 1;
  }
}
