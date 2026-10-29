# Local Windows build recovery — 2026-10-29

**LOCAL BUILD: BLOCKED.** The fail-fast build change is implemented, but the root
`C:\Users\mjgom\Documents\PLC_Student\PLC Lab.exe` has not been created. No launch,
resource, different-working-directory, single-instance or project-argument acceptance
result is claimed.

## Preserved repository state

Work started on clean `claude/product-development-v2` at
`b25e595d699cc9e7460cd4824b2c532372ed1f38`. Existing CP7 commits, including diagnostic
buffer and online-detail work, remain intact. No product/runtime architecture changed.
ADR-0030 continues to require the actual Tauri application in the repository root.

## Toolchain and filesystem evidence

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

**Required user action:** in Norton Security History, review the Rust 1.99.0 compiler
detection for this `.tools\rustup\tmp` extraction and restore/allow only that specific
official compiler file if the entry confirms it. Do not add a directory exclusion.

## Validation and remaining acceptance

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

Next dependency: review the specific security detection, recover the exact pinned
toolchain, run `BUILD_PLC_LAB_RELEASE.bat`, then verify the actual root executable and
all native release smoke scenarios before resuming CP7 feature development.
