//! Repository-owned CI orchestration; existing validators remain unchanged.
import { spawn } from "node:child_process";
import { createWriteStream } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { finished } from "node:stream/promises";
import { pathToFileURL } from "node:url";
import { root } from "./toolchain.mjs";

const rust = (id, args) => ({
  id,
  command: process.execPath,
  args: ["scripts/rust.mjs", ...args],
});
const npm = (id, args, platform) =>
  platform === "win32"
    ? {
        id,
        command: process.env.ComSpec ?? "cmd.exe",
        args: ["/d", "/s", "/c", `npm.cmd ${args.join(" ")}`],
      }
    : { id, command: "npm", args };
/** The Tauri host needs WebView system libraries; the Linux lane validates everything else. */
const portable = ["--workspace", "--exclude", "plc-lab-desktop"];

export function validationPlan(lane, platform = process.platform) {
  if (lane === "windows")
    return [
      npm("canonical-check", ["run", "check"], platform),
      rust("rust-release-tests", [
        "test",
        "--release",
        "--locked",
        "--workspace",
      ]),
      npm("knowledge-pack", ["run", "knowledge:verify"], platform),
      npm("knowledge-install", ["run", "knowledge:install-check"], platform),
      npm("native-desktop", ["run", "test:desktop"], platform),
      npm("release-build", ["run", "build:release:native"], platform),
      npm("release-stage", ["run", "stage:release"], platform),
      npm("release-native", ["run", "test:desktop:release"], platform),
      npm("launcher-native", ["run", "test:launcher"], platform),
      npm("launcher-bat", ["run", "test:launcher:bat"], platform),
      npm("launcher-relocated", ["run", "test:launcher:relocated"], platform),
      npm("bat-relocated", ["run", "test:bat:relocated"], platform),
    ];
  if (lane === "core")
    return [
      rust("rust-format", ["fmt", "--all", "--", "--check"]),
      rust("core-clippy", [
        "clippy",
        "--locked",
        ...portable,
        "--all-targets",
        "--",
        "-D",
        "warnings",
      ]),
      rust("rust-debug-tests", ["test", "--locked", ...portable]),
      rust("rust-release-tests", [
        "test",
        "--release",
        "--locked",
        ...portable,
      ]),
      npm("frontend", ["run", "test:frontend"], platform),
      npm("architecture", ["run", "test:architecture"], platform),
      npm("knowledge-pack", ["run", "knowledge:verify"], platform),
      npm("knowledge-install", ["run", "knowledge:install-check"], platform),
      npm("npm-audit", ["audit", "--omit=dev", "--audit-level=high"], platform),
      {
        id: "cargo-deny",
        command: "cargo-deny",
        args: ["--locked", "check", "--hide-inclusion-graph"],
      },
    ];
  throw new Error(`Unknown validation lane: ${lane}`);
}

export function readToolchain(text) {
  const channel = text.match(/^channel\s*=\s*"([\w.-]+)"\s*$/m)?.[1];
  const profile = text.match(
    /^profile\s*=\s*"(minimal|default|complete)"\s*$/m,
  )?.[1];
  const componentsText = text.match(/^components\s*=\s*(\[[^\n]+\])\s*$/m)?.[1];
  if (!channel || !profile || !componentsText)
    throw new Error(
      "Unsupported or incomplete rust-toolchain.toml; CI cannot guess the toolchain.",
    );
  const components = JSON.parse(componentsText);
  if (
    !Array.isArray(components) ||
    components.some(
      (item) => typeof item !== "string" || !/^[\w-]+$/.test(item),
    ) ||
    !components.includes("rustfmt") ||
    !components.includes("clippy")
  )
    throw new Error("CI requires the declared rustfmt and clippy components.");
  return { channel, profile, components };
}

