# M1C independent validation

Authoritative gate record, 2026-10-14. Actual hosted execution completed by 2026-10-14T08:25:08Z; downloaded evidence review completed at 2026-10-14T08:27:39Z.

REPOSITORY: [PLC-Lab](https://github.com/Michael-projeCtSS/PLC-Lab)

PRIVATE: YES

DEFAULT BRANCH: main

VALIDATED CODE SHA: a4a22f19c7bcdbd1c1d78a38804c29e15da32828

WORKFLOW RUN ID: 38533787674

WORKFLOW: Canonical validation, validation.yml, push on main

The exact code SHA appears in both platform result.json files and completed GitHub run metadata. Later gate/metadata commits change documentation only; their identity is recorded separately after commit creation and must not replace the tested code SHA.

## Executed requirements

| Required evidence                                 | Windows x64 MSVC, windows-2022                 | Linux x64 GNU, ubuntu-24.04  |
| ------------------------------------------------- | ---------------------------------------------- | ---------------------------- |
| Rust fmt/clippy                                   | PASS, full workspace                           | PASS, four core packages     |
| Focused debug core                                | PASS, 115 tests                                | PASS, same 115 tests         |
| Optimized runtime                                 | PASS, 52 tests                                 | PASS, same 52 tests          |
| Optimized clock                                   | PASS, 3 tests                                  | PASS, same 3 tests           |
| TON/rollback/reset/determinism                    | PASS, debug and release                        | PASS, debug and release      |
| Unchanged REAL bit/nesting/non-finite assertions  | PASS, debug and release                        | PASS, debug and release      |
| Architecture/tooling                              | PASS, 17 applicable tests, no skips            | PASS, 15 portable tests      |
| Frontend/build and all Rust workspace regressions | PASS, 18 frontend and 122 workspace Rust tests | Not a Linux lane requirement |
| Actual Tauri/WebView2/CDP/Rust IPC smoke          | PASS                                           | Not a Linux lane requirement |

WINDOWS RESULT: PASS

LINUX RESULT: PASS

FOCUSED DEBUG RESULT: PASS

OPTIMIZED RESULT: PASS

NATIVE WINDOWS ACCEPTANCE: PASS

CROSS-PLATFORM REAL RESULT: PASS

Both jobs/every step completed successfully. Each repository stage returned zero without launch errors/signals. Logs contain actual Running/per-case results, not only compiled binaries. Sorted passing core test names are identical across OS targets for both profiles; all Rust suites have zero failed/ignored/filtered cases. The two original Windows BAT cases are inapplicable on Linux and both ran/passed on Windows. No required check was skipped, blocked or unrun.

Native JSON records a launched application, real IPC v2 development response, wrong-contract rejection APP-IPC-001, seven panels, disabled execution controls and no page errors. Real screenshot and native.log are present/reviewed. This validates existing headless semantics and shell/IPC; no M2A frontend or PLC execution UI was added.

## Evidence references

- [Completed run 38533787674](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38533787674).
- [Windows job 115751656572](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38533787674/job/115751656572), completed 08:25:07Z.
- [Linux job 115751656681](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38533787674/job/115751656681), completed 08:16:02Z.
- windows-validation-1 artifact 11879338664: ci/windows stage/setup logs/results and desktop-smoke/native.log, result.json, desktop.png.
- linux-core-1 artifact 11878763534: core/setup logs/results.
- [Remote report](M1C_REMOTE_VALIDATION_REPORT.md): commands, counts, artifact digests, audit and acceptance matrix.

Downloads, full workflow log, metadata and evidence-review.json remain under ignored artifacts/independent-validation/38533787674. Hosted retention is 14 days. No binary/profile/credential was committed as evidence, no CI repair was needed, and no PLC semantic/runtime source or expected result changed.

## Gate

M1C IMPLEMENTATION REVIEW: PASS

M1C INDEPENDENT VALIDATION: PASS

M1C GATE: PASS

This supersedes the earlier final-gate HOLD using actual independent execution. Historical [Code Integrity diagnosis](M1C_WINDOWS_CODE_INTEGRITY.md), [recovery report](M1C_VALIDATION_RECOVERY_REPORT.md) and [preparation report](CI_PREPARATION_REPORT.md) remain preserved; owner-host security/policy is unchanged.

Next dependency: technical-lead review and explicit M2A authorization. M2A was not started.
