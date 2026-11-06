# Repository cleanup (2026-11-06)

Every file was classified before anything was removed. Nothing was deleted on a file
name alone; each removal names the evidence.

## Classification

| Class                         | What                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Action                                       |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
| REQUIRED                      | `crates/`, `languages/`, `apps/desktop` (Tauri host and frontend entry), `packages/ui`, `packages/ipc`, `knowledge/` (canonical pack and `locales/pl.json`), `scripts/` (every script is called by `package.json`, CI, a BAT, a test or another script), `tests/` (architecture tests run through `tests/architecture/*.test.mjs`), `.github/workflows`, the root BAT files, workspace manifests and lock files, `config/windows-signing.*.example.json` (one per signing provider, now named in `docs/WINDOWS_CODE_SIGNING.md`) | kept                                         |
| SHARED                        | `Cargo.toml`, `package.json`, `tsconfig.json`, `eslint.config.js`, `vitest.config.ts`, `.prettierrc.json`, `deny.toml`, `rust-toolchain.toml`, `.editorconfig`, `.gitattributes`                                                                                                                                                                                                                                                                                                                                                 | kept                                         |
| GENERATED, tracked on purpose | `packages/ipc/src/generated.ts` (`npm run ipc:generate`, checked by CI), `releases/knowledge/*.zip` and `.sha256` (`node scripts/knowledge-pack.mjs zip`, verified), `docs/validation/POLISH_CONTENT_COVERAGE.json`, `docs/validation/POLISH_UI_AUDIT.md`                                                                                                                                                                                                                                                                        | kept, regenerated where their source changed |
| GENERATED, ignored            | `target/` (Cargo, 52 GB), `node_modules/`, `artifacts/` (test output), the root `PLC Lab.exe` and `resources/` (the staged distribution)                                                                                                                                                                                                                                                                                                                                                                                         | see below                                    |
| USER DATA                     | none inside the repository: projects, workspaces and knowledge packs of a user live under `%APPDATA%\dev.plclab.desktop`, which no step touches                                                                                                                                                                                                                                                                                                                                                                                  | untouched                                    |
| UNCERTAIN                     | `.tools/` (local Rust toolchain used by `scripts/rust.mjs`, the eSigner client, and `.tools/build-recovery/1.99.0-incomplete-*`, an incomplete toolchain copy from the 2026-10-29 recovery)                                                                                                                                                                                                                                                                                                                                      | kept: local tooling, not repository content  |
| OBSOLETE                      | see removals                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | removed                                      |

Historical reports (`docs/*_REPORT.md`, `docs/validation/M1C_*`) are records of past
checkpoints and stay; their one broken link (`LabShell.tsx`, replaced by `App.tsx` in
`4b1b127`) was corrected.

## Removals

- Standalone browser preview (commit `90662fb`): `apps/desktop/preview-content.ts`,
  `packages/ui/src/presentation.ts`, `packages/ui/src/preview/`, `scripts/web-preview.mjs`,
  `scripts/isolated-native-test.mjs` and its three PowerShell helpers, the two web-preview
  e2e scripts and their architecture test, `TEST_PLC_LAB_WEB.bat`, `docs/LOCAL_TESTING.md`
  and the `dev:web`, `test:web*` and `test:native:*` package scripts. Evidence: nothing in
  the Tauri build, CI or the remaining tests used them (ADR-0040 superseded).
- `artifacts/native-tests` and `artifacts/native-test-downloads` (local, 100 MB): Windows
  Sandbox preparations and downloaded CI builds written only by the retired
  `isolated-native-test.mjs`; no remaining script reads them and CI keeps its own
  artifacts.
- Older timestamped WebView2 profiles and data folders under `artifacts/*`
  (`webview-<time>`, `data-<time>`, about 480 MB): written by previous native test runs,
  never read again; the newest of each kind per test folder was kept. The native smoke,
  the development session check and the screenshot run now delete their own profile after
  a passing run (a failing run keeps it), so the folder no longer grows.

`target/` is regenerated by any build and was not cleaned; `cargo clean` remains the
developer's choice.

## Dependencies

- npm: every declared package is used (build, lint, unit and e2e tests); no change.
  `npm audit --omit=dev`: 0 vulnerabilities. Development tooling: one moderate advisory
  (`minimatch` inside the ESLint toolchain, GHSA-g3hv-75wg-m249); its only offered fix is a
  breaking ESLint downgrade, so it is recorded, not forced. It is not shipped.
- Rust: every dependency of every crate is used by name in its sources; `cargo deny`
  (advisories, licenses, sources) runs in CI.
- No version was changed.
