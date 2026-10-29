# Final student UX / Polish content pass

Date: 2026-10-30. Overall result: **PARTIAL**. The authorized complete-category strategy
was used; this report does not claim full Polish product coverage.

## Completed

Added strict source-guarded Polish content overlays, all exercise catalogue text, full
practice content, all lessons, all seven troubleshooting cases/debriefs and all seven
standard-block instruction articles. Localized learning/knowledge/progress/About chrome,
task/result/investigation UI and fidelity labels; open content updates immediately on
language changes. Added Polish public-prose search, English aliases/fallback, readable
cards/readers and localized error headings with original technical details.

## Files

- `knowledge/locales/pl.json`, `content-locale.schema.json`: translations and strict format.
- `scripts/content-locales.mjs`: source/value/code guards, complete-category enforcement
  and exact coverage reporting. Desktop build and knowledge checks run it.
- `packages/ui/src/locale/content.ts`: pure presentation projection and public search.
- `packages/ui/src/pages/`, `workspace/Inspector.tsx`, `Toolbar.tsx`, `useLab.ts`, `useLoad.ts`:
  UI integration. Authoritative Rust/IPC/source/project data are unchanged.
- `tests/architecture/content-locales.test.mjs`, `locale/content.test.tsx`,
  `tests/e2e/desktop-smoke.mjs`: negative/schema/coverage/switching/search/privacy/native tests.
- `POLISH_CONTENT_COVERAGE.json`, `POLISH_UI_AUDIT.md`: exact gaps and static UI audit.

## Architecture

[ADR-0036](../adr/0036-bundled-educational-content-locales.md) documents the supplemental
UI-bundled sidecar. The canonical English pack, IDs, ZIP, schemas, loader, IR, VM, clocks,
grading and protocols remain unchanged. No dependencies were added. Projections replace
only existing exact-match prose; they cannot introduce hidden properties or array entries.

## Tests

| Command / check                                         | Result                                                               |
| ------------------------------------------------------- | -------------------------------------------------------------------- |
| `npm run test:frontend`                                 | PASS: 72 tests, 17 files                                             |
| `npm run test:architecture`                             | PASS: 50 tests                                                       |
| `npm run typecheck`                                     | PASS                                                                 |
| `npm run format:check`                                  | PASS, including Rust formatting                                      |
| `npm run lint`                                          | PASS: ESLint and Rust clippy, all workspace targets, warnings denied |
| `npm run knowledge:validate`                            | PASS: 194 items, zero problems; overlay validation passes            |
| `npm run knowledge:verify`                              | PASS: 179 files, 194 items; original deterministic ZIP               |
| `node scripts/native.mjs build --no-bundle -- --locked` | PASS: actual Tauri release                                           |
| `node scripts/stage-release.mjs`                        | PASS: root executable staged                                         |
| `node scripts/distribution.mjs`                         | PASS: one application executable, required resources                 |
| Native root smoke, `--out polish-content-smoke`         | PASS: 24 steps                                                       |

The relocated release passed **24 steps** (`artifacts/polish-content-relocated`); the
optional BAT entry passed **22 steps** (`artifacts/polish-content-bat`). Both launch
the same root Tauri application; the BAT does not perform native restart steps.

Root `PLC Lab.exe`: 16,464,384 bytes. SHA-256, identical to `target/release/plc-lab-desktop.exe`:
`6ffe9d9ed24695b803c332bdb232ed1aff1359158cf142d55d0523025f4062b9`.
The dictionary has **368/368 Polish keys**; the overlay has **1,419 translated fields**.
These counts do not represent complete product coverage.

The native test uses real WebView2 and Rust IPC. It verifies Polish catalogue/search,
language changes in an open lesson/article, root-cause secrecy before diagnosis,
translated unlocked repairs and verified debriefs, programming/grading/engineering tools,
and preferences through fresh native processes. Screenshots/results are in
`artifacts/polish-content-smoke/`. Visual inspection found and fixed raw filter keys;
the final smoke asserts localized filters and absence of those raw keys.

