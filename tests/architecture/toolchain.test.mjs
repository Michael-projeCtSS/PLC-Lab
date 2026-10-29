import assert from "node:assert/strict";
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { spawnSync } from "node:child_process";
import path from "node:path";
import test from "node:test";
import {
  root,
  readToolchain,
  selectToolchain,
} from "../../scripts/toolchain.mjs";

const channel = "1.99.0";
const host = "x86_64-pc-windows-msvc";
const pin = `${channel}-${host}`;
const hash = "b940084d7000000000000000000000000000000000";
const tools = [
  "rustc",
  "cargo",
  "rustfmt",
  "clippy-driver",
  "cargo-fmt",
  "cargo-clippy",
  "rustdoc",
];

function fixture(t) {
  const parent = path.resolve(root, "artifacts/toolchain-fixtures");
  mkdirSync(parent, { recursive: true });
  const directory = mkdtempSync(path.join(parent, "test with spaces-"));
  t.after(() => {
    assert.ok(directory.startsWith(`${parent}${path.sep}`));
    rmSync(directory, { recursive: true, force: true });
  });
  writeFileSync(
    path.join(directory, "rust-toolchain.toml"),
    readFileSync(path.join(root, "rust-toolchain.toml")),
  );
  return {
    directory,
    userHome: path.join(directory, "user"),
    platform: "win32",
    arch: "x64",
    environment: { Path: "unrelated-tools", RUSTUP_TOOLCHAIN: "nightly" },
  };
}

function install(rustupHome, names = tools) {
  const bin = path.join(rustupHome, "toolchains", pin, "bin");
  mkdirSync(bin, { recursive: true });
  for (const name of names)
    writeFileSync(
      path.join(bin, `${name}.exe`),
      "not an executable; never actually launched",
    );
  return bin;
}

// Only unit tests simulate version probes. The BAT regression below uses real processes.
function versions(file, args, options) {
  assert.equal(options.timeout, 5_000);
  assert.equal(options.env.RUSTUP_AUTO_INSTALL, "0");
  assert.equal(options.env.RUSTUP_TOOLCHAIN, pin);
  const command = path.basename(file, ".exe");
  const output = {
    rustc: `rustc ${channel} (${hash.slice(0, 9)} 2026-09-28)\nhost: ${host}\nrelease: ${channel}\ncommit-hash: ${hash}`,
    cargo: `cargo ${channel} (000000000 2026-09-01)`,
    rustfmt: `rustfmt 1.9.0-stable (${hash.slice(0, 9)} 2026-09-28)`,
    "clippy-driver": `clippy 0.1.99 (${hash.slice(0, 9)} 2026-09-28)`,
    rustdoc: `rustdoc ${channel} (${hash.slice(0, 9)} 2026-09-28)`,
  }[command];
  assert.ok(output, `must never run a proxy or installer: ${file}`);
  assert.deepEqual(args, [command === "rustc" ? "-vV" : "--version"]);
  return { status: 0, stdout: output, stderr: "" };
}

test("cargo proxy alone never activates an incomplete local installation or launches a probe", (t) => {
  const options = fixture(t);
  const proxies = path.join(options.directory, ".tools/cargo/bin");
  mkdirSync(proxies, { recursive: true });
  writeFileSync(path.join(proxies, "cargo.exe"), "rustup proxy");
  options.run = () =>
    assert.fail("incomplete toolchains must not execute commands");
  assert.throws(
    () => selectToolchain(options),
    (error) => {
      assert.match(error.message, /PLC LAB BUILD BLOCKED/);
      assert.match(error.message, /1\.99\.0-x86_64-pc-windows-msvc/);
      assert.match(error.message, /Missing: .*rustc\.exe/);
      assert.match(
        error.message,
        /User pinned toolchain: NOT FOUND|Configured system pinned toolchain: NOT FOUND/,
      );
      assert.match(
        error.message,
        /No Rust installation or repair was attempted/,
      );
      return true;
    },
  );
});

