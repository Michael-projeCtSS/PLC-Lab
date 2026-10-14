import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { root } from "../../scripts/toolchain.mjs";

const coreNames = new Set([
  "plc-types",
  "plc-ir",
  "plc-runtime",
  "simulation-core",
  "trace-core",
  "network-core",
  "grading-core",
]);

function workspacePackages() {
  const result = spawnSync(
    process.execPath,
    [
      "scripts/rust.mjs",
      "metadata",
      "--no-deps",
      "--format-version",
      "1",
      "--locked",
    ],
    { cwd: root, encoding: "utf8" },
  );
  assert.equal(result.status, 0, result.stderr);
  const metadata = JSON.parse(result.stdout);
  return new Map(metadata.packages.map((item) => [item.name, item]));
}

test("core dependency closure excludes desktop, UI, and language frontends", () => {
  const packages = workspacePackages();
  const inspect = (name, visited = new Set()) => {
    if (visited.has(name)) return;
    visited.add(name);
    const item = packages.get(name);
    assert.ok(item, `Missing core package ${name}`);
    for (const dependency of item.dependencies.filter(
      (dep) => dep.kind !== "dev",
    )) {
      if (dependency.path) {
        assert.ok(
          coreNames.has(dependency.name),
          `${name} depends on non-core ${dependency.name}`,
        );
        inspect(dependency.name, visited);
      } else {
        assert.ok(
          !/^tauri|react|ts-rs/.test(dependency.name),
          `${name} contains a UI/transport dependency`,
        );
      }
    }
  };
  for (const name of coreNames) inspect(name);
});

test("semantic crates depend only on shared vocabulary and serialization", () => {
  const packages = workspacePackages();
  for (const [name, expected] of [
    ["plc-types", ["serde"]],
    ["plc-ir", ["plc-types"]],
  ]) {
    const dependencies = packages
      .get(name)
      .dependencies.filter((dependency) => dependency.kind !== "dev")
      .map((dependency) => dependency.name)
      .sort();
    assert.deepEqual(
      dependencies,
      expected,
      `${name} must not depend on runtime, simulation, trace, protocols, editors, or UI`,
    );
  }
});

test("runtime uses a leaf simulation clock without reverse or trace execution dependencies", () => {
  const packages = workspacePackages();
  for (const [name, expected] of [
    ["simulation-core", ["plc-types"]],
    ["plc-runtime", ["plc-ir", "plc-types", "simulation-core"]],
  ]) {
    const dependencies = packages
      .get(name)
      .dependencies.filter((dependency) => dependency.kind !== "dev")
      .map((dependency) => dependency.name)
      .sort();
    assert.deepEqual(
      dependencies,
      expected,
      `${name} violates ADR-0010 dependency direction`,
    );
  }
});

test("desktop IPC exposes only harmless foundation information", () => {
  const host = readFileSync(
    path.join(root, "apps/desktop/src-tauri/src/lib.rs"),
    "utf8",
  );
  const handlers = host.match(/generate_handler!\[([^\]]+)\]/)?.[1]?.trim();
  assert.equal(handlers, "commands::get_core_info");
  const capability = JSON.parse(
    readFileSync(
      path.join(root, "apps/desktop/src-tauri/capabilities/main.json"),
      "utf8",
    ),
  );
  assert.deepEqual(capability.permissions, []);
  assert.equal(capability.remote, undefined);
});

test("Sprint 0 renderer contains no student-code evaluator or scan scheduler", () => {
  const visit = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const file = path.join(directory, entry.name);
      if (entry.isDirectory()) visit(file);
      else if (
        /\.(tsx?|js)$/.test(entry.name) &&
        !entry.name.includes(".test.")
      ) {
        const source = readFileSync(file, "utf8");
        assert.doesNotMatch(
          source,
          /\beval\s*\(|new\s+Function\s*\(|\bsetInterval\s*\(/,
          file,
        );
      }
    }
  };
  visit(path.join(root, "packages/ui/src"));
  visit(path.join(root, "apps/desktop/src"));
});
