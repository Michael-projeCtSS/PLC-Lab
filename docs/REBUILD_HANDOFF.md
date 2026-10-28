# Rebuild handoff

Resumable state of the product rebuild. The authoritative history is `git log` on branch
`claude/product-rebuild-v1` (pushed to `origin`, never force-pushed).

## Checkpoint status

| Checkpoint                                     | State                                                                                                                 |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| A. Backend workspace builds                    | PASS locally for every crate except the Tauri host (local host rebuild blocked by Smart App Control); PASS in CI      |
| B. IPC generated, strict tests                 | PASS (`ipc:generate` drift-free, typecheck, ESLint, Vitest)                                                           |
| C. Tauri + rebuilt shell runs                  | PASS: local native debug smoke 2026-10-27 (11 steps)                                                                  |
| D. Free Lab workspace                          | PASS in native smoke and adapter tests                                                                                |
| E. Learn / Practice / Troubleshoot / Knowledge | PASS: content loads, exercises solvable, labs fail until repaired                                                     |
| F. Knowledge ZIP                               | PASS: deterministic, verified byte for byte, Windows Expand-Archive identical                                         |
| G. Project persistence                         | PASS: adapter tests (create, save as, export, import, invalid input)                                                  |
| H. Grading                                     | PASS: practice and troubleshooting grading tests, conformance solvability                                             |
| I. Full CI                                     | See "CI" below                                                                                                        |
| J. Native Windows validation                   | Debug: PASS locally. Release and root launcher: BLOCKED locally (Smart App Control); validated by the CI Windows lane |

## Local environment limitation

Smart App Control on the development machine blocks some freshly compiled, unsigned
binaries (os error 4551): test executables, build scripts (`serde_core` in release) and
proc-macro DLLs (`zerovec_derive`). It was never disabled or bypassed. Consequences: local
release builds and the local root-launcher check cannot run; some test binaries cannot run
locally after relinking. Hosted CI executes all of them.

## Commands that pass locally

`npm run format:check`, `npm run typecheck`, ESLint, `npx vitest run`,
`node --test tests/architecture/*.test.mjs`, `npm run knowledge:verify`,
`npm audit --omit=dev --audit-level=high`,
`cargo check|clippy --workspace --exclude plc-lab-desktop --all-targets --locked`
(`-D warnings`), `cargo fmt --all -- --check`, and every Rust test binary that the local
policy allows to execute.

## CI

The latest run for this branch is listed by `gh run list --branch claude/product-rebuild-v1`.
The Windows lane publishes `plc-lab-windows-release-<attempt>` (staged release, root
launcher, knowledge archive) only when every validation stage passed.

## Next exact tasks

1. If a CI stage fails, read `artifacts/ci/<lane>/<stage>.log` in the uploaded evidence,
   fix the cause and push (no force).
2. Optional product work from [ROADMAP.md](ROADMAP.md) (signed distribution first).
