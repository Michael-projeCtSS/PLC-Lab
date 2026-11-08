// UPDATE_PLC_LAB.bat: build the current commit and install it as the root PLC Lab.exe,
// through the existing release pipeline (release build, stage-release.mjs --development,
// verifyDistribution) and only after proving it runs on this machine.
//
// It never installs over a signed production release, never builds from uncommitted
// changes (the installed build must name its exact source commit), keeps a verified
// backup of the previous distribution, restores it when the new one fails its native
// check, and reports Windows Code Integrity (Smart App Control) blocks without trying to
// avoid them. TEST_PLC_LAB.bat stays the way to run the current source.
import { createHash, randomUUID } from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import {
  copyFile,
  cp,
  mkdir,
  readFile,
  readdir,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { APPLICATION, RESOURCES, verifyDistribution } from "./distribution.mjs";
import { DEVELOPMENT, PRODUCTION } from "./release-trust.mjs";
import { root, toolchainEnvironment } from "./toolchain.mjs";

export const BACKUPS = path.join(root, "artifacts", "plc-lab-backups");
/** Backups the updater keeps; older ones it made itself are removed. */
export const KEEP_BACKUPS = 3;

/**
 * What to do, from the repository and the installed distribution.
 * - dirty: uncommitted or untracked source changes (git status --porcelain is not empty)
 * - manifest: the installed resources/product.json, or null
 * - problems: verifyDistribution findings for the installed root
 */
export function decide({ head, dirty, manifest, problems, force = false }) {
  if (!head)
    return { action: "refuse", reason: "The repository HEAD cannot be read." };
  if (dirty)
    return {
      action: "refuse",
      reason:
        "The checkout has uncommitted changes. The installed PLC Lab.exe must name its exact source commit: commit or stash them first. TEST_PLC_LAB.bat runs uncommitted work.",
    };
  if (manifest && (manifest.releaseKind === PRODUCTION || manifest.signing))
    return {
      action: "refuse",
      reason:
        "A signed production release is installed here. An unsigned development build never replaces it; use a separate development checkout.",
    };
  if (
    !force &&
    manifest?.releaseKind === DEVELOPMENT &&
    manifest.sourceCommit === head &&
    problems.length === 0
  )
    return {
      action: "current",
      reason: `PLC Lab.exe already contains ${head.slice(0, 12)} and its files verify.`,
    };
  return {
    action: "update",
    reason: manifest
      ? `Installed: ${manifest.sourceCommit?.slice(0, 12) ?? "unknown"}, repository: ${head.slice(0, 12)}.`
      : "No staged PLC Lab.exe is installed yet.",
  };
}

/** The updater's own backups to delete so that only the newest `keep` remain. */
export function backupsToPrune(names, keep = KEEP_BACKUPS) {
  return names
    .filter((name) =>
      /^\d{8}T\d{6}Z-[0-9a-f]{7,12}$|^\d{8}T\d{6}Z-unknown$/.test(name),
    )
    .sort()
    .reverse()
    .slice(keep);
}

function git(args) {
  const result = spawnSync("git", args, { cwd: root, encoding: "utf8" });
  if (result.error || result.status !== 0) return null;
  return result.stdout.trim();
}

async function sha256(file) {
  return createHash("sha256")
    .update(await readFile(file))
    .digest("hex");
}

async function readManifest(directory) {
  try {
    return JSON.parse(
      await readFile(path.join(directory, RESOURCES, "product.json"), "utf8"),
    );
  } catch (error) {
    if (error.code === "ENOENT") return null;
    throw error;
  }
}

function run(command, args, label) {
  console.log(`\n> ${label}`);
  const result = spawnSync(command, args, {
    cwd: root,
    env: toolchainEnvironment(),
    stdio: "inherit",
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    const error = new Error(`${label} failed (exit ${result.status}).`);
    error.exitCode = result.status ?? 1;
    throw error;
  }
}

/** The files of a staged distribution (executable, fallback BAT, DLLs, resources). */
function distributionFiles(manifest) {
  return [
    APPLICATION,
    ...(manifest?.fallback?.name ? [manifest.fallback.name] : []),
    ...(manifest?.dependencies ?? []).map((d) => d.name),
  ];
}

async function backup(manifest, problems) {
  try {
    await stat(path.join(root, APPLICATION));
  } catch {
    return null;
  }
  const stamp = new Date()
    .toISOString()
    .replace(/[-:]/g, "")
    .replace(/\.\d+Z$/, "Z");
  const name = `${stamp}-${manifest?.sourceCommit?.slice(0, 12) ?? "unknown"}`;
  const target = path.join(BACKUPS, name);
  await mkdir(target, { recursive: true });
  for (const file of distributionFiles(manifest))
    await copyFile(path.join(root, file), path.join(target, file)).catch(
      (error) => {
        if (error.code !== "ENOENT") throw error;
      },
    );
  await cp(path.join(root, RESOURCES), path.join(target, RESOURCES), {
    recursive: true,
    errorOnExist: true,
  }).catch((error) => {
    if (error.code !== "ENOENT") throw error;
  });
  const verified =
    problems.length === 0 &&
    (await verifyDistribution(target, { production: false })).length === 0;
  await writeFile(
    path.join(target, "BACKUP.json"),
    `${JSON.stringify({ createdBy: "UPDATE_PLC_LAB.bat", sourceCommit: manifest?.sourceCommit ?? null, releaseKind: manifest?.releaseKind ?? null, verified }, null, 2)}\n`,
  );
  for (const old of backupsToPrune(await readdir(BACKUPS)))
    await rm(path.join(BACKUPS, old), { recursive: true, force: true });
  return { directory: target, verified };
}

/** Put a verified backup back in place (the same atomic installer, unsigned mode). */
async function restore(saved) {
  const { installPreparedDistribution } =
    await import("./install-signed-release.mjs");
  const candidate = path.join(BACKUPS, `.restore-${randomUUID()}`);
  await cp(saved.directory, candidate, { recursive: true });
  await rm(path.join(candidate, "BACKUP.json"), { force: true });
  try {
    await installPreparedDistribution(candidate, root, { production: false });
  } finally {
    await rm(candidate, { recursive: true, force: true });
  }
}

async function freePort() {
  return new Promise((resolve) => {
    const server = net.createServer();
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });
}

/**
 * Does Windows let this file run? A blocked executable never starts (or Code Integrity
 * records a block); a started one reports its own problems and exits. Read-only probe.
 */
async function windowsLetsItRun(executable) {
  const { codeIntegrityBlocks } = await import("./human-test.mjs");
  const started = new Date();
  const port = await freePort();
  const scratch = path.join(root, "artifacts", "update-probe", randomUUID());
  const child = spawn(executable, [], {
    cwd: path.dirname(executable),
    env: {
      ...process.env,
      PLC_LAB_NO_DIALOG: "1",
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
      WEBVIEW2_USER_DATA_FOLDER: path.join(scratch, "webview"),
      PLC_LAB_DATA_DIR: path.join(scratch, "data"),
    },
    stdio: "ignore",
  });
  let spawnError = null;
  child.on("error", (error) => (spawnError = error));
  let exited = false;
  child.on("exit", () => (exited = true));
  await delay(8000);
  if (!exited) spawnSync("taskkill", ["/PID", String(child.pid), "/T", "/F"]);
  await delay(1000);
  await rm(scratch, { recursive: true, force: true }).catch(() => undefined);
  const blocks = (codeIntegrityBlocks(started) ?? []).filter((b) =>
    b.toLowerCase().includes(path.basename(executable).toLowerCase()),
  );
  return {
    blocked: blocks.length > 0 || Boolean(spawnError),
    blocks,
    spawnError,
  };
}

function report(lines) {
  console.log(["", "=".repeat(60), ...lines, "=".repeat(60)].join("\n"));
}

export async function update({ force = false } = {}) {
  const head = git(["rev-parse", "HEAD"]);
  const status = git(["status", "--porcelain"]);
  const manifest = await readManifest(root);
  const problems = manifest
    ? await verifyDistribution(root, { production: false })
    : ["no staged distribution"];
  const decision = decide({
    head,
    dirty: status === null || status.length > 0,
    manifest,
    problems,
    force,
  });
  console.log(`Repository HEAD: ${head ?? "unknown"}`);
  console.log(
    `Installed PLC Lab.exe: ${manifest ? `${manifest.releaseKind} from ${manifest.sourceCommit}` : "none (no resources/product.json)"}`,
  );
  console.log(decision.reason);
  if (decision.action === "refuse") {
    const error = new Error(decision.reason);
    error.exitCode = 2;
    throw error;
  }
  if (decision.action === "current") {
    report([
      "PLC LAB IS UP TO DATE",
      `Source commit: ${head}`,
      `Kind: ${manifest.releaseKind} (not code-signed)`,
    ]);
    return { installed: false, head };
  }

  run(
    process.execPath,
    ["scripts/knowledge-pack.mjs", "verify"],
    "Verify the knowledge pack archive matches the pack",
  );
  run(
    process.execPath,
    ["scripts/native.mjs", "build", "--no-bundle", "--", "--locked"],
    "Build the native release application (Tauri, Rust, React)",
  );
  const built = path.join(root, "target", "release", "plc-lab-desktop.exe");
  const builtHash = await sha256(built);

  console.log(
    "\n> Check that Windows lets the new executable run (before replacing anything)",
  );
  const probe = await windowsLetsItRun(built);
  if (probe.blocked) {
    report([
      "WINDOWS BLOCKED THE NEW PLC LAB BUILD (Code Integrity, Smart App Control).",
      ...probe.blocks.map((b) => `  event ${b}`),
      ...(probe.spawnError
        ? [`  start error: ${probe.spawnError.message}`]
        : []),
      "The root PLC Lab.exe was NOT changed. PLC Lab does not change security settings",
      "and no script can lift this block. TEST_PLC_LAB.bat still runs the current source;",
      "see docs/HUMAN_TESTING.md#smart-app-control.",
    ]);
    const error = new Error("Windows blocked the new executable.");
    error.exitCode = 3;
    throw error;
  }

  console.log("\n> Back up the installed distribution");
  const saved = await backup(manifest, problems);
  if (saved)
    console.log(
      `Backup: ${saved.directory} (${saved.verified ? "verified known-good copy" : "copy of files that did not verify"})`,
    );

  run(
    process.execPath,
    ["scripts/stage-release.mjs", "--development"],
    "Stage and install atomically (unsigned development build)",
  );

  console.log("\n> Verify the installed files");
  const installed = await readManifest(root);
  const after = await verifyDistribution(root, { production: false });
  const rootHash = await sha256(path.join(root, APPLICATION));
  const mismatch = [
    ...after,
    ...(installed?.releaseKind !== DEVELOPMENT
      ? ["not labeled as an unsigned development build"]
      : []),
    ...(installed?.sourceCommit !== head
      ? [`manifest names ${installed?.sourceCommit}, not ${head}`]
      : []),
    ...(rootHash !== builtHash
      ? ["PLC Lab.exe differs from the built executable"]
      : []),
    ...(installed?.application?.sha256 !== rootHash
      ? ["manifest hash differs from PLC Lab.exe"]
      : []),
  ];

  let smoke = mismatch.length === 0;
  if (smoke) {
    console.log(
      "\n> Native check of the installed PLC Lab.exe (real window, Rust backend)",
    );
    const result = spawnSync(
      process.execPath,
      [
        "tests/e2e/desktop-smoke.mjs",
        "--exe",
        APPLICATION,
        "--build",
        "release",
        "--out",
        "update-smoke",
      ],
      { cwd: root, env: toolchainEnvironment(), stdio: "inherit" },
    );
    smoke = !result.error && result.status === 0;
  }
  if (!smoke) {
    const { codeIntegrityBlocks } = await import("./human-test.mjs");
    const blocks =
      codeIntegrityBlocks(new Date(Date.now() - 15 * 60_000)) ?? [];
    const restored = saved?.verified
      ? await restore(saved).then(() => true)
      : false;
    report([
      "PLC LAB UPDATE FAILED - the new PLC Lab.exe is NOT confirmed usable.",
      ...mismatch.map((m) => `  ${m}`),
      ...blocks.map((b) => `  Code Integrity event ${b}`),
      restored
        ? `The previous distribution (${saved.directory}) was restored.`
        : "No verified previous distribution was available to restore.",
      "TEST_PLC_LAB.bat still runs the current source.",
    ]);
    const error = new Error(
      "The installed executable failed its native check.",
    );
    error.exitCode = 4;
    throw error;
  }
  report([
    "PLC LAB UPDATED AND VERIFIED",
    `Source commit: ${head}`,
    `Kind: ${DEVELOPMENT} (not code-signed)`,
    `PLC Lab.exe SHA-256: ${rootHash}`,
    "Native check: PASS (window, Rust backend, program, simulation)",
    ...(saved ? [`Previous version kept in: ${saved.directory}`] : []),
  ]);
  return { installed: true, head };
}
