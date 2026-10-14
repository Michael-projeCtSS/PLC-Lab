import { constants } from "node:fs";
import {
  copyFile,
  mkdir,
  mkdtemp,
  readFile,
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

export async function stageArtifactSet(
  sourceDir,
  outputDir,
  files,
  information,
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
    await writeFile(
      path.join(staging, "release-manifest.json"),
      `${JSON.stringify({ ...information, stagedAt: new Date().toISOString(), files: records }, null, 2)}\n`,
    );
    await writeFile(
      path.join(staging, "RUNTIME_REQUIREMENTS.txt"),
      "Local Windows release executable, not an installer.\r\nRequires Windows x64 and an installed Microsoft Edge WebView2 Runtime.\r\nNo WebView2 runtime is bundled. Keep any DLLs listed in release-manifest.json beside the executable.\r\nSee docs/HUMAN_TESTING.md for the validated environment and prerequisites.\r\n",
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
  const files = [{ source: sourceName, target: "PLC Lab.exe" }];
  const systemDependencies = [];
  for (const name of dependencies) {
    try {
      await stat(path.join(sourceDir, name));
      files.push({ source: name, target: name });
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
  const parent = path.join(root, "releases");
  await mkdir(parent, { recursive: true });
  within(await realpath(root), await realpath(parent));
  const destination = path.join(parent, "current");
  await stageArtifactSet(sourceDir, destination, files, {
    appVersion: application.version,
    profile: "release",
    architecture: "x64",
    sourceExecutable: path.relative(root, sourceExe),
    sourceModifiedAt: (await stat(sourceExe)).mtime.toISOString(),
    systemDependencies,
    webView2: "Installed Evergreen Runtime required; not bundled",
  });
  console.log(`Release ready: ${path.join(destination, "PLC Lab.exe")}`);
  console.log(
    "The copied executable is byte-identical to the successful Tauri release build.",
  );
  console.log(
    "Runtime requirements: Windows x64 with Microsoft Edge WebView2 installed. This is not an installer.",
  );
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
