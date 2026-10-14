# M2A implementation report

Date: 2026-10-14. Branch: codex/m2a-st-compiler-editor. Implementation is complete locally; independent CI verification is in progress. This report will be finalized with exact hosted evidence before claiming M2A PASS.

## Completed

Implemented bounded Rust ST lexing/parsing, a dedicated source-spanned AST, semantic compilation with accepted types/symbols/FBs, mandatory IR validation, immutable owned compilation results and direct-source runtime proofs. Added pure typed compile IPC v3, locally bundled Monaco ST editing/highlighting, backend markers/navigation, declaration-only Variables, explicit compile states and monotonic stale-result protection. Extended actual native smoke and both CI lanes without removing M1 checks. No runtime UI was added.

## Files

- languages/st/src: lexer, AST, parser, literals, diagnostics/configuration, compiler and public API.
- languages/st/tests/alpha.rs: 77 substantive source/limit/semantic/runtime/provenance tests.
- crates/app-contract/src/compilation.rs and lib.rs: pure compiler DTO/service, generator and v3 migration; Cargo path dependencies/lockfile updated.
- apps/desktop/src-tauri/src: thin async compilation command on the blocking pool.
- packages/ipc/src: generated declarations, checked compile client and boundary tests.
- packages/ui/src: Monaco facade/component, span converter, compile hook, shell/declaration/diagnostic integration and tests.
- scripts/ci.mjs, architecture guards and tests/e2e/desktop-smoke.mjs: ST coverage plus all existing core/native requirements.
- ADR-0012, Alpha ST specification, diagnostic/feature/test/boundary/architecture/development/roadmap/human documentation and repository language overview.

## Architecture / compiler architecture

ST Source -> Lexer -> Tokens -> dedicated ST AST -> Semantic Compiler -> Canonical PLC IR -> existing Validator -> ValidatedProgram. Headless tests continue through existing owned ExecutionPlan -> existing PLC Runtime -> single_scan -> committed results. CompiledProgram owns an immutable candidate validated at construction; validated() explicitly obtains the existing borrowed wrapper before lowering. Errors cannot return an executable wrapper.

st-frontend production depends only on plc-types/plc-ir. Runtime/clock are dev-dependencies. app-contract adapts compiler data; Tauri remains thin. No core crate imports ST, React or Tauri. ADR-0012 documents this assigned increment; no incompatible M1 decision was discovered or changed.

## ST grammar

One named PROGRAM; VAR_INPUT/VAR_OUTPUT/VAR sections; BOOL/INT/REAL/TIME and local TON declarations; assignment; nested IF with optional ELSE; TON named input/output associations; constants/names/parentheses; NOT, unary minus, arithmetic, comparisons, AND/OR. Semicolons are required after declarations, assignments, calls and END_IF. Keywords/identifiers follow the accepted ASCII case-insensitive policy and original spelling is retained. Non-nested block comments and // comments support Unicode. The Alpha specification explicitly separates implemented/planned/unsupported features and claims no full IEC/vendor compatibility.

## Deterministic identities

POU/document identities start at 1 per compile. Scalar symbol and FB-instance counters increment independently in source declaration order. One explicit semantic TON definition is allocated on the first TON declaration; its numeric identity never selects behavior. Source elements use deterministic syntax preorder. Identical source/configuration produces equal owned candidates/metadata. No random, wall-clock, pointer or unordered-map identity is used; stability across arbitrary edits is not claimed.

## Literals and limits

INT lexing keeps magnitude text; signed-literal construction accepts -32768 and rejects 32768/-32769 without wrapping. Decimal REAL requires digits around a decimal point and optional E exponent; checked PlcReal rejects non-finite results and canonicalizes negative zero. TIME uses exact checked u64 microseconds with strictly descending non-repeating d/h/m/s/ms/us components; fractions, negative values, nanoseconds and overflow are rejected.

Default limits: 65,536 source bytes, 16,384 tokens including EOF, nesting/tree depth 32 and 32 diagnostics. Checked nonzero configurations have hard ceilings 1,048,576/65,536/48/128. Source is bounded before character/token allocation, flat and nested trees are checked before deep construction, and recovery/capped diagnostics stop faulty parsing. No source-dependent filesystem, processes, sockets, imports, native/JavaScript generation or eval exists.

## TON compilation

```text
VAR Delay : TON; END_VAR
Delay(IN := Start, PT := T#300ms, Q => Motor);
```

This declares an accepted FunctionBlockDefinition::ton and a separate FunctionBlockInstance owned by the entry POU. Canonical FunctionBlockCall contains only the declared instance and typed IN/PT/Q associations. Existing validation checks the authoritative catalog relationship; existing lowering normalizes IN/PT then Q/ET regardless of source binding order. No timer implementation exists in ST or React.

## Runtime proof

All proofs begin from actual source text; no test assembles IR or uses a separate ST evaluator.

| Source proof                                      | Actual local result                                                                    |
| ------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Motor := Start                                    | PASS: FALSE/TRUE/FALSE input sequence produces matching outputs                        |
| Motor := Start AND NOT Stop                       | PASS: all four Start/Stop combinations                                                 |
| IF LevelHigh THEN Pump := FALSE ELSE Pump := TRUE | PASS: both branches through existing runtime                                           |
| TON with 100 ms cycles / 300 ms PT                | PASS: scan starts 0/100/200/300 ms produce FALSE/FALSE/FALSE/TRUE; ET 0/100/200/300 ms |
| Reordered TON associations                        | PASS: identical full ScanResults through existing normalized lowering                  |
| Division fault provenance                         | PASS: PLC-RUNTIME-003 resolves to the exact ST expression `1 / 0`                      |

