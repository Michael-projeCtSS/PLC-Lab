import {
  createWriteStream,
  existsSync,
  mkdirSync,
  readFileSync,
} from "node:fs";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { root, selectToolchain, toolchainEnvironment } from "./toolchain.mjs";

export function lockfileMatchesInstalled(expected, installed) {
  if (!expected.packages || !installed.packages) return false;
  for (const [name, actual] of Object.entries(installed.packages)) {
    const wanted = expected.packages[name];
    if (!wanted) return false;
    for (const field of ["version", "resolved", "integrity", "link"]) {
      if (wanted[field] !== actual[field]) return false;
    }
  }
  return true;
}

function runNode(args, options = {}) {
  const result = spawnSync(process.execPath, args, {
    cwd: root,
    env: toolchainEnvironment(),
    stdio: "inherit",
    ...options,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    const error = new Error(
      `Command failed (exit ${result.status ?? result.signal}): node ${args.join(" ")}`,
    );
    error.exitCode = result.status ?? 1;
    throw error;
  }
  return result;
}

/** npm ls reports exact root overrides inside workspaces as "invalid"; they are intended. */
export function onlyOverriddenProblems(problems, overrides) {
  return (
    Array.isArray(problems) &&
    problems.length > 0 &&
    problems.every((problem) => {
      const match = /^invalid: (@?[^@\s]+)@(\S+) /.exec(problem);
      return match !== null && overrides?.[match[1]] === match[2];
    })
  );
}

export function taskListContainsApplication(output, name) {
  return output
    .split(/\r?\n/)
    .some((line) => line.toLowerCase().startsWith(`"${name.toLowerCase()}",`));
}

function prepare(mode) {
  if (process.platform !== "win32")
    throw new Error("The human launchers currently support Windows only.");
  if (Number(process.versions.node.split(".")[0]) < 24)
    throw new Error("Install Node.js 24 or later with npm.");
  const npmCli = path.join(
    path.dirname(process.execPath),
    "node_modules/npm/bin/npm-cli.js",
  );
  if (!existsSync(npmCli))
    throw new Error(
      "npm is missing beside Node.js. Install the standard Node.js distribution with npm.",
    );
  // The development application shares the single-instance identity of PLC Lab.exe: with
  // either one open, a development start would hand over to it and quit.
  const applicationNames = native(mode)
    ? ["plc-lab-desktop.exe", "PLC Lab.exe"]
    : [
        "plc-lab-desktop.exe",
        "PLC Lab.exe",
        // Entry points of earlier distributions: staging removes their files.
        "PLC Lab Desktop.exe",
        "Launcher.exe",
      ];
  for (const name of applicationNames) {
    const query = spawnSync(
      path.join(
        process.env.SystemRoot ?? "C:/Windows",
        "System32/tasklist.exe",
      ),
      ["/FO", "CSV", "/NH", "/FI", `IMAGENAME eq ${name}`],
      { encoding: "utf8", windowsHide: true },
    );
    if (query.error) throw query.error;
    if (query.status !== 0)
      throw new Error(
        `Cannot check running PLC Lab applications: ${query.stderr || query.stdout}`,
      );
    if (taskListContainsApplication(query.stdout, name))
      throw new Error(
        native(mode)
          ? `PLC Lab is already running (${name}). Use its window, or close it and start again: a second start would only hand over to the running one.`
          : `Close the running ${name} window before this operation. Native validation/builds must be able to replace their executable files.`,
      );
  }
  console.log("Checking the pinned Rust toolchain...");
  const toolchain = selectToolchain();
  console.log(
    `Rust health check PASS: ${toolchain.pin}\nUsing ${toolchain.label}: ${toolchain.bin}`,
  );
  for (const report of toolchain.reports.filter((report) => !report.healthy))
    console.log(`${report.label} skipped:\n${report.problems.join("\n")}`);
  if (process.env.CARGO_BUILD_TARGET || process.env.CARGO_TARGET_DIR) {
    throw new Error(
      "Unset CARGO_BUILD_TARGET and CARGO_TARGET_DIR for the standard local human-test workflow. The existing native smoke test uses target/debug.",
    );
  }
  console.log("Checking JavaScript dependencies against package-lock.json...");
  const expected = JSON.parse(
    readFileSync(path.join(root, "package-lock.json"), "utf8"),
  );
  const installedPath = path.join(root, "node_modules/.package-lock.json");
  let matches =
    existsSync(installedPath) &&
    lockfileMatchesInstalled(
      expected,
      JSON.parse(readFileSync(installedPath, "utf8")),
    );
  if (matches) {
    const tree = spawnSync(
      process.execPath,
      ["--use-system-ca", npmCli, "ls", "--all", "--json"],
      {
        cwd: root,
        env: toolchainEnvironment(),
        encoding: "utf8",
      },
    );
    if (tree.error) throw tree.error;
    if (tree.status !== 0) {
      const diagnostics = JSON.parse(tree.stdout || "{}");
      if (diagnostics.error?.code !== "ELSPROBLEMS")
        throw new Error(
          tree.stderr || tree.stdout || "npm could not inspect dependencies.",
        );
      const manifest = JSON.parse(
        readFileSync(path.join(root, "package.json"), "utf8"),
      );
      if (!onlyOverriddenProblems(diagnostics.problems, manifest.overrides)) {
        console.log(
          "The installed dependency tree has missing or incompatible packages.",
        );
        matches = false;
      }
    }
  }
  if (!matches) {
    console.log(
      "Dependencies are missing or differ from the lockfile. Installing with npm ci...",
    );
    runNode(["--use-system-ca", npmCli, "ci"]);
  } else {
    console.log("Dependencies are already valid; no installation required.");
  }
  return npmCli;
}

/** Modes that start the native Tauri development application. */
export function native(mode) {
  return mode === "dev" || mode === "test";
}

/** Windows Code Integrity blocks of PLC Lab build outputs since `since` (read-only). */
export function codeIntegrityBlocks(since) {
  const script = [
    "$ErrorActionPreference = 'Stop'",
    `$since = [datetime]::Parse('${since.toISOString()}').ToLocalTime()`,
    "Get-WinEvent -FilterHashtable @{ LogName = 'Microsoft-Windows-CodeIntegrity/Operational'; Id = 3033, 3077; StartTime = $since } -ErrorAction SilentlyContinue |",
    "  ForEach-Object { if ($_.Message -match 'load (\\\\Device\\\\.*?\\.(exe|dll))') { \"$($_.Id) $($Matches[1])\" } } |",
    "  Where-Object { $_ -match 'PLC_Student|plc-lab' } | Select-Object -Unique",
  ].join("\n");
  const result = spawnSync(
    "powershell.exe",
    ["-NoProfile", "-NonInteractive", "-Command", script],
    { encoding: "utf8", windowsHide: true, timeout: 60_000 },
  );
  if (result.error || result.status !== 0) return null;
  return result.stdout
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

/** Explain a failed native start: a Windows execution block or an application error. */
function explainNativeFailure(started, log) {
  const blocks = codeIntegrityBlocks(started);
  if (blocks === null)
    console.error(
      "The Windows Code Integrity log could not be read, so a Windows execution block cannot be ruled out.",
    );
  else if (blocks.length) {
    console.error(
      [
        "============================================================",
        "WINDOWS BLOCKED A PLC LAB FILE (Code Integrity, Smart App Control).",
        ...blocks.map((b) => `  event ${b}`),
        "Windows refused to run this unsigned build output. PLC Lab does not change",
        "security settings and no script can lift this block. See",
        "docs/HUMAN_TESTING.md#smart-app-control for the legitimate options.",
        "============================================================",
      ].join("\n"),
    );
  } else
    console.error(
      "Windows Code Integrity recorded no block of PLC Lab files: the failure above is a build or application error.",
    );
  if (log) console.error(`Full output of this run: ${log}`);
}

/** Run the development application, mirroring its output into a log file. */
function runLogged(npmCli, script, log) {
  mkdirSync(path.dirname(log), { recursive: true });
  const file = createWriteStream(log);
  return new Promise((resolve, reject) => {
    const child = spawn(
      process.execPath,
      ["--use-system-ca", npmCli, "run", script],
      { cwd: root, env: toolchainEnvironment(), windowsHide: false },
    );
    for (const [from, to] of [
      [child.stdout, process.stdout],
      [child.stderr, process.stderr],
    ])
      from.on("data", (chunk) => {
        to.write(chunk);
        file.write(chunk);
      });
    child.on("error", reject);
    child.on("close", (code) => {
      file.end();
      if (code === 0) resolve();
      else {
        const error = new Error(
          `The development application ended with exit ${code}.`,
        );
        error.exitCode = code ?? 1;
        reject(error);
      }
    });
  });
}

async function main() {
  const mode = process.argv[2];
  const commands = {
    dev: "dev",
    test: "dev",
    checks: "validate",
    "build-release": "build:release",
    // UPDATE_PLC_LAB.bat: build this commit and install it as the root PLC Lab.exe.
    update: null,
  };
  if (!Object.hasOwn(commands, mode))
    throw new Error(`Unknown human-test mode: ${mode}`);
  const npmCli = prepare(mode);
  if (mode === "update") {
    const { update } = await import("./update-exe.mjs");
    await update({
      force: process.argv.includes("--force"),
      codeIntegrityBlocks,
    });
    return;
  }
  console.log(
    native(mode)
      ? "[DEV] Starting the native PLC Lab application (Tauri development build, real Rust backend). Its window opens when the build is ready; close it to finish."
      : `Running canonical repository command: npm run ${commands[mode]}`,
  );
  const started = new Date();
  const log =
    mode === "test"
      ? path.join(root, "artifacts", "test-plc-lab", "last-run.log")
      : null;
  try {
    if (log) await runLogged(npmCli, commands[mode], log);
    else runNode(["--use-system-ca", npmCli, "run", commands[mode]]);
  } catch (error) {
    if (native(mode)) explainNativeFailure(started, log);
    throw error;
  }
  if (native(mode))
    console.log(
      "[DEV] Application closed successfully. The development server has stopped.",
    );
  if (mode === "build-release") {
    const { verifyDistribution } = await import("./distribution.mjs");
    const problems = await verifyDistribution(root, { production: true });
    if (problems.length) throw new Error(problems.join("\n"));
    console.log(
      [
        "============================================================",
        "PLC LAB RELEASE BUILD: SUCCESS - SIGNED PRODUCTION RELEASE",
        `Application: ${path.join(root, "PLC Lab.exe")}`,
        "Double-click PLC Lab.exe to start.",
        "============================================================",
      ].join("\n"),
    );
  }
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  try {
    await main();
  } catch (error) {
    console.error(
      `ERROR: ${error instanceof Error ? error.message : String(error)}`,
    );
    console.error(
      "Review the output above and docs/HUMAN_TESTING.md. No successful result is being reported.",
    );
    process.exitCode = error.exitCode ?? 1;
  }
}
