import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { root } from "../../scripts/toolchain.mjs";
test("physical process is a headless types leaf and application owns every runtime binding", () => {
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
  const packages = JSON.parse(result.stdout).packages;
  assert.deepEqual(
    packages
      .find((p) => p.name === "process-simulation")
      .dependencies.filter((d) => d.kind !== "dev")
      .map((d) => d.name),
    ["plc-types"],
  );
  const model = readFileSync(
    path.join(root, "crates/process-simulation/src/lib.rs"),
    "utf8",
  );
  assert.doesNotMatch(
    model,
    /SystemTime|Instant|thread::sleep|std::(?:fs|net|process)|SymbolId|plc_runtime|tauri|unsafe\s*\{/,
  );
  assert.match(model, /level_permille:\s*u16/);
  assert.match(model, /checked_mul/);
  assert.match(model, /checked_add/);
  assert.match(model, /checked_sub/);
  const app = readFileSync(
    path.join(root, "crates/desktop-session/src/process.rs"),
    "utf8",
  );
  assert.match(app, /eq_ignore_ascii_case/);
  assert.match(app, /runtime\.set_input/);
  assert.doesNotMatch(app, /SystemTime|Instant::|thread::sleep|unsafe\s*\{/);
});
test("tank renderer only formats authoritative geometry without sensor or time integration", () => {
  const panel = readFileSync(
    path.join(root, "packages/ui/src/ProcessPanel.tsx"),
    "utf8",
  );
  assert.doesNotMatch(
    panel,
    /setInterval|setTimeout|requestAnimationFrame|Date\.now|performance\.now|\b(?:fillRate|drainRate)\b|\.levelPermille\s*(?:\+\+|[+-]=|=)|\.lowLevel\s*=|\.highLevel\s*=/,
  );
  assert.match(panel, /snapshot\.lowLevel/);
  assert.match(panel, /snapshot\.highLevel/);
  assert.match(panel, /snapshot\.levelPermille\)?\s*\/\s*1000/);
  const contract = readFileSync(
    path.join(root, "crates/app-contract/src/process.rs"),
    "utf8",
  );
  assert.doesNotMatch(
    contract,
    /Mutex|Runtime::new|TankFillingProcess|\.step\(/,
  );
});
