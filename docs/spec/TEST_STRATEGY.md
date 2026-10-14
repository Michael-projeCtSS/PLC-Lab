# Test strategy

## M3A manual physical feedback acceptance

process-simulation tests cover integer defaults, exact thresholds, +100/-80/+20/0 effects at 100 ms, independent fractional truncation, clamping, reset, repeated complete snapshot equality, finite analog feedback and maximum u64 duration without wrapping. Architecture checks forbid runtime/host/rendering dependencies and wall-time/OS capabilities in this model.

desktop-session tests start from actual ST, including the shared scenes/tank-filling/example.st. Prove the nine-scan causal sequence, all five required name/type/role bindings, mixed-case names with extra variables and shifted IDs, owned write rejection, detach/manual restoration, load/unload lifecycle, Runtime-vs-Process reset, retained TON/user state, and a fault after provisional actuator writes that rolls back PLC state and preserves the whole plant. Existing deterministic Load intent tests remain; an additional private-operation test preserves an attached scene when newest B fails and older A is rejected. Impossible binding corruption is test-only and proves fail-closed application handling without changing core APIs.

Contract tests cover process tags/fields/scalar/range bounds and exact targeting. Frontend tests use fixed DTOs, verify SVG geometry, use supplied booleans without deriving sensors, preserve authority/old-source protections and require explicit example confirmation. No frontend test provides semantic execution proof.

The real Windows smoke inserts the example through UI confirmation, separately Compiles/Loads/Attaches, observes 0->10->...->80% and next-scan valve closure, directly proves Rust rejects an owned sensor write, demonstrates both resets, detach/pure scan, stale-source blocking and new-Load scene removal. Every UI mutation is correlated with one real targeted Rust observation. No mocked transport/model is used. Both CI lanes add process-simulation to fmt/clippy/debug/release selections and retain every existing REAL/TON/compiler/rollback/Load-intent test. Scope remains manual: no trace/exercise/grading/network/background engine.

## M2B manual session acceptance

Headless desktop-session tests use actual ST compilation, validation, lowering and Runtime. Cover prepare-then-swap, generations/stale commands, role/type/identity checks, default/staged/committed observations, concurrent serialization, same-scan read-after-write, exact TON, source-backed faults/rollback, reset and unload. Contract tests prove checked finite REAL and exact u64 TIME/IDs plus strict fields/discriminants. Frontend tests use fixed backend DTOs to check presentation, explicit numeric commits, separate markers, source association and old-response filtering. No frontend test evaluates ST.

M2B-R1 deterministically reserves A then B and invokes the private preparation/commit operations in inverted completion order: B commits, A is rejected, B remains observable. Another test loads/scans X, reserves A, then calls public Load B with actual invalid ST: B fails, A is rejected, the complete X observation survives and valid C succeeds. These tests use no sleeps, delays, source-size assumptions or timing races. Boundary tests cover intent exhaustion and version rejection before reservation. Frontend tests cover discarding an obsolete APP-RUNTIME-STALE-LOAD response after B loads, and failing closed without automatic retry when that error reaches the current request. Existing tests remain required in both debug/release CI profiles.

Native Windows smoke edits actual Monaco source and performs real compile/load/input/scan/reset/unload IPC. Each mutation is correlated with one coherent get_runtime_snapshot response for the exact loaded ID. It proves same-scan results, four-scan TON timing, the actual fault expression underline and rolled-back state, reset and typed INT/REAL/maximum TIME. Existing M0–M2A checks remain. Both CI lanes execute application/session and typed contract tests in debug/release; no required test can be replaced by compilation.

## M2A ST source acceptance

languages/st/tests/alpha.rs starts every execution proof from actual ST text, then uses lexer -> AST -> compiler -> existing IR validation -> existing runtime lowering/execution. It covers the full assigned lexer/parser/semantic matrix, signed INT minimum, finite REAL, exact checked TIME, limits (including flat/deep trees), deterministic identities, Unicode/CRLF positions, source-backed runtime division fault and identical normalized TON behavior for reordered bindings. Deterministic malformed-input cases exercise bounded error handling without a second evaluator.

