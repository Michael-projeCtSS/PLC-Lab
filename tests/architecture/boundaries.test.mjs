// Architecture rules for the rebuilt product, checked against `cargo metadata` (the real
// dependency graph) and against the security-relevant host configuration.
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { root } from "../../scripts/toolchain.mjs";

const read = (p) => readFileSync(path.join(root, p), "utf8");

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
const packages = workspacePackages();
const deps = (name) =>
  packages
    .get(name)
    .dependencies.filter((d) => d.kind !== "dev")
    .map((d) => d.name)
    .sort();
function closure(name, seen = new Set()) {
  for (const d of packages
    .get(name)
    .dependencies.filter((x) => x.kind !== "dev" && x.path)) {
    if (!seen.has(d.name)) {
      seen.add(d.name);
      closure(d.name, seen);
    }
  }
  return seen;
}

// Semantic and simulation layers: no transport, host, UI or application adapter.
const semantic = [
  "plc-types",
  "plc-ir",
  "plc-runtime",
  "st-frontend",
  "simulation-core",
  "trace-core",
  "lab-model",
  "process-simulation",
  "hardware-model",
  "network-core",
  "grading-core",
  "lab-engine",
  "knowledge-core",
  "project-model",
];

test("semantic layers never depend on transport, host, UI or the desktop adapter", () => {
  for (const name of semantic) {
    assert.ok(packages.has(name), `missing package ${name}`);
    for (const d of closure(name))
      assert.ok(
        !["app-contract", "desktop-session", "plc-lab-desktop"].includes(d),
        `${name} depends on ${d}`,
      );
    for (const d of deps(name))
      assert.doesNotMatch(d, /^tauri|^ts-rs$|^tokio$/, `${name} uses ${d}`);
  }
});

test("vocabulary crates stay minimal and one runtime executes every language", () => {
  assert.deepEqual(deps("plc-types"), ["serde"]);
  assert.deepEqual(deps("plc-ir"), ["plc-types"]);
  assert.deepEqual(deps("st-frontend"), ["plc-ir", "plc-types"]);
  assert.ok(!closure("st-frontend").has("plc-runtime"));
  const runtimes = [...packages.keys()].filter((n) => /runtime/.test(n));
  assert.deepEqual(runtimes, ["plc-runtime"], "exactly one PLC runtime crate");
  assert.ok(closure("lab-engine").has("plc-runtime"));
});

test("the engine is headless and owns no knowledge, persistence or transport", () => {
  const engine = closure("lab-engine");
  for (const forbidden of ["knowledge-core", "project-model", "app-contract"])
    assert.ok(!engine.has(forbidden), `lab-engine depends on ${forbidden}`);
  assert.ok(
    !closure("grading-core").has("lab-engine"),
    "grading never re-simulates",
  );
  assert.ok(!closure("grading-core").has("process-simulation"));
});

test("the IPC contract is DTO-only and the adapter has no host dependency", () => {
  assert.deepEqual(deps("app-contract"), ["serde", "serde_json", "ts-rs"]);
  for (const d of deps("desktop-session"))
    assert.doesNotMatch(d, /^tauri|^tokio/);
  // winsafe: safe Win32 bindings, used only for the startup message box and the WebView2
  // runtime check before the window exists (ADR-0030).
  assert.deepEqual(
    deps("plc-lab-desktop").filter((d) => !/^tauri/.test(d)),
    ["app-contract", "desktop-session", "tokio", "winsafe"],
  );
});

