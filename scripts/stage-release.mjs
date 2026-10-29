import { constants } from "node:fs";
import {
  copyFile,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  realpath,
  rename,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import path from "node:path";
import { createHash, randomUUID } from "node:crypto";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { root, toolchainEnvironment } from "./toolchain.mjs";
import {
  DEFAULT_PACK,
  DEFAULT_ZIP,
  PACK_NAME,
  verifyZip,
} from "./knowledge-pack.mjs";
import {
  APPLICATION,
  LEGACY_DIRS,
  LEGACY_FILES,
  PRODUCT_MANIFEST,
  RESOURCES,
  verifyDistribution,
} from "./distribution.mjs";

// The product root holds the one application executable `PLC Lab.exe` (the Tauri
// release binary itself, renamed) and `resources/` with the data it reads (ADR-0030).
export { APPLICATION };

function within(parent, target) {
  const relative = path.relative(parent, path.resolve(target));
  if (
    !relative ||
    relative.startsWith(`..${path.sep}`) ||
    relative === ".." ||
    path.isAbsolute(relative)
  ) {
    throw new Error(
      `Artifact operation would leave its intended directory: ${target}`,
    );
  }
}

async function hash(file) {
  return createHash("sha256")
    .update(await readFile(file))
    .digest("hex");
}

async function copyTree(source, destination, prefix, records) {
  await mkdir(destination, { recursive: true });
  for (const entry of (await readdir(source, { withFileTypes: true })).sort(
    (a, b) => a.name.localeCompare(b.name),
  )) {
    const from = path.join(source, entry.name);
    const to = path.join(destination, entry.name);
    const name = `${prefix}/${entry.name}`;
    if (entry.isSymbolicLink())
      throw new Error(`Symbolic links are not staged: ${from}`);
    if (entry.isDirectory()) await copyTree(from, to, name, records);
    else {
      await copyFile(from, to, constants.COPYFILE_EXCL);
      const sourceHash = await hash(from);
      if (sourceHash !== (await hash(to)))
        throw new Error(`Copied artifact verification failed: ${name}`);
      records.push({ name, sha256: sourceHash });
    }
  }
}

export async function stageArtifactSet(
  sourceDir,
  outputDir,
  files,
  information,
  trees = [],
) {
  await mkdir(path.dirname(outputDir), { recursive: true });
  const parent = await realpath(path.dirname(outputDir));
  const current = path.join(parent, path.basename(outputDir));
  const staging = await mkdtemp(path.join(parent, ".staging-"));
  const backup = path.join(parent, `.previous-${randomUUID()}`);
  for (const target of [current, staging, backup]) within(parent, target);
  let backedUp = false;
  let promoted = false;
  try {
    const records = [];
    for (const file of files) {
      for (const name of [file.source, file.target]) {
        if (
          !name ||
          path.basename(name) !== name ||
          name === "." ||
          name === ".."
        )
          throw new Error(
            "Artifact filenames must not contain directory paths.",
          );
      }
      const source = path.join(sourceDir, file.source);
      const destination = path.join(staging, file.target);
      if (!(await stat(source)).isFile())
        throw new Error(`Build artifact is not a file: ${source}`);
      await copyFile(source, destination, constants.COPYFILE_EXCL);
      const sourceHash = await hash(source);
      if (sourceHash !== (await hash(destination)))
        throw new Error(`Copied artifact verification failed: ${file.target}`);
      records.push({ name: file.target, sha256: sourceHash });
    }
    for (const tree of trees) {
      const parts = tree.target.split("/");
      if (parts.some((part) => !part || part === "." || part === ".."))
        throw new Error("Artifact tree targets must be relative folder names.");
      await copyTree(
        tree.source,
        path.join(staging, ...parts),
        tree.target,
        records,
      );
    }
    await writeFile(
      path.join(staging, PRODUCT_MANIFEST),
      `${JSON.stringify({ ...information, stagedAt: new Date().toISOString(), files: records }, null, 2)}\n`,
    );
    await writeFile(
      path.join(staging, "RUNTIME_REQUIREMENTS.txt"),
      [
        "PLC Lab portable release (not an installer).",
        "Start it by double-clicking PLC Lab.exe in the folder that contains this resources folder.",
        "Requires Windows 10/11 x64 and the Microsoft Edge WebView2 Runtime (preinstalled on current Windows; not bundled).",
        "Knowledge packs are read from resources\\knowledge-packs\\ and from %APPDATA%\\dev.plclab.desktop\\knowledge-packs.",
        "Keep PLC Lab.exe and this resources folder together; product.json lists every file with its SHA-256.",
        "",
      ].join("\r\n"),
    );
    try {
      await rename(current, backup);
      backedUp = true;
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
    try {
      await rename(staging, current);
      promoted = true;
    } catch (error) {
      if (backedUp) {
        await rename(backup, current);
        backedUp = false;
      }
      throw error;
    }
  } finally {
    // All recursive cleanup targets were resolved and checked within this output parent.
    if (!promoted) await rm(staging, { recursive: true, force: true });
    if (promoted && backedUp) {
      try {
        await rm(backup, { recursive: true });
      } catch (error) {
        console.warn(
          `Release is ready, but the previous artifact directory could not be removed: ${backup}: ${error.message}`,
        );
      }
    }
  }
}

function capture(command, args) {
  const result = spawnSync(command, args, {
    cwd: root,
    env: toolchainEnvironment(),
    encoding: "utf8",
  });
  if (result.error) throw result.error;
  if (result.status !== 0)
    throw new Error(
      `${command} failed (exit ${result.status}): ${result.stderr || result.stdout}`,
    );
  return result.stdout;
}

async function main() {
  if (process.platform !== "win32" || process.arch !== "x64")
    throw new Error(
      "Local release staging is currently validated for Windows x64 only.",
    );
  const metadata = JSON.parse(
    capture(process.execPath, [
      "scripts/rust.mjs",
      "metadata",
      "--no-deps",
      "--format-version",
      "1",
      "--locked",
    ]),
  );
  const application = metadata.packages.find(
    (item) => item.name === "plc-lab-desktop",
  );
  const binary = application?.targets.find((item) => item.kind.includes("bin"));
  if (!binary)
    throw new Error("Cargo metadata does not identify the desktop executable.");
  const sourceDir = path.join(metadata.target_directory, "release");
  const sourceName = `${binary.name}.exe`;
  const sourceExe = path.join(sourceDir, sourceName);
  const installer = path.join(
    process.env["ProgramFiles(x86)"] ?? "C:/Program Files (x86)",
    "Microsoft Visual Studio/Installer/vswhere.exe",
  );
  const dumpbin = capture(installer, [
    "-latest",
    "-products",
    "*",
    "-requires",
    "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
    "-find",
    "VC/Tools/MSVC/**/bin/Hostx64/x64/dumpbin.exe",
  ])
    .trim()
    .split(/\r?\n/)[0];
  if (!dumpbin)
    throw new Error(
      "MSVC dumpbin is required to audit release DLL dependencies. Install Desktop development with C++.",
    );
  const imports = capture(dumpbin, ["/dependents", sourceExe]);
  const dependencies = [...imports.matchAll(/^\s+([\w.-]+\.dll)\s*$/gim)].map(
    (match) => match[1],
  );
  if (dependencies.length === 0)
    throw new Error(
      "Could not identify executable DLL imports; release artifact set was not verified.",
    );
  // DLLs built with the application sit beside it; system DLLs are only recorded.
  const localDependencies = [];
  const systemDependencies = [];
  for (const name of dependencies) {
    try {
      await stat(path.join(sourceDir, name));
      localDependencies.push(name);
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
      if (!/^(api-ms-win-|ext-ms-win-)/i.test(name)) {
        await stat(
          path.join(process.env.SystemRoot ?? "C:/Windows", "System32", name),
        );
      }
      systemDependencies.push(name);
    }
  }
  // The default knowledge pack ships as the verified release archive's content.
  const pack = verifyZip(DEFAULT_ZIP, DEFAULT_PACK);
  if (pack.problems.length)
    throw new Error(
      `The knowledge pack archive does not verify: ${pack.problems.join("; ")}. Run npm run knowledge:zip.`,
    );
  const productRoot = await realpath(root);
  // The executable first: it cannot be replaced while PLC Lab runs, and nothing else
  // changes when that fails.
  await installFile(sourceExe, path.join(productRoot, APPLICATION));
  const installed = [];
  for (const name of localDependencies) {
    await installFile(path.join(sourceDir, name), path.join(productRoot, name));
    installed.push({ name, sha256: await hash(path.join(sourceDir, name)) });
  }
  await stageArtifactSet(
    sourceDir,
    path.join(productRoot, RESOURCES),
    [],
    {
      appVersion: application.version,
      profile: "release",
      architecture: "x64",
      application: { name: APPLICATION, sha256: await hash(sourceExe) },
      dependencies: installed,
      sourceExecutable: path.relative(root, sourceExe),
      sourceModifiedAt: (await stat(sourceExe)).mtime.toISOString(),
      systemDependencies,
      webView2: "Installed Evergreen Runtime required; not bundled",
      knowledgePack: {
        archive: path.relative(root, DEFAULT_ZIP).split(path.sep).join("/"),
        sha256: await hash(DEFAULT_ZIP),
      },
    },
    [{ source: DEFAULT_PACK, target: `knowledge-packs/${PACK_NAME}` }],
  );
  await removeLegacy(productRoot);
  const problems = await verifyDistribution(productRoot);
  if (problems.length)
    throw new Error(
      `The staged distribution is incomplete: ${problems.join("; ")}`,
    );
  console.log(`Release ready: ${path.join(productRoot, APPLICATION)}`);
  console.log(
    "Start PLC Lab by double-clicking PLC Lab.exe in this folder. It is the Tauri release build itself, byte-identical to the successful build.",
  );
  console.log(
    "Runtime requirements: Windows x64 with Microsoft Edge WebView2 installed. This is not an installer.",
  );
}

/** Replace a file in the product root atomically and verify its bytes. */
async function installFile(source, target) {
  const temporary = `${target}.${randomUUID()}.tmp`;
  await copyFile(source, temporary, constants.COPYFILE_EXCL);
  try {
    if ((await hash(source)) !== (await hash(temporary)))
      throw new Error("copy verification failed");
    await rename(temporary, target);
  } catch (error) {
    await rm(temporary, { force: true });
    throw new Error(
      `${path.basename(target)} could not be replaced (${error.message}). Close PLC Lab and try again.`,
      { cause: error },
    );
  }
}

/** Remove entry points of earlier distributions (the retired launcher and the separate
 * application folder); earlier builds generated them, never the user. */
async function removeLegacy(productRoot) {
  for (const name of LEGACY_FILES) {
    const file = path.join(productRoot, name);
    within(productRoot, file);
    await rm(file, { force: true });
  }
  for (const name of LEGACY_DIRS) {
    const dir = path.join(productRoot, name);
    within(productRoot, dir);
    await rm(dir, { recursive: true, force: true });
  }
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  main().catch((error) => {
    console.error(
      `Release staging failed: ${error.message}. Close the previous release before rebuilding.`,
    );
    process.exitCode = 1;
  });
}
