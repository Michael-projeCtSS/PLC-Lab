# Local Windows build recovery — 2026-10-29

**LOCAL BUILD RECOVERY: PARTIAL.** After the user reported a Norton exception, the
exact pinned Rust toolchain was recovered and the actual root Tauri release application
was built, staged and successfully tested. The complete `BUILD_PLC_LAB_RELEASE.bat`
validation remains blocked by Windows application-control error 4551 when launching
generated test executables. It is not reported as passing. See the final section for
current acceptance results; the earlier sections retain the initial failure evidence.

## Preserved repository state

Work started on clean `claude/product-development-v2` at
`b25e595d699cc9e7460cd4824b2c532372ed1f38`. Existing CP7 commits, including diagnostic
buffer and online-detail work, remain intact. No product/runtime architecture changed.
ADR-0030 continues to require the actual Tauri application in the repository root.

## Initial toolchain and filesystem evidence

| Check                                      | Result                                                                                                                     |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| Pin                                        | `1.99.0-x86_64-pc-windows-msvc`, minimal, rustfmt + clippy                                                                 |
| Local installation                         | Incomplete: actual `rustc.exe`, `rustfmt.exe`, `cargo-fmt.exe`, `rustdoc.exe` absent                                       |
| Actual local Cargo/clippy files            | Present; not sufficient for selection                                                                                      |
| Local cargo/bin                            | rustup proxies present; these previously activated installation                                                            |
| User/system pin                            | Not found in standard user homes; no cargo/rustc/rustup on inherited PATH; no configured external Rust homes               |
| Active rustup loop on arrival              | None; an old release BAT window had no build/toolchain children                                                            |
| ACL/attributes                             | Inspected `.tools` and pinned toolchain `bin`; current user inherits full control, no deny or read-only attribute observed |
| Normal file create/rename/delete           | PASS in dedicated `.tools/plc-build-diagnostic-<id>` folder                                                                |
| Harmless dummy `.exe` create/rename/delete | PASS; never executed                                                                                                       |
| Diagnostic cleanup                         | PASS                                                                                                                       |
| Antivirus                                  | Norton 360 for Gamers registered active; Defender reports Passive Mode and real-time protection disabled                   |

The repository contains no explicit host override in `rust-toolchain.toml`; the Windows
x64 host is derived from the running platform. No latest/stable substitution is used.

## One bounded explicit repair attempt

After verifying fail-fast behavior, one explicit installation used the existing rustup
1.29.1 with the exact pin, minimal profile, rustfmt, clippy and `--no-self-update`.
The diagnostic harness had a 90-second deadline and would terminate only its own rustup
process tree at the first rename retry/access error. It exited with failure after about
five seconds. No further repair was attempted.