| Resolution | Result                                                           |
| ---------- | ---------------------------------------------------------------- |
| 1366×768   | PASS: native catalogue/article/Free Lab layout, no page overflow |
| 1920×1080  | PASS: same checks/screenshots                                    |
| 2560×1440  | PASS: same checks/screenshots                                    |

Home and exercise cards are coherent in Polish. The article reader preserves readable
width, scrolling code/tables and provenance. Engineering tools retain full behavior but
still contain English labels/device text; visual layout PASS does not mean translation
completeness. No mock replaces the native acceptance path.

Local full debug/release Rust test execution is still limited by the previously recorded
Windows application-control error 4551. No security setting/toolchain repair was repeated.
Hosted CI is the evidence source for those suites. Prior commit `54c97b9` was checked:
[canonical validation 40967852784](https://github.com/Michael-projeCtSS/PLC-Lab/actions/runs/40967852784)
completed successfully in Windows and Linux before this pass.

## Acceptance criteria

| Requested criterion                                  | Status and exact scope                                                                                            |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Entire product in Polish                             | PARTIAL                                                                                                           |
| Typed PL/EN keys, parity and safe fallback           | PASS for all registered keys                                                                                      |
| Exercise catalogue                                   | PASS: 46/46 titles and summaries                                                                                  |
| Full practice / troubleshooting exercise text        | PASS: 10/10 practice, 7/7 cases                                                                                   |
| Full commissioning content                           | PARTIAL: catalogue 29/29; complete instructions 0/29                                                              |
| All lessons                                          | PASS: 22/22, including questions/feedback                                                                         |
| Knowledge articles                                   | PARTIAL: instructions 7/7; fundamentals 0/23, protocols 0/9, troubleshooting articles 0/29, vendors/families 0/24 |
| Exact automated coverage audit                       | PASS: 2,125 remaining fields listed individually                                                                  |
| Source/tag/value/provenance and grading preservation | PASS: originals unchanged, strict negative and non-prose preservation tests                                       |
| Immediate switching and persistent preference        | PASS: unit and native process-restart checks                                                                      |
| Polish knowledge search                              | PARTIAL: translated public prose + aliases work; untranslated bodies need English                                 |
| Polish root-cause secrecy                            | PASS: all seven projection cases and real native diagnosis/verification gate                                      |
| Advanced engineering UI                              | PARTIAL: primary/task chrome translated; detailed forms remain in literal audit                                   |
| Backend/compiler errors                              | PARTIAL: localized headings/known failure guidance, original technical detail retained                            |
| Settings/About, empty/loading states                 | PASS for touched surfaces; advanced gaps remain                                                                   |
| Reading/cards/focus and three requested resolutions  | PASS for touched surfaces and captured native layouts                                                             |
| Root `PLC Lab.exe` is the real rebuilt application   | PASS: distribution verifier and native smoke                                                                      |

## Assumptions

Use a UI-bundled overlay for the current built-in English pack rather than extending the
portable pack format. Do not rewrite template comments where user comments cannot be
reliably distinguished. Source metadata and official names remain original. Reuse existing
Settings/Basic/Advanced presentation; do not introduce new PLC functionality.

## Known limitations

Full commissioning instructions/findings and four knowledge article categories remain
English, including their titles. Scene/device names, deep Watch/Force/Hardware/Network/
Diagnostics forms, dynamic grading explanations, compiler/runtime detail and provenance
notes remain English. The static UI audit is a review aid, not whole-product coverage.
The sidecar is bundled, not an importable locale for arbitrary third-party packs.

## Next dependency

Translate the full commissioning category, then remaining article categories and detailed
engineering forms using the existing guards. No editor/runtime/protocol expansion is
part of the remaining localization work.
