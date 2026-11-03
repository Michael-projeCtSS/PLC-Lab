// The signed Windows catalog binds product.json to the final application and resources.
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import {
  lstat,
  mkdtemp,
  readFile,
  readdir,
  realpath,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import {
  loadSigningConfig,
  signCatalog,
  verifyCatalog,
} from "./windows-signing.mjs";

const execute = promisify(execFile);
const implementation = fileURLToPath(
  new URL("./windows-manifest.ps1", import.meta.url),
);
export const MANIFEST_CATALOG = "resources/product.cat";
const manifestName = "resources/product.json";
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");

function fail(message) {
  throw new Error(`SIGNATURE VERIFICATION FAILED: ${message}`);
}
function safeName(name) {
  return (
    typeof name === "string" &&
    !/[\\:\r\n\0"]/.test(name) &&
    name.split("/").every((part) => part && part !== "." && part !== "..")
  );
}

/** Every resource is covered, including requirements text and future extra resource files. */
export async function manifestMembers(productRoot) {
  productRoot = await realpath(path.resolve(productRoot));
  const members = new Map();
  const add = async (name, expectedHash) => {
    if (
      !safeName(name) ||
      name.toLowerCase() === MANIFEST_CATALOG.toLowerCase()
    )
      fail("unsafe or recursive catalog member path.");
    const file = path.join(productRoot, ...name.split("/"));
    const info = await lstat(file);
    const resolved = await realpath(file);
    if (
      !info.isFile() ||
      info.isSymbolicLink() ||
      !resolved.startsWith(`${productRoot}${path.sep}`)
    )
      fail(`catalog member must be a regular file inside the product: ${name}`);
    if (/[\r\n\0"]/.test(file))
      fail("catalog paths contain unsupported control characters or quotes.");
    const sha256 = hash(await readFile(file));
    if (
      expectedHash !== undefined &&
      (!/^[a-f0-9]{64}$/.test(expectedHash) || expectedHash !== sha256)
    )
      fail(
        `product manifest hash disagrees with final catalog member: ${name}`,
      );
    const key = name.toLowerCase();
    if (members.has(key) && members.get(key).name !== name)
      fail("catalog member names collide on Windows.");
    members.set(key, { name, file, sha256 });
  };
  await add(manifestName);
  const manifest = JSON.parse(
    await readFile(path.join(productRoot, manifestName), "utf8"),
  );
  if (manifest.application?.name !== "PLC Lab.exe")
    fail("product manifest must name the single PLC Lab.exe application.");
  await add(manifest.application.name, manifest.application.sha256);
  if (
    !Array.isArray(manifest.dependencies ?? []) ||
    !Array.isArray(manifest.files ?? [])
  )
    fail("product manifest file lists must be arrays.");
  for (const record of manifest.dependencies ?? []) {
    if (!safeName(record.name) || record.name.includes("/"))
      fail("dependency catalog members must be top-level filenames.");
    await add(record.name, record.sha256);
  }
  for (const record of manifest.files ?? [])
    await add(`resources/${record.name}`, record.sha256);
  if (manifest.fallback) {
    if (manifest.fallback.name !== "RUN_PLC_LAB.bat")
      fail("unexpected fallback startup filename.");
    await add(manifest.fallback.name, manifest.fallback.sha256);
  }
  const walk = async (directory, prefix) => {
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const name = `${prefix}/${entry.name}`;
      if (entry.isSymbolicLink())
        fail(`resource symbolic links are refused: ${name}`);
      if (entry.isDirectory())
        await walk(path.join(directory, entry.name), name);
      else if (name.toLowerCase() !== MANIFEST_CATALOG.toLowerCase())
        await add(name);
    }
  };
  await walk(path.join(productRoot, "resources"), "resources");
  try {
    await lstat(path.join(productRoot, "RUN_PLC_LAB.bat"));
    if (!manifest.fallback)
      fail(
        "the fallback BAT must be recorded in product.json before catalog creation.",
      );
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  return [...members.values()].sort((a, b) =>
    a.name.localeCompare(b.name, "en"),
  );
}

async function invoke(action, members, catalogPath, toolPath) {
  if (process.platform !== "win32")
    throw new Error(
      "SIGNING NOT CONFIGURED: Windows SDK MakeCat and SignTool are required for signed manifest catalogs.",
    );
  const temporary = await mkdtemp(path.join(tmpdir(), "plc-manifest-catalog-"));
  const inventory = path.join(temporary, "members.json");
  try {
    await writeFile(inventory, JSON.stringify(members), { flag: "wx" });
    const args = [
      "-NoLogo",
      "-NoProfile",
      "-NonInteractive",
      "-File",
      implementation,
      "-Action",
      action,
      "-InventoryPath",
      inventory,
      "-CatalogPath",
      catalogPath,
    ];
    if (toolPath) args.push("-ToolPath", path.resolve(toolPath));
    const { stdout } = await execute("powershell.exe", args, {
      encoding: "utf8",
      windowsHide: true,
      timeout: 180_000,
      maxBuffer: 1024 * 1024,
    });
    return JSON.parse(stdout.trim());
  } catch (error) {
    const message = `${error.stderr ?? ""}\n${error.stdout ?? ""}`.match(
      /(?:SIGNING NOT CONFIGURED|SIGNATURE VERIFICATION FAILED):[^\r\n]*/,
    )?.[0];
    // eslint-disable-next-line preserve-caught-error -- Provider/tool output is deliberately withheld from errors to prevent credential leakage.
    throw new Error(
      message ??
        "SIGNATURE VERIFICATION FAILED: Windows catalog operation failed; check Windows SDK tools and docs/WINDOWS_CODE_SIGNING.md.",
    );
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}

export async function createManifestCatalog(productRoot, { makecatPath } = {}) {
  const members = await manifestMembers(productRoot);
  const catalogPath = path.resolve(productRoot, MANIFEST_CATALOG);
  await invoke("Create", members, catalogPath, makecatPath);
  return catalogPath;
}

export async function signManifestCatalog(productRoot, { config } = {}) {
  config ??= await loadSigningConfig();
  const catalogPath = await createManifestCatalog(productRoot);
  await signCatalog(catalogPath, { config });
  return verifyManifestCatalog(productRoot, { config });
}

export async function verifyManifestCatalog(productRoot, { config } = {}) {
  config ??= await loadSigningConfig();
  const catalogPath = path.resolve(productRoot, MANIFEST_CATALOG);
  const evidence = await verifyCatalog(catalogPath, { config });
  const members = await manifestMembers(productRoot);
  await invoke("VerifyMembers", members, catalogPath, config.signtoolPath);
  return {
    ...evidence,
    catalogPath,
    members: members.map((member) => member.name),
  };
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const [action, productRoot] = process.argv.slice(2);
  try {
    const operations = {
      create: createManifestCatalog,
      sign: signManifestCatalog,
      verify: verifyManifestCatalog,
    };
    if (!operations[action] || !productRoot)
      throw new Error(
        "Usage: node scripts/windows-manifest.mjs create|sign|verify <product-root>",
      );
    console.log(JSON.stringify(await operations[action](productRoot), null, 2));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