test("the host uses only the reviewed single-instance plugin and the web view gets only event permissions", () => {
  const manifest = read("apps/desktop/src-tauri/Cargo.toml");
  // tauri-plugin-single-instance has no web-view commands: on Windows a named mutex and a
  // message-only window, on Linux the session D-Bus. No other plugin is allowed.
  assert.deepEqual(manifest.match(/tauri-plugin[\w-]*/g), [
    "tauri-plugin-single-instance",
  ]);
  const host = read("apps/desktop/src-tauri/src/lib.rs");
  assert.match(
    host,
    /tauri_plugin_single_instance::init\(\|app, args, cwd\| \{\s*open_requested\(app, &args, std::path::Path::new\(&cwd\)\);\s*focus_main_window\(app\);\s*\}\)/,
    "a second launch may only hand over a project file and focus the window",
  );
  assert.match(
    host,
    /fn open_requested[^{]*\{\s*let Some\(path\) = startup::open_request\(args, cwd\)/,
    "launch arguments pass only through the typed open-project parser",
  );
  const startup = read("apps/desktop/src-tauri/src/startup.rs");
  assert.match(startup, /pub const OPEN_PROJECT: &str = "--open-project";/);
  assert.doesNotMatch(
    startup,
    /Command|process::|spawn|shell|fs::/,
    "a launch request is parsed, never executed or read here",
  );
  const resources = read("apps/desktop/src-tauri/src/resources.rs");
  assert.doesNotMatch(
    resources,
    /Command|spawn|shell|open_request/,
    "release resources are only inspected, never executed",
  );
  assert.match(resources, /std::env::current_exe\(\)/);
  assert.ok(
    host.indexOf(".plugin(tauri_plugin_single_instance") <
      host.indexOf(".setup("),
    "the single-instance plugin is registered before the application starts",
  );
  const capability = JSON.parse(
    read("apps/desktop/src-tauri/capabilities/main.json"),
  );
  assert.deepEqual(capability.permissions.sort(), [
    "core:event:allow-listen",
    "core:event:allow-unlisten",
  ]);
  const config = JSON.parse(read("apps/desktop/src-tauri/tauri.conf.json"));
  const csp = config.app.security.csp;
  assert.match(csp, /script-src 'self'(;|$)/);
  assert.doesNotMatch(csp, /unsafe-eval|https?:\/\/(?!ipc\.localhost)/);
  assert.match(read("Cargo.toml"), /unsafe_code = "forbid"/);
});

function files(dir, ext) {
  const out = [];
  for (const entry of readdirSync(path.join(root, dir))) {
    const rel = path.join(dir, entry);
    if (statSync(path.join(root, rel)).isDirectory())
      out.push(...files(rel, ext));
    else if (ext.some((e) => entry.endsWith(e))) out.push(rel);
  }
  return out;
}

test("simulation time never comes from the wall clock", () => {
  const timeless = [
    "crates/plc-runtime/src",
    "crates/simulation-core/src",
    "crates/process-simulation/src",
    "crates/hardware-model/src",
    "crates/network-core/src",
    "crates/lab-engine/src",
    "crates/grading-core/src",
    "crates/trace-core/src",
    "crates/desktop-session/src/workspace.rs",
  ];
  for (const target of timeless) {
    const list = target.endsWith(".rs") ? [target] : files(target, [".rs"]);
    for (const f of list)
      assert.doesNotMatch(
        read(f),
        /SystemTime|Instant::|thread::sleep|tokio::time/,
        `${f} uses wall-clock time`,
      );
  }
  const host = read("apps/desktop/src-tauri/src/execution_driver.rs");
  assert.match(host, /wait\(period\)\.await;\s*match step\(\)\.await/);
  assert.doesNotMatch(host, /interval\(|elapsed\(|Instant|SystemTime/);
});

test("the user interface never executes PLC logic or drives cycles", () => {
  const ui = files("packages/ui/src", [".ts", ".tsx"]).filter(
    (f) => !f.endsWith(".test.ts") && !f.endsWith(".test.tsx"),
  );
  for (const f of ui) {
    const text = read(f);
    assert.doesNotMatch(
      text,
      /\beval\(|new Function\(|dangerouslySetInnerHTML/,
      f,
    );
    assert.doesNotMatch(
      text,
      /setInterval|requestAnimationFrame/,
      `${f} drives timing`,
    );
    assert.doesNotMatch(
      text,
      /@tauri-apps\/api/,
      `${f} bypasses the typed IPC client`,
    );
  }
  const client = read("packages/ipc/src/client.ts");
  assert.doesNotMatch(client, /"tick"|"step_cycle"/);
});

test("release builds do not embed repository knowledge paths", () => {
  const host = read("apps/desktop/src-tauri/src/lib.rs");
  const repoPath = host.indexOf("CARGO_MANIFEST_DIR");
  assert.ok(repoPath > 0);
  assert.match(
    host.slice(Math.max(0, repoPath - 120), repoPath),
    /#\[cfg\(debug_assertions\)\]/,
    "the repository pack root exists only in development builds",
  );
});

test("user-visible diagnostics use product terminology, not milestone names", () => {
  const offenders = [];
  for (const dir of ["languages/st/src", "crates"]) {
    for (const file of files(dir, [".rs"])) {
      if (file.split(path.sep).includes("tests")) continue;
      read(file)
        .split("\n")
        .forEach((line, i) => {
          const code = line.replace(/\/\/.*$/, "");
          if (/"[^"]*\b(Alpha|M[0-9][AB]?|FOUNDATION)\b[^"]*"/.test(code))
            offenders.push(`${file}:${i + 1}: ${line.trim()}`);
        });
    }
  }
  assert.deepEqual(offenders, []);
});
