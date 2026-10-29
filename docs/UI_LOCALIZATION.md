# Student interface and Polish localization

The interface has centralized typed English/Polish dictionaries under
`packages/ui/src/locale/`, a Settings button in the shell, durable interface preferences,
and an English runtime fallback. The Rust execution, simulation, trace, grading, project
format and desktop IPC contract are unchanged. No dependency was added.

## Scope

Translated surfaces: main navigation, Home orientation/actions, Free Lab project list
and creation flow, exercise/commissioning/troubleshooting navigation and introductions,
Settings, About labels, workspace tabs, primary build/load and PLC/simulation controls,
variables headings/empty state, force warning, causal sequence and important tooltips.

Settings offers Polski/English, Dark (the only supported appearance), Comfortable or
Compact density and beginner explanations. Changes apply immediately. Language and the
other interface preferences survive closing/reopening the app. Settings displays real
version/build/knowledge-pack information through the existing About backend queries.

Basic Free Lab prioritizes Program, Process, essential variable monitoring and the causal
inspector. The project tree, hardware/network editing, commissioning, Watch, waveforms,
events and deep properties appear in Advanced. Diagnostic access remains visible when
an alarm is present; active forces remain prominently visible in either mode. Lessons
and fault-finding labs retain the views needed to complete their tasks. No PLC state is
reset by a presentation-mode switch.

PLC mode controls and simulation controls have separate labelled groups. Keyboard
accessible descriptions explain their semantics. Enabled beginner explanations also
show a visible help bubble when a control receives keyboard focus. Build/load success
has a compact confirmation; compiler diagnostics appear above the editor, with buttons
that focus their source location. New error messages bring the messages dock forward.

Settings has labelled selectors, initial focus, Escape dismissal and focus restoration.
Selected tabs use `aria-selected`; button focus remains visible. Navigation can scroll
within the header at narrower sizes while retaining access to Settings. Native smoke
checks the Polish Free Lab at 1366×768, 1920×1080 and 2560×1440.

## Deliberately untranslated areas

This is **partial product localization**, not a translated course catalogue. Lesson and
knowledge articles, exercise titles/summaries/hints from packs, process component names,
deep hardware/network/Watch inspectors, commissioning checklist findings, some advanced
forms and structured backend compiler/runtime diagnostics remain English. Source code,
user names/tags, industrial protocol/vendor names and typed values are preserved.
Remaining learning-content translation must use explicit content locales rather than
modifying canonical source examples or replacing arbitrary text in the DOM.

## Terminology

| English           | Polish interface term                                            |
| ----------------- | ---------------------------------------------------------------- |
| commissioning     | uruchamianie                                                     |
| troubleshooting   | diagnostyka usterek                                              |
| hardware          | sprzęt                                                           |
| process           | proces                                                           |
| fault             | usterka; błąd for program/compiler errors                        |
| diagnostics       | diagnostyka                                                      |
| Watch table       | tabela podglądu (Watch)                                          |
| force             | wymuszenie (Force)                                               |
| download to PLC   | wgraj do PLC                                                     |
| field/load supply | zasilanie obiektowe / zasilanie obciążenia, according to context |
| scan              | cykl PLC                                                         |
| input image       | obraz wejść                                                      |

PLC RUN/STOP, Structured Text (ST), LAD, FBD, CFC, SFC, I/O, vendor names and protocol
names retain their recognizable engineering spelling.

## Validation

- TypeScript, ESLint and frontend production build.
- Frontend tests: language switch/persistence, selectors, failure reporting, dictionary
  completeness/interpolation/fallback, locale default, Basic/Advanced visibility,
  separate PLC/simulation groups and the online-monitoring empty state.
- Existing architecture tests retain the single authoritative Rust runtime and root
  one-EXE distribution requirements.
- Native smoke uses the real root Tauri executable, changes language/preferences,
  reloads and closes/restarts the app, exercises the backend and verifies the Polish
  workspace at all three sizes. Screenshots/results are saved under
  `artifacts/student-ux-smoke` locally.

Local results for this slice: **67 frontend tests passed**, **45 architecture tests
passed**, TypeScript/ESLint/formatting passed, native release compilation/staging passed,
and the root native smoke passed **23 steps**. The last step reuses the same WebView
profile through two fresh native processes and verifies persisted English, then Polish,
density and explanation choices. The distribution verifier confirms the root executable
is byte-identical to the compiled Tauri binary and requires no second application EXE.

| Requested criterion                                                | Acceptance                                                           |
| ------------------------------------------------------------------ | -------------------------------------------------------------------- |
| Typed PL/EN dictionaries, completeness and English fallback        | PASS                                                                 |
| Language switch and persistence, Polish locale default             | PASS                                                                 |
| Settings outside About, density and explanations                   | PASS                                                                 |
| Supported appearance                                               | PASS: Dark; Light/System intentionally deferred                      |
| Home orientation and primary continuation action                   | PASS                                                                 |
| Basic Free Lab and complete Advanced toolset                       | PASS                                                                 |
| Lesson-required views and persistent force/fault visibility        | PASS                                                                 |
| Distinct PLC/simulation groups, primary build/load, error guidance | PASS                                                                 |
| Important empty states and concise accessible explanations         | PASS                                                                 |
| 1366×768, 1920×1080 and 2560×1440                                  | PASS, native screenshots inspected                                   |
| Focus, accessible names/descriptions, selected-tab state           | PASS for touched controls                                            |
| Root `PLC Lab.exe` distribution preserved and rebuilt              | PASS                                                                 |
| Entire product/course Polish translation                           | PARTIAL: explicitly excluded content and advanced views listed above |

Next safe dependency: translate the remaining advanced UI surfaces using these keys,
then introduce localized educational content as a separate task. No language editor,
runtime or protocol expansion belongs to this slice.

The existing Windows policy limitation on certain generated Rust test executables
(error 4551) is unchanged. No repair or security-setting change is part of this task.
Hosted CI remains responsible for the full debug/release Rust suites. The root
`PLC Lab.exe` remains the actual staged Tauri application, with data in `resources/`.
