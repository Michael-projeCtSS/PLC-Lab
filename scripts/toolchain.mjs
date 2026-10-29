import { existsSync, readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { homedir } from "node:os";
import { fileURLToPath } from "node:url";
import path from "node:path";

export const root = fileURLToPath(new URL("../", import.meta.url));

// Shared with explicit CI setup. Ordinary build commands never install Rust.
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
  if (!/^\d+\.\d+\.\d+$/.test(channel))
    throw new Error(
      "Rust must be pinned to an exact version in rust-toolchain.toml.",
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

function hostTriple(platform, arch) {
  const cpu = { x64: "x86_64", arm64: "aarch64" }[arch];
  const system = {
    win32: "pc-windows-msvc",
    linux: "unknown-linux-gnu",
    darwin: "apple-darwin",
  }[platform];
  if (!cpu || !system)
    throw new Error(`Unsupported Rust host: ${platform}/${arch}`);
  return `${cpu}-${system}`;
}

function candidateEnvironment(candidate, environment, pin, platform) {
  const selected = { ...environment };
  const pathKey =
    Object.keys(environment).find((key) => key.toUpperCase() === "PATH") ??
    "PATH";
  const bin = path.join(candidate.rustupHome, "toolchains", pin, "bin");
  const suffix = platform === "win32" ? ".exe" : "";
  selected.CARGO_HOME = candidate.cargoHome;
  selected.RUSTUP_HOME = candidate.rustupHome;
  selected.RUSTUP_TOOLCHAIN = pin;
  // Defence in depth if a third-party command explicitly calls a rustup proxy.
  selected.RUSTUP_AUTO_INSTALL = "0";
  selected.RUSTC = path.join(bin, `rustc${suffix}`);
  selected.RUSTDOC = path.join(bin, `rustdoc${suffix}`);
  selected[pathKey] = [
    bin,
    path.join(candidate.cargoHome, "bin"),
    environment[pathKey] ?? "",
  ].join(path.delimiter);
  return selected;
}

/** Check actual installed binaries, never the cargo/bin rustup proxies. */
export function inspectToolchain(
  candidate,
  { pin, channel, host, environment, platform, run = spawnSync },
) {
  const bin = path.join(candidate.rustupHome, "toolchains", pin, "bin");
  const suffix = platform === "win32" ? ".exe" : "";
  if (!existsSync(candidate.rustupHome) && !existsSync(candidate.cargoHome))
    return {
      ...candidate,
      healthy: false,
      problems: [`Not installed: ${path.dirname(bin)}`],
    };
  const commands = [
    "rustc",
    "cargo",
    "rustfmt",
    "clippy-driver",
    "cargo-fmt",
    "cargo-clippy",
    "rustdoc",
  ];
  const missing = commands
    .map((name) => path.join(bin, `${name}${suffix}`))
    .filter((file) => !existsSync(file));
  if (missing.length)
    return {
      ...candidate,
      healthy: false,
      problems: missing.map((file) => `Missing: ${file}`),
    };
  const env = candidateEnvironment(candidate, environment, pin, platform);
  let compilerHash;
  for (const command of commands) {
    // cargo-fmt/cargo-clippy are entry points for the checked components.
    if (command === "cargo-fmt" || command === "cargo-clippy") continue;
    const file = path.join(bin, `${command}${suffix}`);
    const result = run(file, [command === "rustc" ? "-vV" : "--version"], {
      cwd: root,
      env,
      encoding: "utf8",
      windowsHide: true,
      timeout: 5_000,
    });
    const output = result.stdout?.trim() ?? "";
    let problem;
    if (result.error || result.status !== 0)
      problem =
        result.error?.message ??
        `exit ${result.status}: ${result.stderr || output}`;
    else if (command === "rustc") {
      compilerHash = output.match(/^commit-hash: ([a-f0-9]+)$/m)?.[1];
      if (
        !output.split(/\r?\n/).includes(`release: ${channel}`) ||
        !output.split(/\r?\n/).includes(`host: ${host}`) ||
        !compilerHash
      )
        problem = `Expected Rust ${channel} for ${host}, got ${output}`;
    } else if (command === "cargo" || command === "rustdoc") {
      if (!output.startsWith(`${command} ${channel} `))
        problem = `Expected ${command} ${channel}, got ${output}`;
    } else {
      // rustfmt/clippy have their own version numbers; their Rust commit identifies
      // which compiler release they accompany.
      const hash = output.match(/\(([a-f0-9]{7,40})\s/)?.[1];
      if (!hash || !compilerHash.startsWith(hash))
        problem = `Component does not match pinned Rust commit ${compilerHash}: ${output}`;
    }
    if (problem)
      return {
        ...candidate,
        healthy: false,
        problems: [`Unusable: ${file}: ${problem}`],
      };
  }
  return { ...candidate, healthy: true, problems: [], environment: env, bin };
}

export function selectToolchain({
  directory = root,
  environment = process.env,
  userHome = homedir(),
  platform = process.platform,
  arch = process.arch,
  run = spawnSync,
} = {}) {
  const { channel } = readToolchain(
    readFileSync(path.join(directory, "rust-toolchain.toml"), "utf8"),
  );
  const host = hostTriple(platform, arch);
  const pin = `${channel}-${host}`;
  const candidates = [
    {
      label: "Project-local toolchain",
      cargoHome: path.join(directory, ".tools/cargo"),
      rustupHome: path.join(directory, ".tools/rustup"),
    },
    {
      label: "Configured system pinned toolchain",
      cargoHome: environment.CARGO_HOME ?? path.join(userHome, ".cargo"),
      rustupHome: environment.RUSTUP_HOME ?? path.join(userHome, ".rustup"),
    },
    {
      label: "User pinned toolchain",
      cargoHome: path.join(userHome, ".cargo"),
      rustupHome: path.join(userHome, ".rustup"),
    },
  ];
  const reports = [];
  const visited = new Set();
  for (const candidate of candidates) {
    const key = path.resolve(candidate.rustupHome);
    if (visited.has(key)) continue;
    visited.add(key);
    const report = inspectToolchain(candidate, {
      pin,
      channel,
      host,
      environment,
      platform,
      run,
    });
    reports.push(report);
    if (report.healthy) return { ...report, pin, reports };
  }
  throw new Error(
    [
      "============================================================",
      "PLC LAB BUILD BLOCKED",
      `Pinned Rust: ${pin}`,
      ...reports.flatMap((report) => [
        `${report.label}: ${existsSync(report.rustupHome) || existsSync(report.cargoHome) ? "BROKEN / INCOMPLETE" : "NOT FOUND"}`,
        ...report.problems,
      ]),
      "Build stopped before compilation. No Rust installation or repair was attempted.",
      "See docs/HUMAN_TESTING.md#rust-toolchain-preflight for recovery.",
      "============================================================",
    ].join("\n"),
  );
}

export function toolchainEnvironment(options) {
  return selectToolchain(options).environment;
}