Rust app-contract tests cover pure compiler DTOs/version rejection/source limits and no executable response data. Frontend tests cover typed response validation, explicit scalar-to-UTF-16 spans, compile states, declaration-only display, clearing obsolete diagnostics and asynchronous revision races/unmount. Monaco is isolated as presentation in UI unit tests; actual Windows native smoke uses real Monaco, real Rust compile IPC, valid/default/TON source and a Unicode-positioned error marker. All earlier IPC, disabled-controls, page-error and layout checks remain.

Both independent lanes include st-frontend in focused debug/release tests and Linux clippy. Existing M1 core packages, REAL assertions, rollback/TON/clock checks and Windows native acceptance remain required. Compilation/build-only outcomes cannot replace actual execution.

The golden rule is:

> Same program + same initial state + same event sequence = same trace.

The equality assumes the same simulation seed, timing configuration, semantic profile, and trace schema. Canonical trace comparison must include scan numbers, virtual timestamps, ordered changes/events, faults, and source mapping. Exclude wall-clock rendering and transport delivery metadata. A random fault source must be seeded with a specified algorithm; identical events at identical times need a documented tie-breaker.

## Layers

| Layer                       | Responsibility                                                                              | Current / future                                                                            |
| --------------------------- | ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Unit                        | Pure types, DTOs, adapter validation, UI states; normal, boundary, and negative cases       | Implemented for substantive foundation modules                                              |
| Semantic                    | PLC type/operator/control-flow/block rules, scan output commit, faults                      | M1B rules plus M1C TON tested in release; full validation pending; language semantics later |
| M1A semantic representation | Value/type rules, symbols/scopes, source anchors, typed IR structure and rejection          | Implemented headless tests; no expression evaluation or scan behavior                       |
| Integration                 | Language frontend → IR → runtime; runtime → process/network → trace                         | Direct IR -> owned execution tested; language/process/network/trace later                   |
| Determinism                 | Repeat identical inputs/seeds/timing; compare canonical trace, including fault cases        | M1B snapshots/faults/clock tested; canonical trace comparison later                         |
| Cross-language equivalence  | Equivalent ST/LAD/FBD/STL programs with identical stimuli yield equivalent behavioral trace | Add when each frontend is approved; source-location differences normalized explicitly       |
| End-to-end exercises        | Real source compile/run → process → trace → visible result → behavioral grading             | DEMO-001 must pass with no mocked runtime/process                                           |
| Native shell acceptance     | Real Tauri executable → React → IPC → Rust → response → visible diagnostics                 | Implemented Windows WebView2 smoke harness                                                  |
| Architecture                | Core cannot depend on host/renderer/frontends; no executable IPC in Sprint 0                | Implemented Cargo closure and host/renderer checks                                          |

## Foundation acceptance

Run `npm run check` and `npm run test:desktop`. The native test launches the compiled desktop with embedded frontend assets, checks the seven panels and disabled controls, invokes Rust for package information, rejects unsupported IPC versions, confirms an execution command is absent, and captures a screenshot. Unit tests can mock transport to exercise errors in isolation; native acceptance does not mock it.

Formatting includes source and documentation, excluding generated outputs and the verbatim engineering contract. Strict TypeScript, ESLint, cargo fmt, and clippy with warnings denied are gates. Rust and npm dependencies are lockfile controlled. Empty reservation crates have no contrived functionality tests; their boundaries are checked centrally. The desktop main entry point has its empty unit-test harness disabled; its startup and command adapter are covered by the real native smoke test. Rust contract, diagnostics, and conformance tests remain enabled. Tests for any new substantive module are required.

## M1B headless execution acceptance

Direct-IR tests run the real validation -> owned lowering -> Runtime -> single_scan -> committed memory path. They reuse the small test-only assembler and execute the four approved golden examples. No parser/runtime mocks or wall-clock sleeps are involved.

