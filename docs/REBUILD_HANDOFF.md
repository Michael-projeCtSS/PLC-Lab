# Rebuild handoff

Branch: `claude/product-rebuild-v1` (not pushed until the CI checkpoint).

This file records the resumable state of the product rebuild. It is updated at every
checkpoint commit. The authoritative history is `git log`.

## Checkpoint status

| Checkpoint                                     | State                                                                                   |
| ---------------------------------------------- | --------------------------------------------------------------------------------------- |
| A. Backend workspace builds                    | PASS (`cargo check --workspace --all-targets`, `clippy -D warnings`)                    |
| B. IPC generated, strict tests                 | PASS (`npm run ipc:generate`, `npm run typecheck`, `npm run lint`, `vitest run`)        |
| C. Tauri + rebuilt shell                       | Code complete; native launch not yet validated                                          |
| D. Free Lab workspace                          | UI complete over real backend; native validation pending                                |
| E. Learn / Practice / Troubleshoot / Knowledge | UI complete; knowledge content in progress                                              |
| F. Knowledge ZIP                               | Pending                                                                                 |
| G. Project persistence                         | Backend + UI complete (create, open, save, save as, import, export, autosave of source) |
| H. Grading                                     | PASS in `crates/desktop-session/tests/app.rs` (practice and troubleshooting)            |
| I. Full CI                                     | Pending (workflow still describes the previous milestone)                               |
| J. Native Windows validation                   | Pending                                                                                 |

## Commands that pass locally

- `npm run format:check`
- `npm run typecheck`
- `npm run lint` (ESLint and `cargo clippy --workspace --all-targets --locked -- -D warnings`)
- `npx vitest run`
- `node --test tests/architecture/*.test.mjs`
- `cargo test --workspace --locked --no-fail-fast`: every suite that Windows allows to
  execute passes. Smart App Control (os error 4551) blocks some freshly built test
  binaries; those suites were run with `cargo test --release` and pass, except the
  `app-contract` unit binary, which is blocked in both profiles locally. Its drift check
  is equivalent to `npm run ipc:generate` followed by `git diff --exit-code
packages/ipc/src/generated.ts`, which is clean. Windows security is not bypassed.
- `npm run build --workspace @plc-lab/desktop`

## Architecture decisions in this rebuild

- ADR-0018 world time and PLC modes, ADR-0019 causal observation and compact trace,
  ADR-0020 headless engine boundary, ADR-0021 IR readiness and STL control flow.
- `desktop-session` is an application adapter only (no simulation semantics). Typed
  `AppError` values map to `IpcError` codes only at its public methods.
- The host never interprets error codes: pacing uses the typed `TickOutcome`.
- The UI validates every response with combinators typed against the generated contract.

## Next exact tasks

1. Knowledge content under `knowledge/PLC_Lab_Knowledge_Pack_v1/` (schemas, sources,
   fundamentals, vendors, protocols, troubleshooting, 20 lessons, exercises).
2. Pack ZIP script and `releases/knowledge/PLC_Lab_Knowledge_Pack_v1.zip`.
3. Release staging (`releases/current/`), root launcher `PLC Lab.exe`,
   `RUN_PLC_LAB.bat`, updated BAT helpers.
4. CI workflow update, docs rewrite, `docs/PRODUCT_REBUILD_REPORT.md`, push, CI.
