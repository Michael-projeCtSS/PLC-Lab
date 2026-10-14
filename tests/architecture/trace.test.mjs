import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import path from "node:path";
import { root } from "../../scripts/toolchain.mjs";
const read = (file) => readFileSync(path.join(root, file), "utf8");
test("trace remains a generic observational leaf without runtime/process dependencies", () => {
  const cargo = read("crates/trace-core/Cargo.toml");
  assert.match(cargo, /plc-types\.workspace/);
  assert.doesNotMatch(
    cargo,
    /plc-runtime|process-simulation|desktop-session|app-contract|serde|tauri|st-frontend/,
  );
  for (const name of ["plc-runtime", "process-simulation"])
    assert.doesNotMatch(read(`crates/${name}/Cargo.toml`), /trace-core/);
  const core = read("crates/trace-core/src/lib.rs");
  assert.match(core, /TraceBuffer<T>/);
  assert.match(core, /VecDeque/);
  assert.doesNotMatch(
    core,
    /SystemTime|Instant|std::(?:fs|net|thread)|serde_json|Runtime::|\.step\(/,
  );
});
test("session history stores typed observations rather than presentation blobs", () => {
  const app = read("crates/desktop-session/src/trace.rs").split(
    "#[cfg(test)]",
  )[0];
  assert.doesNotMatch(
    app,
    /TraceBuffer<TraceEntryDto>|TraceBuffer<RuntimeSnapshotDto>|serde_json|active_since|SystemTime|Instant|thread::|\.single_scan\(|\.set_input\(/,
  );
  assert.match(app, /observation: Observation/);
  assert.match(app, /self\.trace\s*\.append/);
  assert.match(
    read("apps/desktop/src-tauri/src/commands.rs"),
    /s\.trace_page\(request\)/,
  );
});
test("Timeline uses typed request-driven history without simulation or replay", () => {
  const hook = read("packages/ui/src/useSessionTrace.ts"),
    view = read("packages/ui/src/TimelinePanel.tsx");
  assert.match(hook, /getTracePage/);
  assert.match(hook, /page\.sessionId !== sessionId/);
  assert.match(view, /Historical Trace/);
  for (const text of [hook, view, read("packages/ui/src/trace-history.ts")])
    assert.doesNotMatch(
      text,
      /Date\.now|performance\.now|setInterval|requestAnimationFrame|setTimeout|singleScan|resetRuntime|setRuntimeInput|attachProcessScene|levelPermille\s*[+-]=|simulationTimeMicros\s*[+-]=/,
    );
});