Coverage includes stable staged-input/image separation, internal/output read-after-write and persistence, success publication, all-writable rollback with previously non-default outputs, unchanged failed time/count, read-only entry inputs, unsupported InOut, foreign POU exclusion, exact defaults, checked INT faults, binary32 bit patterns/canonical zero/subnormals/non-finite faults, TIME comparisons, left-to-right faults, eager booleans, selected IF only, exact budget boundaries, clock/count overflow, Faulted refusal, and deterministic full reset. Repeated identical programs/defaults/input sequences/periods compare owned results and fault diagnostics exactly.

The golden rule remains the future trace requirement. M1B compares snapshots/faults because trace recording is explicitly excluded. Fixed REAL bit tests must run on additional supported target CI before cross-platform determinism is claimed. The native acceptance still checks the unchanged shell and read-only IPC, never UI execution.

Focused checks: `node scripts/rust.mjs test --locked -p plc-types -p plc-ir -p plc-runtime -p simulation-core`. Full regression: `npm run validate`. Architecture guards enforce simulation-core as a clock/types leaf and exclude trace/UI/frontend authority from execution.

## M1C standard function-block acceptance

Use the real typed catalog -> canonical call validation -> owned normalized plan -> synchronous runtime path. TON tests use only SimulationClock scan starts, never wall-clock sleeps. Preserve every accepted M1B regression test, including optimized REAL bit-pattern checks.

The required matrix covers: false input; zero PT; exact 100 ms cycle/300 ms preset timeline; capped ET; false after expiry and new activation; decreasing/increasing PT; Q/ET writes and same-scan visibility; omitted associations; sequential repeated calls; skipped calls and later elapsed jumps; missing IN/PT; wrong IN/PT/Q/ET types; read-only inputs; unknown instances/definitions/parameters; authoritative relationship uniqueness; wrong direction/duplicates/unsupported kind; arithmetic, budget and clock rollback; reset; independent instances; identical sequences and snapshots.

Public tests observe immutable instance Q/ET without accessing active_since. Private runtime tests additionally compare complete committed instance maps to prove epoch rollback after later arithmetic, later budget, budget between mutation and either output, and clock/count completion failure. Impossible clock reversal must be APP-CORE-001 with call source. Reversed binding tests prove IN-before-PT evaluation and Q-before-ET writes through budget/fault source evidence. No private state setter or fake SymbolId exists in production.

Focused debug command: `node scripts/rust.mjs test --locked -p plc-types -p plc-ir -p plc-runtime -p simulation-core`. Optimized command: `node scripts/rust.mjs test --release --locked -p plc-runtime -p simulation-core`. Canonical full gate remains `npm run validate`, including frontend, architecture, all Rust, production frontend build and native read-only IPC. Actual executed outcomes belong in the completion report; blocked/unexecuted cases cannot be declared passing.

## Later language/exercise acceptance

Before implementing language features, specify normal/error/boundary cases: typed arithmetic and conversions, IF branches, time overflow, timer edges, execution budgets, invalid memory, call depth, and cancellation. Define fault effects on partial state, trace, and virtual time. Timer tests use virtual ticks, never sleeps or machine elapsed time. Cross-language tests reuse the same stimuli and process models. Headless grading and visible rendering must consume the same evidence.

Every meaningful exercise needs an observable process outcome and behavioral/timing/safety checks; final output values alone cannot prove correctness. Store representative versioned golden traces only after schema/semantics approval. Rebaseline changes require review explaining semantic or schema changes; do not bless a failing trace blindly.

## Limits of current evidence

Sprint 0 proves shell/IPC boundaries; M1A proves semantic representation; accepted M1B adds headless execution, transactions, budgets and checked clock on the current Windows target. M1C's scope adds standard TON and transactional private state; see its completion report for actual test outcomes. M1A golden tests still only validate; separate runtime goldens execute direct IR. No tests prove a language frontend, other standard blocks, general loop/call sandbox, process/network model, trace recorder, protocol conformance, or exercise grading. Native smoke remains Windows-only. Source-pattern guards are regression checks, not a complete security proof.