test("healthy local pin is preferred and uses direct compiler and Cargo paths", (t) => {
  const options = fixture(t);
  const local = path.join(options.directory, ".tools/rustup");
  const bin = install(local);
  options.run = versions;
  const result = selectToolchain(options);
  assert.equal(result.label, "Project-local toolchain");
  assert.equal(result.environment.RUSTUP_HOME, local);
  assert.equal(result.environment.RUSTC, path.join(bin, "rustc.exe"));
  assert.equal(result.environment.RUSTDOC, path.join(bin, "rustdoc.exe"));
  assert.equal(result.environment.Path.split(path.delimiter)[0], bin);
  assert.equal(options.environment.RUSTUP_TOOLCHAIN, "nightly");
});

test("broken inherited local homes fall back to the exact installed user pin", (t) => {
  const options = fixture(t);
  const local = path.join(options.directory, ".tools/rustup");
  install(local, ["cargo"]);
  install(path.join(options.userHome, ".rustup"));
  options.environment.RUSTUP_HOME = local;
  options.environment.CARGO_HOME = path.join(options.directory, ".tools/cargo");
  options.run = versions;
  const result = selectToolchain(options);
  assert.equal(result.label, "User pinned toolchain");
  assert.equal(
    result.environment.RUSTUP_HOME,
    path.join(options.userHome, ".rustup"),
  );
  assert.equal(
    result.environment.CARGO_HOME,
    path.join(options.userHome, ".cargo"),
  );
  assert.equal(result.reports.length, 2);
});

test("CI/custom homes work without repository-local Rust or developer paths", (t) => {
  const options = fixture(t);
  options.environment.RUSTUP_HOME = path.join(
    options.directory,
    "runner-rustup",
  );
  options.environment.CARGO_HOME = path.join(options.directory, "runner-cargo");
  install(options.environment.RUSTUP_HOME);
  options.run = versions;
  const result = selectToolchain(options);
  assert.equal(result.label, "Configured system pinned toolchain");
  assert.equal(result.environment.CARGO_HOME, options.environment.CARGO_HOME);
});

test("unusable complete local installation falls back to a working user installation", (t) => {
  const options = fixture(t);
  const local = path.join(options.directory, ".tools/rustup");
  install(local);
  install(path.join(options.userHome, ".rustup"));
  options.run = (...args) =>
    args[2].env.RUSTUP_HOME === local
      ? { status: null, error: new Error("Access denied") }
      : versions(...args);
  const result = selectToolchain(options);
  assert.equal(result.label, "Configured system pinned toolchain");
  assert.match(result.reports[0].problems[0], /Access denied/);
});

test("Linux CI selects the exact GNU host and binaries without Windows suffixes", (t) => {
  const options = fixture(t);
  options.platform = "linux";
  const linuxHost = "x86_64-unknown-linux-gnu";
  const linuxPin = `${channel}-${linuxHost}`;
  const bin = path.join(
    options.directory,
    ".tools/rustup/toolchains",
    linuxPin,
    "bin",
  );
  mkdirSync(bin, { recursive: true });
  for (const name of tools) writeFileSync(path.join(bin, name), "fixture");
  options.run = (file, args, settings) => {
    assert.equal(settings.env.RUSTUP_TOOLCHAIN, linuxPin);
    const result = versions(`${file}.exe`, args, {
      ...settings,
      env: { ...settings.env, RUSTUP_TOOLCHAIN: pin },
    });
    return { ...result, stdout: result.stdout.replace(host, linuxHost) };
  };
  const result = selectToolchain(options);
  assert.equal(result.pin, linuxPin);
  assert.equal(result.environment.RUSTC, path.join(bin, "rustc"));
});