export async function runStages(
  stages,
  { directory, lane, cwd = root, display = true },
) {
  await mkdir(directory, { recursive: true });
  const report = {
    lane,
    platform: process.platform,
    node: process.version,
    revision: process.env.GITHUB_SHA ?? null,
    result: "RUNNING",
    stages: stages.map(({ id }) => ({ id, result: "NOT_RUN" })),
  };
  const save = () =>
    writeFile(
      path.join(directory, "result.json"),
      `${JSON.stringify(report, null, 2)}\n`,
    );
  await save();
  for (const [index, stage] of stages.entries()) {
    if (!/^[a-z][a-z0-9-]+$/.test(stage.id))
      throw new Error("CI stage IDs must be safe log basenames.");
    const log = createWriteStream(path.join(directory, `${stage.id}.log`));
    const command = [stage.command, ...stage.args].join(" ");
    log.write(`${command}\n`);
    if (display) console.log(`\n[${lane}] ${stage.id}: ${command}`);
    const child = spawn(stage.command, stage.args, {
      cwd,
      env: process.env,
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let launchError;
    const copy = (stream, output) =>
      stream.on("data", (chunk) => {
        log.write(chunk);
        if (display) output.write(chunk);
      });
    copy(child.stdout, process.stdout);
    copy(child.stderr, process.stderr);
    child.on("error", (error) => {
      launchError = error.message;
      log.write(`Process creation failed: ${launchError}\n`);
    });
    const outcome = await new Promise((resolve) =>
      child.on("close", (code, signal) =>
        resolve({
          exitCode: code ?? 1,
          signal,
          launchError: launchError ?? null,
        }),
      ),
    );
    log.end();
    await finished(log);
    const passed =
      outcome.exitCode === 0 && !outcome.launchError && !outcome.signal;
    report.stages[index] = {
      id: stage.id,
      result: passed ? "PASS" : "FAIL",
      ...outcome,
    };
    if (!passed) {
      report.result = "FAIL";
      await save();
      return report;
    }
    await save();
  }
  report.result = "PASS";
  await save();
  return report;
}

async function main(mode) {
  if (mode === "status") {
    console.log(
      "Local validation status:\nHOST POLICY MAY BLOCK GENERATED EXECUTABLES\nSee docs/validation/M1C_WINDOWS_CODE_INTEGRITY.md and docs/validation/CI_VALIDATION.md.",
    );
    return;
  }
  let stages;
  if (mode === "dependencies")
    stages = [npm("npm-ci", ["ci"], process.platform)];
  else if (mode === "setup-rust") {
    const toolchain = readToolchain(
      await readFile(path.join(root, "rust-toolchain.toml"), "utf8"),
    );
    stages = [
      {
        id: "rustup-install",
        command: "rustup",
        args: [
          "toolchain",
          "install",
          toolchain.channel,
          "--profile",
          toolchain.profile,
          ...toolchain.components.flatMap((component) => [
            "--component",
            component,
          ]),
        ],
      },
      rust("rust-version", ["--version"]),
    ];
  } else if (mode === "install-deny") {
    stages = [
      rust("cargo-deny-install", [
        "install",
        "cargo-deny",
        "--locked",
        "--version",
        "^0.18",
      ]),
    ];
  } else if (mode === "windows-prerequisites") {
    if (process.platform !== "win32")
      throw new Error("Windows prerequisites require Windows.");
    stages = [
      {
        id: "windows-prerequisites",
        command: "powershell.exe",
        args: ["-NoProfile", "-File", "scripts/ci-windows-prerequisites.ps1"],
      },
    ];
  } else {
    if (mode === "windows" && process.platform !== "win32")
      throw new Error("Canonical native validation requires Windows.");
    if (mode === "core" && process.platform !== "linux")
      throw new Error("The independent core lane requires Linux.");
    stages = validationPlan(mode);
  }
  const report = await runStages(stages, {
    directory: path.join(root, "artifacts", "ci", mode),
    lane: mode,
  });
  if (report.result !== "PASS") process.exitCode = 1;
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href
)
  main(process.argv[2]).catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
