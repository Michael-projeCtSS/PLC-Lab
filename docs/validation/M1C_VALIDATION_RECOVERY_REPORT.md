# M1C-V validation recovery report

Date: 2026-10-14, Europe/Warsaw. Scope: M1C-V-001 through M1C-V-012. M1C SOURCE CODE REVIEW: PASS, as supplied by the technical lead. This task diagnoses the environment and preserves that accepted implementation. Validation recovery did not succeed because Windows still blocks required executables.

## Completed

Identified the actual Smart App Control policy by its event-supplied friendly name/GUID/hash, correlated 3077/3089 and supplementary 3118 events, compared passing/blocked PE files, checked ordinary corruption/dependency/path causes, inspected available code-signing stores/workflows and ran exactly three controlled existing-test samples. All three compiled successfully and were denied before test execution. No PLC feature work or test substitution was performed.

## Files

Only two repository documents were added:

- [M1C_WINDOWS_CODE_INTEGRITY.md](M1C_WINDOWS_CODE_INTEGRITY.md): detailed commands, host observations, every distinct blocked executable, correlated records, comparison hashes and limitations.
- [M1C_VALIDATION_RECOVERY_REPORT.md](M1C_VALIDATION_RECOVERY_REPORT.md): this recovery report and disposition.

Four ignored `artifacts/m1cv/*.json` files retain sanitized local evidence. One missing disposable Cargo TON executable was rebuilt through its normal test command. No release artifact was modified. Existing staged/untracked work was preserved; no commit or remote change was made.

## Active Windows policy

Windows reports Windows 11 Pro 26H2, x64, version/build 10.0.26300.9550. SmartAppControlState is On and VerifiedAndReputablePolicyState is 1. Kernel and user-mode Code Integrity enforcement statuses both report 2.

The actual blocking Event 3077 policy is:

```text
Name:       VerifiedAndReputableDesktop
GUID:       {0283ac0f-fff1-49ae-ada1-8a933130cad6}
PolicyID:   27555.1000.240208
PolicyHash: 2668895A5B233A80432D00D67251D7B7F52686A3FB13780F4B242C5A1F937A01
```

This identifies Smart App Control enforcement from host evidence and Microsoft's friendly-name/state mapping. It is not inferred solely from a GUID. [Microsoft inspection reference](https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/test-your-app-with-smart-app-control)

`CiTool.exe -lp -json` returned access denied, OperationResult -2147024891 (0x80070005). No elevation was requested. Event 3099 was absent from retained logs. Accordingly, the report lists eight existing policy-file names/IDs but does not invent per-policy active/audit/authorization states for the seven other files. The blocking policy itself is confirmed enforced by its 3077 records.

## Code Integrity evidence

The captured original failures cover seven executable paths: debug plc-ir, debug plc-runtime, debug plc-types, debug TON, the debug desktop, release semantic-runtime and release simulation-core. Each has mapped record IDs, timestamps, parent processes and correlated signature records in the detailed diagnosis.