## Source mapping / diagnostics

Meaningful tokens/AST nodes retain the common one-based Unicode scalar TextSpan. Compilation registers SourceRef/SourceMap anchors for generated IR and diagnostics; runtime lowering preserves executed-node provenance. The frontend explicitly maps backend scalar columns to Monaco UTF-16 code units. Unit tests cover ASCII, Polish characters, en/em dash, non-BMP characters, line endings, multiline/EOF spans and invalid bounds. Native smoke compares the actual underline geometry against the rendered Missing word after a Unicode comment, so an encoding shift is observable.

Stable new codes: PLC-ST-001 lexical/comment; 002 syntax; 003 literal; 004 unsupported; 005 duplicate/collision; 006 unknown identifier; 007 type/condition; 008 invalid assignment target; 009 invalid TON declaration/call/association; 010 resource/configuration limit. Core PLC-COMP/runtime codes retain their meanings. Compiler errors are returned as data with code/severity/message/optional span. APP-IPC-005 identifies a failed compilation worker separately.

## Desktop UI / native IPC

Monaco uses the locally pinned 0.57.0 editor API/worker with a PLC Lab ST presentation tokenizer. Compile uses actual Rust IPC, never a browser compiler. States: NOT COMPILED, COMPILING, COMPILE SUCCESS, COMPILE FAILED. Editing clears obsolete results; source/request BigInt counters prevent older responses from overriding newer source, including unmount cases. Markers and diagnostic navigation use backend spans. Variables shows source names/types/roles and —/Not running, never authoritative values.

IPC changes from v2 to v3, preserving rejection of unsupported versions. compile_st accepts source/contractVersion and returns a generated, runtime-checked success/failure union with diagnostics and declaration metadata. No IR, ExecutionPlan, memory image or persistent runtime reaches React or is created by Compile. The original core-information command and all four disabled execution controls remain covered.

## Tests

| Executed command / check                             | Local result                                                                                                                              |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Focused debug types/IR/runtime/clock/ST/app-contract | PASS: 203 tests, including 77 ST and 11 contract/IPC tests                                                                                |
| Optimized runtime/clock/ST                           | PASS: 52 runtime + 3 clock + 77 ST = 132                                                                                                  |
| npm run test:frontend                                | PASS: 47 tests                                                                                                                            |
| npm run test:architecture                            | PASS: 18 tests                                                                                                                            |
| npm run lint                                         | PASS: ESLint and workspace/all-target clippy with warnings denied                                                                         |
| npm run validate                                     | PASS after supported native harness corrections; all 205 workspace Rust tests, format/type/lint/frontend/architecture/build/native checks |
| Extended actual Tauri/WebView2 native smoke          | PASS: editor/default/TON compilation, declaration display, Unicode marker, cleanup, actual IPC v3, disabled controls, no page errors      |
| Windows independent CI                               | NOT_RUN at preparation; required before final PASS                                                                                        |
| Linux independent CI                                 | NOT_RUN at preparation; required before final PASS                                                                                        |

Initial new UI matcher incompatibility and native assumptions about Monaco's old textarea/text-containing underline were corrected without changing PLC assertions. The native test now uses the supported textbox role and checks underline bounds against the real word. One transient Windows file-mapping error during formatting resolved on ordinary retry, without security changes. The final local canonical command passed; superseded failed attempts are not counted as PASS.

## Regression audit

Accepted M1 plc-types/plc-ir/plc-runtime/simulation-core source and tests remain byte-identical. Their original REAL, TON, clock and transactional behavior tests remain mandatory. M1 validation documents and exact accepted code/gate SHAs are unchanged. Existing contract/native assertions retain meaning with the explicitly assigned v3 migration. Added ST production dependency direction remains acyclic; no other parsing library or runtime was introduced.

## Acceptance criteria

Implementation criteria 1–33 and 35 are covered by the actual local source/runtime/IPC/editor tests and regression audit. Criterion 34 (both independent platform lanes) is incomplete until hosted terminal evidence is reviewed. The report will record all 35 individually with final evidence after CI.

## Scope audit

No Run/Pause/Stop/Single Scan UI execution, background PLC task, value polling/watch/forcing, process or network simulation, trace recorder/timeline implementation, grading, LAD/FBD/STL, other timers/counters/edge blocks, user functions/FBs, persistence, cloud or AI generation entered M2A. Existing unavailable process/network/timeline panels remain presentation only.

## Assumptions / known limitations

Handwritten parsing avoids dependencies for the approved subset. Literal signs are source-literal interpretation; general unary expressions remain canonical operations. Recovery targets obvious syntax errors rather than every malformed grammar. The subset and resource ceilings are explicitly documented. Desktop execution/persistence is intentionally unavailable. Monaco is lazy-loaded but its editor chunk is about 2.7 MB before gzip, producing a non-failing Vite size warning; no size gate was weakened.

## Next dependency

Complete/review both required independent CI lanes, then technical-lead lexer/parser/compiler/IPC/editor source review. No M2B authorization is inferred.

## Gate

M2A GATE: FAIL

Independent CI acceptance is not yet complete. M2B has not started.
