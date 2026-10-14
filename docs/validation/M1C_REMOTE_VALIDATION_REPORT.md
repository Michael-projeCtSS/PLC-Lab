# M1C remote validation report

Date: 2026-10-14. Scope: M1C-CI-RUN, resumed at REMOTE-003 following owner installation/authentication of GitHub CLI. Evidence review: 2026-10-14T08:27:39Z. The earlier CLI-blocked attempt is retained in baseline Git history.

## Repository publication

REMOTE CREATED: YES

PRIVATE: YES

REPOSITORY NAME: PLC-Lab

REPOSITORY: [PLC-Lab](https://github.com/Michael-projeCtSS/PLC-Lab)

DEFAULT BRANCH: main

REMOTE NAME: origin

REMOTE URL: https://github.com/Michael-projeCtSS/PLC-Lab.git

VALIDATION COMMIT SHA: a4a22f19c7bcdbd1c1d78a38804c29e15da32828

VALIDATED CODE SHA: a4a22f19c7bcdbd1c1d78a38804c29e15da32828

GitHub CLI 2.102.0 was available at its installed Program Files path. `gh auth status` succeeded and `gh api user` confirmed the active account. Credential values were omitted. The fresh local remote check was empty. The authenticated owner's PLC-Lab lookup returned HTTP 404; creation used --private. GitHub API checks before publication and after validation confirmed private=true and default_branch=main.

Preserved and committed all 149 audited files, including accumulated staged/unstaged/untracked M0–M1C work, human-test tooling, CI preparation and validation documentation. Used the existing author configuration without changing it. Baseline message: `PLC Lab: baseline through M1C and independent validation workflow`. Added credential-free HTTPS origin and pushed main normally with its upstream. No history rewrite, force push, unrelated overwrite, tag, release, collaborator or secret was involved.

Complete staging exposed two whitespace issues: Git interpreted the verbatim AGENTS.md CRLF bytes as trailing whitespace, and Cargo.toml had an extra EOF blank line. Added a file-specific cr-at-eol attribute while retaining the normal whitespace checks; removed only the manifest's extra blank line. AGENTS.md bytes and Cargo semantics were preserved. Corrected `git diff --cached --check` passed before committing.

## Secret audit

PUBLICATION SECRET AUDIT: PASS

Fresh audit at 2026-10-14T08:11:26Z covered 149 unique working-tree candidates and all 28 previously staged versions: 175 text versions and two icon assets. Checked private-key/token signatures, credential assignments/URLs, environment/credential/certificate files, executable archives, logs and generated directories. No suspected secret or prohibited path was found; no credential value was printed. Final baseline index contained exactly the audited 149 files, including AGENTS.md and validation.yml.

target, node_modules, .tools, artifacts, dist, logs and releases/current remain ignored; .env and .env.local ignore probes passed. Only releases/README.md is included from releases. No generated executable, personal profile or credential file was published. Later documentation changes were checked separately. This is a reasonable publication audit, not proof against every possible secret format or an audit of unrelated host stores.

## Workflow

WORKFLOW NAME: Canonical validation

WORKFLOW FILE: .github/workflows/validation.yml

RUN ID: 38533787674

RUN COMMIT: a4a22f19c7bcdbd1c1d78a38804c29e15da32828

RUN BRANCH: main

RUN EVENT: push

RUN START: 2026-10-14T08:15:08Z

RUN TERMINAL UPDATE: 2026-10-14T08:25:08Z

WINDOWS JOB: PASS

LINUX JOB: PASS

[Completed run](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38533787674), [Windows job 115751656572](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38533787674/job/115751656572), [Linux job 115751656681](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/38533787674/job/115751656681).

Used the automatic push run; no manual dispatch was necessary. `gh run watch --interval 30 --exit-status` reached success after both jobs completed. Every workflow step succeeded. Workflow/setup/orchestration files, dependencies and assertions remained unchanged; Actions permissions remain contents: read and checkout does not persist credentials.

## Windows evidence

windows-2022 x64 / MSVC Rust 1.99.0 / Node v24.21.0. Job ran 08:15:11Z–08:25:07Z. Every stage records PASS, exitCode=0, no signal/launch error and the exact validated revision.

| Executed requirement                                                               | Actual result                                            | Evidence inside windows-validation-1                          |
| ---------------------------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------- |
| npm run check: Prettier, Rust fmt, TypeScript, ESLint, workspace/all-target clippy | PASS                                                     | ci/windows/canonical-check.log                                |
| Architecture/tooling and frontend tests                                            | PASS: 17 architecture/tooling, 18 frontend, no skips     | ci/windows/canonical-check.log                                |
| All Rust workspace regressions                                                     | PASS: 122 executed tests, zero failures/ignored/filtered | ci/windows/canonical-check.log                                |
| Focused debug types/IR/runtime/clock                                               | PASS: 26 + 34 + 52 + 3 = 115                             | ci/windows/m1c-debug.log                                      |
| Optimized plc-runtime                                                              | PASS: 10 unit + 28 semantic + 14 TON = 52                | ci/windows/runtime-clock-release.log                          |
| Optimized simulation-core                                                          | PASS: 3 clock tests                                      | ci/windows/runtime-clock-release.log                          |
| Production frontend and actual Tauri build                                         | PASS                                                     | ci/windows/canonical-check.log, ci/windows/native-desktop.log |
| Actual native Tauri/WebView2/CDP Rust IPC smoke                                    | PASS                                                     | ci/windows/native-desktop.log, desktop-smoke/result.json      |

Rust logs contain actual Running entries for .exe processes and per-case results. Debug/release both executed all 14 TON integration cases and private transaction tests. Epoch/memory rollback after arithmetic, budget and completion faults, exact 100 ms/300 ms timing, reset and deterministic repeated sequences passed. Compilation-only evidence was not substituted.

## Linux evidence

ubuntu-24.04 x64 / GNU Rust 1.99.0 / Node v24.21.0. Job ran 08:15:10Z–08:16:02Z. Same validated revision; all five stages returned zero without launch errors/signals.

| Executed requirement                               | Actual result                   | Evidence inside linux-core-1               |
| -------------------------------------------------- | ------------------------------- | ------------------------------------------ |
| Rust fmt and core/all-target clippy                | PASS                            | core/rust-format.log, core/core-clippy.log |
| Debug plc-types/plc-ir/plc-runtime/simulation-core | PASS: 115 executed tests        | core/m1c-debug.log                         |
| Optimized runtime/clock                            | PASS: 52 runtime + 3 clock = 55 | core/runtime-clock-release.log             |
| Portable architecture/tooling                      | PASS: 15 applicable tests       | core/architecture.log                      |

The two existing Windows BAT-only cases report SKIP on Linux under their original platform guards; both executed/passed in Windows's 17/17 suite. They are not Linux portable requirements. No required semantic/runtime test was skipped, ignored, blocked or left unrun. No guard was changed. Linux corroborates core behavior and does not replace Windows native acceptance.

## REAL determinism

WINDOWS REAL DETERMINISM: PASS

LINUX REAL DETERMINISM: PASS

CROSS-PLATFORM REAL RESULT: PASS

All three existing tests passed unchanged in debug and release on both x64 targets:

- real_results_have_fixed_binary32_bits_subnormals_and_canonical_zero
- real_operation_nesting_is_not_reassociated_or_constant_folded
- real_nonfinite_arithmetic_cannot_enter_authoritative_memory

Compared the complete sorted passing-test name sets: identical 115 debug and 55 release cases across Windows/Linux, zero failed/ignored/filtered. No platform-specific expected bits, normalization, new arithmetic semantics or disabled assertion was introduced. This evidence applies to the recorded targets/toolchain/profiles.

## Artifact review

| Artifact             | ID          | Archive digest                                                          |
| -------------------- | ----------- | ----------------------------------------------------------------------- |
| windows-validation-1 | 11879338664 | sha256:52eb65efbf600e41c8d79fac86700912c7e2f418b9e1f8232b2021eb138da16c |
| linux-core-1         | 11878763534 | sha256:b01519bfa4028266495d6ad4c6e59186f4126720e0e0060c5566658927211faf |

Downloaded/reviewed both archives, completed run metadata and full workflow log. Checked all setup and stage result records against the exact SHA. Viewed the real desktop screenshot. Native JSON confirms applicationStarted=true, actual IPC v2 development response, APP-IPC-001 wrong-contract rejection, unavailable PLC execution, seven panels, disabled controls and no page errors. native.log exists and is empty because the successful application emitted no stdout/stderr. No executable/profile was uploaded.

Windows evidence contains ci/windows stage logs/results, setup logs and desktop-smoke/native.log, result.json, desktop.png. Linux contains core stage logs/results and setup logs. Local downloads and evidence-review.json remain under ignored artifacts/independent-validation/38533787674. Hosted artifact retention is 14 days.

## CI repairs

CI-ONLY REPAIR ITERATIONS: 0 / 2

CI-ONLY FIX COMMIT SHAS: NONE

PLC SEMANTIC/RUNTIME SOURCE CHANGED: NO

First hosted run passed. Only pre-baseline non-report edits were the two publication whitespace corrections above. Hash comparison with the initial audit confirmed all other existing files unchanged, including core/runtime/IR/types, tests, UI/IPC, native smoke, workflow, CI scripts, lockfiles and toolchain. Later commits record documentation only. No Windows security setting, signing material or Actions permission was changed.

## Native acceptance

NATIVE WINDOWS ACCEPTANCE: PASS

The unchanged test built/launched the actual native executable, connected to its WebView2 CDP endpoint, invoked real Rust IPC and completed its assertions. No mock or hosted-runner limitation was needed.

## Local checks

Fresh secret/path audit and CLI/account/remote checks passed. Corrected staged whitespace check passed. `npm.cmd run format:check` passed, and `npm.cmd run test:ci` passed all seven tests. Final documentation formatting, Git whitespace and protected-source comparisons passed. These supplement actual hosted execution; blocked local PLC/native executables were not retried.

## Acceptance criteria

| Requirement                          | Result                                                                                          |
| ------------------------------------ | ----------------------------------------------------------------------------------------------- |
| REMOTE-001 audit                     | PASS: current tree/index, no publication risk found                                             |
| REMOTE-002 preserve work             | PASS: complete approved state, no destructive recovery                                          |
| REMOTE-003 authentication            | PASS: CLI, active account and auth verified                                                     |
| REMOTE-004 identity/privacy          | PASS: intended absent owner repository created private and rechecked                            |
| REMOTE-005 baseline                  | PASS: audited 149-file commit with whitespace checks                                            |
| REMOTE-006/007 origin/push           | PASS: secure HTTPS, normal upstream main push, exact SHA                                        |
| CI-RUN-001 trigger                   | PASS: automatic push run used                                                                   |
| CI-RUN-002/003 platform execution    | PASS: all Windows and portable Linux requirements executed                                      |
| CI-RUN-004 REAL                      | PASS: unchanged assertions on both OS/profile combinations                                      |
| CI-RUN-005 terminal watch            | PASS: both jobs completed successfully                                                          |
| CI-RUN-006/007 failure/repair policy | PASS: no hosted failure, zero repairs, assertions/semantics preserved                           |
| CI-RUN-008 native                    | PASS: actual Tauri/WebView2/CDP/Rust IPC                                                        |
| CI-RUN-009 artifacts                 | PASS: metadata, logs, archives, native JSON and screenshot reviewed                             |
| CI-RUN-010/011 success/gate          | PASS: all required executions, gate record created                                              |
| CI-RUN-012 incomplete-gate policy    | PASS: no blocked/unrun requirement relabeled PASS                                               |
| CI-RUN-013 stop                      | PASS: M2A not started                                                                           |
| Security constraints                 | PASS: private/read-only Actions, no credentials/secrets/collaborators/releases/security changes |

## Gate and commit identity

M1C IMPLEMENTATION REVIEW: PASS

M1C INDEPENDENT VALIDATION: PASS

M1C GATE: PASS

The validated code SHA is the baseline above. Later gate-recording/metadata commits change documentation only and are not represented as the tested code revision. The exact gate-recording SHA is recorded after that commit exists. See [M1C_INDEPENDENT_VALIDATION.md](M1C_INDEPENDENT_VALIDATION.md) for the authoritative record. Historical local Smart App Control reports remain preserved.

## Architecture, assumptions and known limitations

No product architecture changed. Used the authenticated user's private account and prepared runner families/commands. Native acceptance is the existing debug smoke; release desktop launch is not required here. Local Smart App Control remains unchanged and can still block owner-host executables. Independent PASS applies to the recorded Windows/Linux x64 code revision; it does not repair local policy.

## Next dependency

Technical-lead review of this first external validation report, followed by explicit M2A authorization. Stopped after validation and gate documentation. M2A was not started.