test("wrong compiler release, host, component commit, failed execution and timeouts are rejected", (t) => {
  const options = fixture(t);
  install(path.join(options.directory, ".tools/rustup"));
  for (const fail of [
    (r) => ({
      ...r,
      stdout: r.stdout.replace("release: 1.99.0", "release: 1.98.0"),
    }),
    (r) => ({
      ...r,
      stdout: r.stdout.replace(`host: ${host}`, "host: wrong-host"),
    }),
    (r) => ({ ...r, stdout: r.stdout.replace("cargo 1.99.0", "cargo 1.98.0") }),
    (r) => ({
      ...r,
      stdout: r.stdout.replace(
        "rustfmt 1.9.0-stable (b940084d7",
        "rustfmt 1.9.0-stable (deadbeef0",
      ),
    }),
    (r) => ({
      ...r,
      stdout: r.stdout.replace(
        "clippy 0.1.99 (b940084d7",
        "clippy 0.1.99 (deadbeef0",
      ),
    }),
    () => ({ status: 5, stderr: "Access denied", stdout: "" }),
    () => ({
      status: null,
      error: Object.assign(new Error("probe timed out"), { code: "ETIMEDOUT" }),
    }),
  ]) {
    options.run = (...args) => fail(versions(...args));
    assert.throws(() => selectToolchain(options), /Unusable:/);
  }
});

test("missing required formatter and clippy entry points are diagnosed without executing proxies", (t) => {
  const options = fixture(t);
  install(path.join(options.directory, ".tools/rustup"), [
    "cargo",
    "rustc",
    "rustdoc",
  ]);
  options.run = () => assert.fail("no incomplete component probes");
  assert.throws(() => selectToolchain(options), /Missing: .*rustfmt\.exe/);
});

test("floating Rust channels are refused rather than installing latest", () => {
  const text = readFileSync(path.join(root, "rust-toolchain.toml"), "utf8");
  assert.throws(
    () => readToolchain(text.replace(channel, "stable")),
    /exact version/,
  );
});

test(
  "real release BAT fails promptly on cargo-proxy-only install without starting rustup",
  { skip: process.platform !== "win32" },
  (t) => {
    const options = fixture(t);
    mkdirSync(path.join(options.directory, "scripts"));
    for (const file of [
      "BUILD_PLC_LAB_RELEASE.bat",
      "scripts/human-launcher.bat",
      "scripts/human-test.mjs",
      "scripts/toolchain.mjs",
    ])
      copyFileSync(path.join(root, file), path.join(options.directory, file));
    const proxies = path.join(options.directory, ".tools/cargo/bin");
    mkdirSync(proxies, { recursive: true });
    writeFileSync(
      path.join(proxies, "cargo.exe"),
      "broken proxy must never run",
    );
    const pathKey =
      Object.keys(process.env).find((key) => key.toUpperCase() === "PATH") ??
      "PATH";
    const result = spawnSync(
      process.env.ComSpec ?? "C:/Windows/System32/cmd.exe",
      [
        "/d",
        "/c",
        `call "${path.join(options.directory, "BUILD_PLC_LAB_RELEASE.bat")}"`,
      ],
      {
        cwd: path.join(process.env.SystemRoot ?? "C:/Windows", "System32"),
        env: {
          ...process.env,
          [pathKey]: [
            path.dirname(process.execPath),
            path.join(process.env.SystemRoot ?? "C:/Windows", "System32"),
          ].join(path.delimiter),
          USERPROFILE: options.userHome,
          HOME: options.userHome,
          CARGO_HOME: path.join(options.userHome, ".cargo"),
          RUSTUP_HOME: path.join(options.userHome, ".rustup"),
          PLC_LAB_NO_PAUSE: "1",
        },
        encoding: "utf8",
        windowsVerbatimArguments: true,
        timeout: 15_000,
      },
    );
    assert.ifError(result.error);
    assert.equal(result.status, 1);
    const output = result.stdout + result.stderr;
    assert.match(output, /PLC LAB BUILD BLOCKED/);
    assert.match(output, /Project-local toolchain: BROKEN/);
    assert.match(output, /Missing: .*rustc\.exe/);
    assert.match(output, /RELEASE BUILD FAILED/);
    assert.doesNotMatch(
      output,
      /retrying rename|RELEASE BUILD: SUCCESS|Installing with npm ci/,
    );
  },
);
