# M2A implementation report

Date: 2026-10-14. Branch: codex/m2a-st-compiler-editor. Local and independent Windows/Linux validation passed. Technical-lead source review remains the next dependency before any M2B authorization.

PRIVATE repository: [Michael-projeCtSS/PLC-Lab](https://github.com/Michael-projeCtSS/PLC-Lab). VALIDATED CODE SHA: `90517c9e9504dfc4f437135b3f5f32f181e43880`. This final report is recorded by a later documentation-only commit; its changes do not alter the validated implementation. Main and accepted M1 history remain unchanged.

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

## Architecture

The assigned ST frontend and compilation UI reuse the frozen M1 foundation. ADR-0012 records implementation choices within the authorized scope; no M1 semantics or fundamental architecture changed.

## Compiler architecture

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

All proofs begin from actual source text; no test assembles IR or uses a separate ST evaluator. Each proof below passed locally and on independent Windows x64/Linux x64, in debug and release profiles.

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

| Executed command / check                             | Local result                                                                                                                                                  |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Focused debug types/IR/runtime/clock/ST/app-contract | PASS: 203 tests, including 77 ST and 11 contract/IPC tests                                                                                                    |
| Optimized runtime/clock/ST                           | PASS: 52 runtime + 3 clock + 77 ST = 132                                                                                                                      |
| npm run test:frontend                                | PASS: 47 tests                                                                                                                                                |
| npm run test:architecture                            | PASS: 18 tests                                                                                                                                                |
| npm run lint                                         | PASS: ESLint and workspace/all-target clippy with warnings denied                                                                                             |
| npm run validate                                     | PASS after supported native harness corrections; all 205 workspace Rust tests, format/type/lint/frontend/architecture/build/native checks                     |
| Extended actual Tauri/WebView2 native smoke          | PASS: editor/default/TON compilation, declaration display, Unicode marker, cleanup, actual IPC v3, disabled controls, no page errors                          |
| npm run ci:windows                                   | PASS: canonical checks, 205 workspace Rust tests, 192 focused debug tests, 132 release tests, 47 frontend tests, 18 architecture checks and real native smoke |
| npm run ci:core                                      | PASS: Rust fmt/core clippy, 192 focused debug tests, 132 release tests and all 16 portable architecture checks                                                |

Focused local Rust commands executed:

```text
node scripts/rust.mjs test --locked -p plc-types -p plc-ir -p plc-runtime -p simulation-core -p st-frontend -p app-contract
node scripts/rust.mjs test --release --locked -p plc-runtime -p simulation-core -p st-frontend
```

The typed IPC evidence includes 11 app-contract Rust tests, 13 new frontend compile-contract tests, preserved core-information tests and actual native requests. No required Rust test was ignored or filtered. Linux skips exactly two Windows-only BAT checks; these both executed successfully among Windows' 18 architecture checks. They are not reported as Linux passes.

Initial new UI matcher incompatibility and native assumptions about Monaco's old textarea/text-containing underline were corrected without changing PLC assertions. The native test now uses the supported textbox role and checks underline bounds against the real word. One transient Windows file-mapping error during formatting resolved on ordinary retry, without security changes. The final local canonical command passed; superseded failed attempts are not counted as PASS.

## Independent CI evidence

[Canonical validation run 38548231937](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38548231937), attempt 1, push of the exact validated SHA, reached SUCCESS for both required jobs. No CI repair commits were needed.

| Job                            | ID           | Terminal result | UTC execution interval       |
| ------------------------------ | ------------ | --------------- | ---------------------------- |
| Windows canonical validation   | 115799517558 | SUCCESS         | 2026-10-14 10:15:04–10:25:35 |
| Linux headless core validation | 115799518023 | SUCCESS         | 2026-10-14 10:15:04–10:16:04 |

Downloaded and inspected each artifact's result.json and actual test logs. Both lane reports identify the validated SHA; every required stage has PASS, exitCode 0, null signal and null launchError. Each log contains executed test binaries and passing test outcomes, including 77 ST tests in each debug/release lane. Sorted passing-test lists match between platforms: 192 debug and 132 release. The unchanged REAL non-finite, non-reassociation and fixed binary32/subnormal/canonical-zero assertions pass in both profiles on both x64 platforms. Original TON, transactional rollback and clock tests also execute in each applicable profile.

Windows canonical-check.log independently confirms format, typecheck, ESLint/workspace clippy, all workspace/frontend/architecture tests and the production frontend build. native-desktop.log and desktop-smoke/result.json confirm the actual debug Tauri application starts with WebView2, IPC v3 connects, unsupported versions are rejected, default and TON ST compile in Rust, declarations appear, PLC-ST-006 is visible, the Unicode-positioned Monaco marker is correct and obsolete markers clear. All seven panels remain present, runtime values are unavailable, four execution controls remain disabled and pageErrors is empty. The downloaded desktop.png was visually inspected.

Artifacts retained by the unchanged workflow for 14 days:

- windows-validation-1: ID 11886360679, digest `sha256:25568358e75a13f2e38f918fa46679962814cfc1f4c998cc7942a26439d07689`.
- linux-core-1: ID 11885357387, digest `sha256:838f5f32a3a7361e8aa921123cf1c564016780299cd3696115e363b632d3945f`.

Downloaded local evidence is under artifacts/m2a-ci/38548231937, with evidence-audit.json recording the cross-platform comparison. Generated logs, native binaries, browser profiles and screenshots are ignored and were not committed. The publication audit found no credential signatures or forbidden binary/key files among the 168 publication candidates. Repository privacy was verified through the GitHub API; no remote/permissions/collaborator/secret changes were made.

## Regression audit

Accepted M1 plc-types/plc-ir/plc-runtime/simulation-core source and tests remain byte-identical. Their original REAL, TON, clock and transactional behavior tests remain mandatory. M1 validation documents and exact accepted code/gate SHAs are unchanged. Existing contract/native assertions retain meaning with the explicitly assigned v3 migration. Added ST production dependency direction remains acyclic; no other parsing library or runtime was introduced.

## Acceptance criteria

| #   | Criterion                           | Result | Evidence                                                                                 |
| --- | ----------------------------------- | ------ | ---------------------------------------------------------------------------------------- |
| 1   | Real source lexed in Rust           | PASS   | Handwritten lexer; keyword, token, comment and error tests                               |
| 2   | Dedicated ST AST parsed from source | PASS   | Parser tests start with ST text and inspect source-spanned AST                           |
| 3   | AST distinct from canonical IR      | PASS   | Separate ast.rs types; semantic compiler performs translation                            |
| 4   | Accepted type/symbol rules          | PASS   | Existing PlcType, compatibility functions and SymbolTable                                |
| 5   | Existing canonical IR               | PASS   | Source pipeline builds existing Program/Statement/Expression/FB constructors             |
| 6   | Mandatory existing IR validation    | PASS   | CompiledProgram is constructed only after existing validation succeeds                   |
| 7   | Existing ExecutionPlan lowering     | PASS   | Headless tests call existing plc_runtime::lower on ValidatedProgram                      |
| 8   | Existing PLC Runtime execution      | PASS   | Actual compiled ST runs through Runtime::single_scan                                     |
| 9   | Motor follows Start                 | PASS   | pipeline_motor_tracks_start: FALSE/TRUE/FALSE                                            |
| 10  | Start AND NOT Stop                  | PASS   | pipeline_start_and_not_stop_truth_table: all four combinations                           |
| 11  | IF execution                        | PASS   | pipeline_if_selects_existing_runtime_branches: both branches                             |
| 12  | Existing TON behavior               | PASS   | Existing explicit TON catalog/definition/instance/lowering/runtime                       |
| 13  | Exact 100/300 ms TON timeline       | PASS   | pipeline_exact_ton_timeline_and_binding_order_equivalence                                |
| 14  | No duplicate runtime                | PASS   | ST production dependencies contain types/IR only; execution is test-only                 |
| 15  | Source provenance                   | PASS   | Division fault resolves PLC-RUNTIME-003 to original `1 / 0` span                         |
| 16  | Unicode locations                   | PASS   | Scalar-position compiler tests and actual native Unicode marker test                     |
| 17  | Explicit UTF-16 conversion          | PASS   | source-spans.ts and ten position/range tests                                             |
| 18  | INT minimum -32768                  | PASS   | semantic_min_int_is_valid                                                                |
| 19  | INT range errors                    | PASS   | 32768/-32769 diagnosed without wrapping                                                  |
| 20  | Non-finite REAL errors              | PASS   | Existing finite PlcReal construction plus overflow/non-finite tests                      |
| 21  | Checked exact TIME                  | PASS   | Checked integer microseconds; component/order/repetition/overflow tests                  |
| 22  | Compilation limits                  | PASS   | Source/token/nesting/tree/diagnostic limits and malformed-input tests                    |
| 23  | Invalid source cannot lower         | PASS   | pipeline_invalid_source_has_no_executable_wrapper                                        |
| 24  | Deterministic compilation           | PASS   | Equal source/config produces identical owned IR and identity ordering                    |
| 25  | TON argument-order equivalence      | PASS   | Reordered named bindings produce identical complete ScanResults                          |
| 26  | Monaco integrated                   | PASS   | Locally bundled editor/worker; actual native surface                                     |
| 27  | Real typed compile IPC              | PASS   | v3 generated contract and actual Rust/Tauri native compilation                           |
| 28  | Backend diagnostics to markers      | PASS   | Native visible PLC-ST-006 and underline geometry; no frontend semantic errors            |
| 29  | Declaration-only Variables          | PASS   | Name/type/role with unavailable values; native metadata checks                           |
| 30  | Compilation distinct from running   | PASS   | Four compile states, Not running label and stale-response tests                          |
| 31  | Execution disabled                  | PASS   | All four controls remain disabled before/after valid and invalid compilation             |
| 32  | No frontend PLC evaluator           | PASS   | Boundary audit and architecture guard; editor tokenizer is presentation-only             |
| 33  | Accepted regression semantics/tests | PASS   | Frozen core files/tests and historical evidence unchanged; original regressions retained |
| 34  | Independent M2A CI                  | PASS   | Run 38548231937: both jobs SUCCESS; actual test/native artifacts inspected               |
| 35  | No later-stage implementation       | PASS   | Scope/file/dependency audit; process/network/trace/grading remain unavailable            |

## Scope audit

No Run/Pause/Stop/Single Scan UI execution, background PLC task, value polling/watch/forcing, process or network simulation, trace recorder/timeline implementation, grading, LAD/FBD/STL, other timers/counters/edge blocks, user functions/FBs, persistence, cloud or AI generation entered M2A. Existing unavailable process/network/timeline panels remain presentation only.

## Assumptions

Handwritten parsing is the smallest dependency-free implementation for the assigned subset. Signed literals are interpreted at the source boundary; general unary expressions use existing canonical operations. No incompatibility requiring a proposed M1 architecture change was found.

## Known limitations

Recovery targets obvious syntax errors rather than every malformed grammar. The subset and resource ceilings are explicitly documented. Desktop execution/persistence is intentionally unavailable. Monaco is lazy-loaded but its editor chunk is about 2.7 MB before gzip, producing a non-failing Vite size warning; no size gate was weakened.

## Next dependency

Technical-lead source review of the lexer, parser, semantic compiler, typed IPC and editor integration. Validation PASS does not replace this required review or authorize M2B. Stop here.

## Gate

M2A GATE: PASS

All 35 implementation/validation acceptance criteria passed. Source review is pending as explicitly required before the next milestone. M2B has not started.
