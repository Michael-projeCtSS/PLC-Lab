import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { root, toolchainEnvironment } from "./toolchain.mjs";

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
  const applicationNames =
    mode === "dev"
      ? ["plc-lab-desktop.exe"]
      : ["plc-lab-desktop.exe", "PLC Lab Desktop.exe", "PLC Lab.exe"];
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
        `Close the running ${name} window before this operation. Native validation/builds must be able to replace their executable files.`,
      );
  }
  console.log("Checking the pinned Rust toolchain...");
  runNode(["scripts/rust.mjs", "--version"]);
  runNode(["scripts/rust.mjs", "fmt", "--version"]);
  runNode(["scripts/rust.mjs", "clippy", "--version"]);
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

function main() {
  const mode = process.argv[2];
  const commands = {
    dev: "dev",
    checks: "validate",
    "build-release": "build:release",
  };
  if (!Object.hasOwn(commands, mode))
    throw new Error(`Unknown human-test mode: ${mode}`);
  const npmCli = prepare(mode);
  console.log(
    mode === "dev"
      ? "[DEV] Starting the real Tauri development application. Close its window to finish."
      : `Running canonical repository command: npm run ${commands[mode]}`,
  );
  runNode(["--use-system-ca", npmCli, "run", commands[mode]]);
  if (mode === "dev")
    console.log(
      "[DEV] Application closed successfully. The development server has stopped.",
    );
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  try {
    main();
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
