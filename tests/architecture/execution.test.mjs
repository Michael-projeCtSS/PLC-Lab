import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { root } from "../../scripts/toolchain.mjs";
const read = (p) => readFileSync(path.join(root, p), "utf8");
test("domain execution contains one scan path and no host pacing or wall time", () => {
  const files = readdirSync(path.join(root, "crates/desktop-session/src"))
    .filter((f) => f.endsWith(".rs"))
    .map(
      (f) => read(`crates/desktop-session/src/${f}`).split("#[cfg(test)]")[0],
    );
  const domain = files.join("\n");
  assert.equal([...domain.matchAll(/runtime\s*\.\s*single_scan\(/g)].length, 1);
  assert.doesNotMatch(
    domain,
    /SystemTime|Instant|thread::sleep|tokio|tauri|std::time|Duration/,
  );
  assert.match(domain, /active_lease == Some\(lease\)/);
  assert.match(domain, /epoch\.checked_add\(1\)/);
});
test("host pacing is isolated and no browser tick IPC or timer can drive PLC execution", () => {
  const host = read("apps/desktop/src-tauri/src/execution_driver.rs");
  assert.match(host, /tokio::time::sleep/);
  assert.match(host, /wait\(period\)\.await;[\s\S]*match tick\(\)\.await/);
  assert.doesNotMatch(host, /interval\(|elapsed\(|Instant|SystemTime/);
  const handlers = read("apps/desktop/src-tauri/src/lib.rs");
  assert.doesNotMatch(handlers, /commands::continuous_tick|commands::tick/);
  for (const p of [
    "packages/ui/src/useRuntimeSession.ts",
    "packages/ui/src/useSessionTrace.ts",
    "packages/ui/src/coalesced-refresh.ts",
    "packages/ipc/src/execution.ts",
  ])
    assert.doesNotMatch(
      read(p),
      /setTimeout|setInterval|requestAnimationFrame|continuous_tick|Date\.now|performance\.now/,
    );
  assert.match(
    read("packages/ui/src/useRuntimeSession.ts"),
    /getRuntimeSnapshot/,
  );
  assert.match(read("packages/ui/src/useSessionTrace.ts"), /getTracePage/);
});