All blocks require signing level 2 but validate at level 1, with status 0xc0e90002. Correlated 3089 records report TotalSignatureCount 0, SignatureType 0, ValidatedSigningLevel 0, VerificationError 0, Publisher/Issuer Unknown and no signer hashes. Zero-signature VerificationError 0 is not a successful execution result. The SDK names the status STATUS_SYSTEM_INTEGRITY_POLICY_VIOLATION. [Microsoft event definitions](https://learn.microsoft.com/en-us/windows/security/application-security/application-control/app-control-for-business/operations/event-tag-explanations)

New controlled-attempt blocks are 39608, 39613 and 39618; associated 3089 records are 39607/39609, 39612/39614 and 39617/39619, matched by exact ActivityID. Their 3118 records 39610, 39615 and 39620 show that Defender/cloud evaluation was attempted, with an empty reported threat name. Raw cloud/trust fields were retained without guessing an undocumented verdict.

## Block classification

**BLOCK CLASS: A — unsigned/untrusted generated executable rejected by enforced Smart App Control.**

The correlated events, Authenticode NotSigned results and fresh-build rejection establish this class. Cloud authorization participated, but the logs do not expose why its per-file trust decision differed between passing and blocked binaries. No specific explicit deny rule, parent restriction, path rule, malformed executable or invalid signed hash was established. Do not equate a policy block with malware detection or a PLC logic failure.

## Passing versus blocked comparison

| Sample                               | Measured result                                                                            | Execution evidence                                                                                    |
| ------------------------------------ | ------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------- |
| Optimized runtime unit executable    | 1,027,072 bytes; AMD64 PE32+; NotSigned; no MOTW                                           | Passed in M1C                                                                                         |
| Optimized TON executable             | 1,056,256 bytes; AMD64 PE32+; NotSigned; no MOTW                                           | All 14 tests passed in M1C                                                                            |
| Debug TON executable                 | Previously missing; normal rebuild produces 1,672,192-byte AMD64 PE32+; NotSigned; no MOTW | Fresh executable blocked in this task                                                                 |
| Optimized simulation-core executable | 715,264 bytes; AMD64 PE32+; NotSigned; no MOTW                                             | Existing unchanged bytes blocked again                                                                |
| Retained older PLC Lab release       | 8,588,288 bytes; AMD64 PE32+; NotSigned; no MOTW                                           | Historical human-tooling report records successful launch; not relaunched as an acceptance substitute |
| New desktop smoke executable         | 12,257,792 bytes; AMD64 PE32+; NotSigned; no MOTW                                          | Prior Node/native launch rejected by the same policy                                                  |

Full paths, creation/modification times, SHA256 values, additional types/workspace executables and event signing fields are in the detailed document. Existing files were not modified during comparison. Older successful debug desktop bytes were overwritten and cannot be recovered from current files. The original debug TON file's disappearance is unexplained; no relevant Defender detection/remediation event was found, and no antivirus attribution is claimed.

Passing and blocked files share unsigned status, x64 architecture and absence of MOTW. Therefore none of those attributes alone explains the difference, and release profile is not a safe substitute for debug/native checks. Historical release semantic-runtime failure and its later passing rebuilt version have different hashes. The exact cloud/cache reason is not established.

## Reproduction

All commands used the normal repository wrapper, working directory and pinned toolchain. Three samples were sufficient; no policy workaround or alternate executable host was used.

| Normal command                                                                                                                                                    | Compilation                              | Process creation    | Command exit / tests executed |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------- | ------------------- | ----------------------------- |
| `node scripts/rust.mjs test --locked -p plc-types --lib function_blocks::tests::ton_signature_is_authoritative_typed_and_independent_of_definition_id -- --exact` | Success, existing test build             | BLOCKED, error 4551 | 1 / 0                         |
| `node scripts/rust.mjs test --release --locked -p simulation-core --lib clock::tests::starts_at_zero_and_advances_exactly_without_wall_clock -- --exact`          | Success, existing release build          | BLOCKED, error 4551 | 1 / 0                         |
| `node scripts/rust.mjs test --locked -p plc-runtime --test ton exact_100ms_cycle_300ms_preset_uses_scan_start_and_caps_elapsed -- --exact --nocapture`            | Fresh compile/relink succeeded in 0.93 s | BLOCKED, error 4551 | 1 / 0                         |

The regenerated TON file and unchanged types/clock files are valid-header x64 PE images with all declared raw sections within file length. Repeated hashes are stable and match the new event flat hashes. MSVC dumpbin can read them; VCRUNTIME140.dll, ucrtbase.dll and directly imported Windows system DLLs are present. No truncation, zero-length file, moved target or missing direct runtime dependency was found.

TEMP/TMP remain the normal user-local Temp folder, CARGO_TARGET_DIR is unset and target resolves inside the repository. Sample source files and binaries do not carry Zone.Identifier. No Cargo output deletion was needed; a missing output was regenerated normally. That fresh build still produced the same policy denial.

The signing audit found zero CodeSigningCert entries in both accessible CurrentUser/My and LocalMachine/My stores. An SDK signing tool exists, but no suitable certificate/private key or configured repository signing workflow was found. Unknown external organization/cloud/HSM provision cannot be ruled out. No signing operation was authorized or attempted.

## Security changes

SECURITY SETTINGS MODIFIED: NO

Smart App Control, CI enforcement, antivirus, trust stores, Secure Boot, registry security configuration and execution policy were not changed. No exclusions, new certificates, signatures, policy refresh/removal, binary relocation or interpreted/renamed execution was used. Defender's pre-existing passive mode and reported real-time state were observed, not modified. The SAC state and blocking policy hash remained unchanged.

## Source changes and architecture

PLC semantic/runtime source remains unchanged. SHA256 comparison of 66 files across crates, language reservations, packages, desktop source and scripts found zero changes. There are no validation-tooling source changes, altered assertions, removed checks, mocks or alternate runtime implementations. Only the two documents and ignored diagnostic/build artifacts above were created. ADRs and the accepted shell/IPC were not redesigned. No M2 work was started.

## Validation retry

The original requirements are unchanged. With all three controlled samples still denied and no provisioned signing route found, M1C-V-011 requires stopping executable retries. Full gates were not rerun merely to generate more known-blocked launches. Historical outcomes below are explicitly identified as M1C evidence, not new executed M1C-V passes. Compilation-only results do not count as test execution.

| Original gate / command                                                                        | Status and actual evidence                                                                                                                                                  |
| ---------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `node scripts/rust.mjs test --locked -p plc-types -p plc-ir -p plc-runtime -p simulation-core` | BLOCKED. Original run passed IR 34 and runtime 38 before debug TON denial; this task reproduced required debug types/TON launch denial. Full command not repeated in M1C-V. |
| `node scripts/rust.mjs test --release --locked -p plc-runtime -p simulation-core`              | BLOCKED. Original runtime 52 passed but optimized clock launch was denied; this task reproduced that unchanged clock executable denial. Full command not repeated.          |
| `node scripts/rust.mjs test --release --locked -p simulation-core`                             | BLOCKED. Required optimized clock lane cannot start; controlled exact-test command above executed zero tests.                                                               |
| `npm.cmd run validate`                                                                         | BLOCKED. Original command ended with policy-denied workspace runtime unit executable. Prerequisite launch denial persists; full command not repeated.                       |
| `npm.cmd run test:rust`                                                                        | BLOCKED. Original workspace run passed contract 5, conformance 2 and IR 34 before denied runtime unit launch. Not rerun in M1C-V.                                           |
| `npm.cmd run test:desktop`                                                                     | BLOCKED. Original frontend/native build succeeded, then Node spawn UNKNOWN corresponded to policy block 39598. Not rerun in M1C-V or replaced by the older release.         |
| `npm.cmd run test:architecture`                                                                | PASS in original M1C validation: 10 tests; no new M1C-V execution claimed.                                                                                                  |
| `npm.cmd run test:frontend`                                                                    | PASS in original M1C validation: 18 tests; no new M1C-V execution claimed.                                                                                                  |
| `npm.cmd run lint`                                                                             | PASS in original M1C validation: ESLint and workspace/all-target clippy; no new M1C-V execution claimed.                                                                    |
| `npm.cmd run build`                                                                            | PASS in original desktop-build lane: TypeScript and production Vite build; no new M1C-V execution claimed.                                                                  |
| `npm.cmd run format:check`                                                                     | PASS in M1C-V after formatting the two new documents, including Rust fmt; accepted source files unchanged.                                                                  |

The prior 14 optimized TON tests and 52 optimized runtime tests remain useful evidence. They do not satisfy the blocked debug, optimized clock, full workspace or native lanes. The validation gate stays failed until every original required gate actually executes and passes.

## Acceptance criteria

| M1C-V requirement                          | Result                                                                                                                                    |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- |
| 001 Preserve security                      | PASS: no setting, trust or policy changes                                                                                                 |
| 002 Identify active policy                 | PARTIAL: actual blocking policy/state identified; CiTool denies full per-policy enumeration and 3099 is unavailable                       |
| 003 Correlate CI events                    | PASS: every captured blocked executable and 3077/3089 relationship documented; 3118 supplementary evidence retained                       |
| 004 Classify using evidence                | PASS: unsigned/untrusted enforced-policy rejection; cloud-rule detail explicitly unresolved                                               |
| 005 Compare binaries                       | PASS: retained release/runtime and blocked debug/clock/desktop samples measured; missing/overwritten original bytes explicitly identified |
| 006 Minimal reproduction                   | PASS: exactly three normal existing-test samples; all actual outcomes recorded                                                            |
| 007 Benign checks                          | PASS: PE/ranges/hash/import/path/MOTW checks and normal regeneration; fresh output still blocked                                          |
| 008 Signing availability audit             | PASS: both stores and repository inspected; no provisioned usable route found                                                             |
| 009 Preserve test architecture             | PASS: no substitutions or assertion changes                                                                                               |
| 010 Recover original validation if allowed | BLOCKED: required executables remain denied; no gate marked passed from compile-only or historical-only evidence                          |
| 011 Stop and provide owner options         | PASS: diagnosis complete, no policy workaround attempted                                                                                  |
| 012 No feature changes                     | PASS: 66 inspected source/config/script files byte-identical                                                                              |

## Assumptions and known limitations

The supplied technical-lead source-code PASS is authoritative; earlier documents are retained as historical validation evidence. No hidden organization-managed signing route is assumed. CiTool access is unavailable without privileges, no 3099 activation events are retained, cloud trust reasons are not publicly decoded from the observed raw fields, and old overwritten/missing binary bytes cannot be reconstructed. These limit the diagnosis but do not weaken the directly observed policy rejection.

## Recommended owner options

These are choices for the owner, not actions performed by this task.

| Option | Owner-controlled route                                                                                                                                                                                                                          |
| ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A      | Approve a dedicated development machine or VM whose application-control policy supports normal local build/test executables while retaining its intended protections.                                                                           |
| B      | Provision or authorize a legitimate trusted signing workflow meeting Smart App Control/active-policy requirements for generated tests and desktop binaries. No certificate/workflow was provisioned here.                                       |
| C      | If the machine is organization-managed, ask its authorized administrator whether an appropriately scoped organization-managed App Control route is supported. Do not assume consumer Smart App Control accepts an ordinary supplemental policy. |
| D      | Approve another validation environment for core debug/release tests, while retaining a separately approved Windows x64 native Tauri/WebView2 acceptance lane. Passing core tests elsewhere does not replace native acceptance.                  |

Option B requires a genuine policy-recognized signing route; an untrusted self-signed root or simply installing signtool is insufficient. Microsoft describes supported code-signing choices for Smart App Control. [Microsoft signing guidance](https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/code-signing-for-smart-app-control)

None was performed. Disabling Smart App Control or reducing required tests is not proposed as an automated recovery.

## Next dependency

Owner approval of a suitable validation environment or legitimate policy-compatible signing route, followed by every original M1C gate. Technical-lead code review is already accepted; M2 remains unauthorized.

## Gate

M1C VALIDATION RECOVERY: HOST POLICY BLOCKED

M1C GATE: FAIL

Stopped before M2.
