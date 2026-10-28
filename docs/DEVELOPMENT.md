# Development

## Prerequisites

- Node.js 24 (CI pins 24.15.0) with npm.
- Rust 1.99.0 with rustfmt and clippy (`rust-toolchain.toml`). An optional
  workspace-local installation in `.tools/` is used automatically by `scripts/rust.mjs`.
- Windows: Visual Studio Build Tools with "Desktop development with C++" and the Windows
  SDK, and the WebView2 Runtime. Linux: the core, frontend and architecture checks run
  without the Tauri host.

## Repository layout

```
apps/desktop/            Vite entry and Tauri host (src-tauri)
apps/launcher/           root PLC Lab.exe launcher
crates/                  Rust core (see docs/PRODUCT_ARCHITECTURE.md)
languages/st/            Structured Text frontend
packages/ipc/            generated contract, strict validators, typed client
packages/ui/             application shell, pages, engineering workspace, design system
knowledge/               canonical knowledge pack source
releases/                knowledge archive (committed) and staged release (generated)
scripts/                 toolchain, CI, release staging, knowledge pack tooling
tests/architecture/      dependency graph, security posture, CI and tooling tests
tests/conformance/       cross-crate tests incl. knowledge pack solvability
tests/e2e/               native WebView2 smoke test
tests/frontend/          frontend test setup and contract-valid fixtures
docs/                    product and engineering documentation, ADRs, history
```

## Everyday commands

```
npm run dev                      # Tauri development app
npm run typecheck
npm run lint                     # ESLint + clippy -D warnings
npm run format / format:check    # Prettier + rustfmt
npm run test:frontend            # Vitest
npm run test:architecture        # node --test tests/architecture
npm run test:rust                # cargo test --workspace --locked
npm run test:desktop             # native smoke of the debug app
npm run ipc:generate             # regenerate packages/ipc/src/generated.ts
npm run knowledge:validate|zip|verify
```

## Changing the IPC contract

1. Edit DTOs in `crates/app-contract` (every request carries `contract_version`; use
   `deny_unknown_fields`; 64-bit values as `ExactU64`; narrow enumerated strings with
   `#[ts(type = "...")]`).
2. `npm run ipc:generate`.
3. Update validators in `packages/ipc/src/validators.ts` (TypeScript fails if a validator
   no longer matches the generated type).
4. `app-contract`'s drift test and `packages/ipc` tests must pass.

## Adding content

Add JSON under `knowledge/PLC_Lab_Knowledge_Pack_v1/` following the schemas, then run
`npm run knowledge:validate`, the conformance tests (`cargo test -p plc-conformance`) and
`npm run knowledge:zip` to refresh the archive. New exercises need a reference solution in
`tests/conformance/solutions/<exercise-id>.st`.

## Windows Smart App Control

Smart App Control may block freshly built, unsigned executables (os error 4551), including
test binaries and build scripts. Never disable it. Re-run the affected suite in the other
profile (`cargo test --release -p <crate>`) where that works, and rely on hosted CI for
binaries the local policy blocks. Report blocked suites explicitly; do not count them as
passed.