The cached official compiler archive SHA-256 is
`20fed82e629c1ed145f9e9bdee9aad65e790d4237a49bcfc61221eda69e35b14`, identical to the
[official Rust checksum](https://static.rust-lang.org/dist/rustc-1.99.0-x86_64-pc-windows-msvc.tar.xz.sha256).

The retained log is `artifacts/build-recovery/bounded-rustup-repair.log` (local,
git-ignored). It records:

- Component installation failure: `rustc-x86_64-pc-windows-msvc`, conflict
  `bin\rust-gdb`, while recovering the previously partial installation.
- Windows error **225** during removal of extraction directory
  `C:\Users\mjgom\Documents\PLC_Student\.tools\rustup\tmp\tj8_qw7u4aeovcr3_dir`:
  “operation did not complete successfully because the file contains a virus or
  potentially unwanted software” (the host emitted the Polish equivalent).
- Its remaining extraction subtree is `rustc\bin`; `rustc.exe` is absent there. The
  log does not emit the exact per-file security detection or a rustc rename pair.
- Intended compiler destination:
  `C:\Users\mjgom\Documents\PLC_Student\.tools\rustup\toolchains\1.99.0-x86_64-pc-windows-msvc\bin\rustc.exe`.
- Rollback rename failures, Windows error **145** (directory not empty):
  `.tools\rustup\tmp\32fld9lj30upckap_dir\bk` to the pinned toolchain's `share`, and
  `.tools\rustup\tmp\fktz_s_spat2tmw5_dir\bk` to its `share\doc`.

The original `rustc.exe` rename/access-denied error **5** was reported in the task;
this attempt did not reproduce that exact error. Error 225 directly establishes a
security rejection during extraction cleanup. Norton is the likely blocker given the
active products, but its detection history has not been inspected and attribution is
not proven. Ordinary filesystem renames work. No ACL, antivirus or Windows security
setting was changed. The partial Rust installation is left for recovery after review.

**Initial required user action:** in Norton Security History, review the Rust 1.99.0 compiler
detection for this `.tools\rustup\tmp` extraction and restore/allow only that specific
official compiler file if the entry confirms it. Do not add a directory exclusion.
The user subsequently reported creating a Norton exception and requested another try.

## Initial validation and remaining acceptance

- Real `BUILD_PLC_LAB_RELEASE.bat`, `PLC_LAB_NO_PAUSE=1`: exits 1 in about **0.6 seconds**,
  reports the missing pinned files and missing system pin, and starts no rustup process.
- Selector regression coverage: incomplete proxy-only installation, direct local pin,
  exact user fallback despite inherited broken homes, configured CI homes, version/host
  and component mismatch, execution errors/timeouts, missing components, floating pin
  rejection, and the real BAT from another working directory in an isolated fixture.
- Focused toolchain/human-build/CI/distribution tests: **29 passed**. JavaScript ESLint,
  Prettier and `git diff --check`: **PASS**. Full architecture testing is blocked at the boundary test's required
  Cargo metadata invocation. Rust formatting, clippy, Rust tests and native builds are
  blocked by the missing compiler. Hosted CI status must be reported separately.
- Existing release staging still copies the Tauri binary byte-for-byte, verifies the
  distribution hashes and removes legacy entry points. The fallback BAT still starts
  only `"%~dp0PLC Lab.exe"`. Those source-level contracts are tested; a successful local
  release is pending.

## Retry after the user-reported Norton exception

The incomplete toolchain was moved, without deleting it, to
`.tools/build-recovery/1.99.0-incomplete-1a60255c355b423fbfc1e7e3ef465ee0`.
Both absolute move targets were verified beneath the repository's `.tools` directory.
A fresh explicit installation of the exact pin, minimal profile, rustfmt and clippy
completed successfully in about seven seconds, without a rename retry. Compiler,
Cargo, formatter, clippy-driver and rustdoc health checks pass using project-local Rust.
The agent did not change any security settings.

The real release BAT passed formatting, TypeScript, ESLint/clippy, all **45 architecture
tests** and **58 frontend tests**. Its Rust debug tests stopped when Windows refused to
execute `target/debug/deps/st_frontend-2b6d4765cb3525b4.exe` with error **4551**
(application-control policy blocked this file). No assertion failed in the executed
tests. The documented release-profile alternative also reached a policy block at
`target/release/deps/grading_causality-d78bc2c976eb0912.exe`. The targeted release
`st-frontend --lib` invocation executed successfully, but that library target has no
tests and is not treated as proof that the full frontend suite passed.

The temporary release-only validation wiring used for that investigation was removed.
The canonical BAT, npm validation gates, debug/release CI lanes and application source
remain as committed in `3f0840b`. No test is skipped or converted into success.

With the compiler healthy, the native build and staging were run separately to satisfy
the physical executable requirement and verify application behavior. This does not
turn the blocked complete validation into a pass.

| Acceptance criterion                                                 | Result                                                                             |
| -------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Exact project-local pinned Rust and health check                     | PASS                                                                               |
| Rustup installation retry loop                                       | FIXED; no retry during recovery                                                    |
| `BUILD_PLC_LAB_RELEASE.bat`                                          | BLOCKED by Windows error 4551 in generated tests                                   |
| `node scripts/native.mjs build --no-bundle -- --locked`              | PASS                                                                               |
| `node scripts/stage-release.mjs` and `node scripts/distribution.mjs` | PASS                                                                               |
| Root `PLC Lab.exe` exists                                            | YES, 16,394,752 bytes                                                              |
| Actual AMD64 PE/Tauri application                                    | PASS; MZ/PE/AMD64 headers checked                                                  |
| Byte identity with `target/release/plc-lab-desktop.exe`              | PASS                                                                               |
| Wrapper or required second PLC application executable                | NO                                                                                 |
| Root native smoke                                                    | PASS, 21 steps                                                                     |
| Different working directory                                          | PASS; smoke starts the application from the Windows temporary directory            |
| Home/navigation and Knowledge/resources                              | PASS, including knowledge search and loading from executable-relative `resources/` |
| Second invocation/single instance                                    | PASS; exactly one application remains and answers                                  |
| Project file argument and hand-over                                  | PASS                                                                               |
| Clean close                                                          | PASS                                                                               |
| Relocated distribution in a path with spaces                         | PASS, 21 steps                                                                     |
| Fallback `RUN_PLC_LAB.bat`                                           | PASS, 20 steps; launches only root `PLC Lab.exe`                                   |
| Legacy entry points and `releases/current` dependency                | ABSENT, verified by distribution check                                             |

Root and compiled executable SHA-256:
`1c0125a9b8889ba33c5c2749fb2e951e8b3742c0b7578c848d9c6cb5442a47b5`.

Hosted [Canonical validation run 40962333885](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/40962333885)
completed with **PASS in both Windows and Linux lanes** for source commit
`3f0840b4565ab8d51430d60c74d36081f8965715`. This includes the full debug/release tests
and Windows native release/distribution smoke checks. Only this recovery document is
changed after that validated application source.

Local evidence:

- `artifacts/build-recovery/rustup-repair-after-user-exception.log`
- `artifacts/build-recovery/release-build-after-recovery.log` (canonical BAT error 4551)
- `artifacts/build-recovery/release-build-release-validation.log` (release test error 4551)
- `artifacts/release-smoke/result.json` and `final.png`
- `artifacts/release-relocated-smoke/result.json`
- `artifacts/release-bat-smoke/result.json`

No product features or runtime architecture changed. Remaining limitation: the local
Windows policy prevents complete execution of the generated debug/release test suites;
do not disable that policy or report the complete BAT as successful. The application
itself can be started by double-clicking the root `PLC Lab.exe`. CP7 feature development
remains outside this recovery task.

Next dependency: a Windows environment/policy-approved execution route for the blocked
generated tests is needed before the full local BAT can be marked PASS. No additional
Norton action is requested for the working application.
