# Student interface and Polish localization

The interface uses typed English/Polish dictionaries in `packages/ui/src/locale/`.
Settings persists language, density and beginner explanations in a versioned WebView
record. Dark is the supported appearance. Changes apply immediately; native smoke also
verifies persistence through fresh processes. PLC semantics, project formats, protocols
and the IPC contract are unchanged.

## Exact content coverage

Educational content is **completely localized**: every approved prose field of every
exercise, lesson and article has Polish text or an explicit `keep` reason, and every group
is declared complete so a removed field fails validation. Complete content includes
every approved prose field, not only titles. Localization never rewrites the canonical English knowledge pack;
its only content change in this work was a factual correction of the retentive-memory
wording in nine vendor family articles, recorded in the pack ZIP and its digest.

| Category                                       | Complete Polish content | Polish title + summary |
| ---------------------------------------------- | ----------------------: | ---------------------: |
| Exercises, total                               |                   49/49 |                  49/49 |
| Practice (subset)                              |                   12/12 |                  12/12 |
| Troubleshooting cases (subset)                 |                     8/8 |                    8/8 |
| Commissioning (subset)                         |                   29/29 |                  29/29 |
| Lessons                                        |                   25/25 |                  25/25 |
| Fundamentals                                   |                   25/25 |                  25/25 |
| Instruction articles                           |                     7/7 |                    7/7 |
| Protocol articles                              |                     9/9 |                    9/9 |
| Troubleshooting articles (distinct from cases) |                   30/30 |                  30/30 |
| Vendor/family articles                         |                   24/24 |                  24/24 |

Lessons include objectives, section prose/diagrams and questions/options/explanations.
Practice/case content includes instructions, hints, objectives, scenario names and grading
titles/failure feedback. Cases include symptoms, diagnosis/repair labels and gated debriefs.
Instruction articles include interfaces and equivalence notes for TON, TOF, TP, R_TRIG,
F_TRIG, CTU and CTD. Fundamentals, troubleshooting, protocol and vendor/family articles
are complete,
including facts and simulation fidelity lists. Source code remains original.
Fields that stay identical to English (product names, code, values, identifiers) carry
an explicit `keep` reason; `content:coverage` prints them per reason.

`npm run content:coverage` lists **0 missing display fields**. The checked-in
[machine-readable report](validation/POLISH_CONTENT_COVERAGE.json) lists every item/path.
The [UI literal audit](validation/POLISH_UI_AUDIT.md) lists remaining static candidates by
file/line, with engineering tokens and localization keys excluded. It is a review aid,
not a whole-product percentage. Dynamic/backend/device text needs manual inspection.

## Content model, fallback and search

The strict sibling sidecar `knowledge/locales/pl.json` is bundled into the UI. See
[ADR-0036](adr/0036-bundled-educational-content-locales.md). Each replacement requires an
exact English source match. Missing/stale/custom fields retain English; English mode
uses originals. Switching language updates open cards, lessons, articles and exercise
instructions without refetching or mutating stored data or the PLC. Fallback notices mark
any field that falls back to English (a stale or newly added canonical text). No second
Polish pack is registered.

The desktop build, `content:validate`, `knowledge:validate` and `knowledge:verify` validate
the overlay. Tests enforce declared complete categories and report incomplete categories.
Canonical pack schemas, IDs and deterministic ZIP remain unchanged.

Polish search merges public localized prose from installed library/catalog entries with
backend English search. Accent-insensitive matching supports `opóźnione załączenie` and
`żywe zero`; aliases include `falownik` to `VFD`, `sterownik` to `PLC` and `wymuszenie` to
`force`. All public bodies are translated, so Polish words find every article and
exercise. Filters/ranking are deterministic. Hidden diagnosis/repair/debrief content
is never indexed. Projection cannot introduce absent repair/debrief data or references.
All 37 investigations (8 troubleshooting cases, 29 commissioning labs) have projection
and search anti-leak tests; native smoke verifies real backend gates
before diagnosis and translated repair/debrief availability after successful verification.

## Interface and remaining gaps

The dictionary contains **1,099 keys in each language** (parity and interpolation
parameters verified). Every screen takes its labels from it: navigation, Home, exercises,
lessons, knowledge, progress, Settings and About, and the engineering views — Hardware
(compare, configured, detected, signals, module inspector, channel and loop
configuration), Network (topology, nodes, connection inspector, traffic, ping, network
configuration), Watch tables, variables and online detail, force banner, waveforms, event
log, Diagnostic Buffer, CPU diagnostics, Commissioning, process view labels, toolbar and
editor. `node scripts/ui-locale-audit.mjs` reports no literal candidates; `RETAIN`, `TID`
and `5 min` are listed as identical in Polish with their reasons.

Backend text is worded from codes, never from English sentences ([ADR-0041](adr/0041-coded-backend-messages.md)):

- Commissioning findings and hints, hardware findings (message, expected, detected), the
  signal map's device state and every event carry a coded message with named values.
  `packages/ui/src/locale/backend-pl.ts` words all 307 message and term codes the Rust
  sources contain; a test fails when a code has no Polish wording, when placeholders
  differ, or when a Polish text equals the English one without a stated reason.
- CPU diagnostic buffer entries are worded from their kind and typed details, connection
  states, ping results, Modbus exceptions, channel qualities and module states from their
  codes, and module type names by type, keeping vendor product names as sent.
- Compiler and runtime diagnostics show a Polish title for each of the 42 `PLC-*` codes
  next to the compiler's original technical message (names, types and positions).

Process view component kinds, live quantities and configuration keys are worded by their
identifiers. Text that stays as sent: scene and device labels, tags, project and program names, the
compiler's and runtime's detailed messages, I/O binding issues, hardware validation
errors and knowledge-pack load errors. An unknown code shows the backend's English text.

Basic Free Lab prioritizes Program, Process, variables and causal explanations. Advanced
retains all engineering tools. Lessons keep required views; active fault/force warnings
remain visible. Settings has focus handling and accessible selectors. Card wrapping,
readable article widths, section spacing, code/table scrolling and provenance layout
support longer Polish text without shrinking fonts.

Intentionally unchanged: ST syntax, code/user comments, names/tags, serialized enums,
vendor/protocol/product names and official titles/URLs/revisions. Provenance records,
including our source notes, remain English; translating explanatory source notes is a gap.

## Terminology

| English                   | Polish                                      |
| ------------------------- | ------------------------------------------- |
| commissioning             | uruchamianie                                |
| troubleshooting           | diagnostyka usterek                         |
| hardware / process        | sprzęt / proces                             |
| fault                     | usterka; błąd for program/compiler failures |
| Watch table               | tabela podglądu (Watch)                     |
| force                     | wymuszenie (Force)                          |
| download                  | wgraj do PLC                                |
| field/load supply         | zasilanie obiektowe / zasilanie obciążenia  |
| module electronics supply | zasilanie elektroniki modułu                |
| input/output image        | obraz wejść / obraz wyjść                   |
| scan cycle                | cykl PLC                                    |
| diagnostic buffer         | bufor diagnostyczny                         |
| permissive / interlock    | warunek zezwalający / blokada               |
| seal-in / live zero       | podtrzymanie / żywe zero                    |

PLC RUN/STOP, Structured Text (ST), I/O, PID, VFD and protocol/vendor names retain recognizable
forms. Fidelity descriptions are translated only for display; unknown values fall back.

## Validation and next dependency

See [completion report](validation/POLISH_CONTENT_PASS.md) for commands, resolutions,
release evidence and hosted CI. The Windows policy still blocks some generated Rust test
executables (4551); no security policy change is made. Hosted CI runs full debug/release
Rust suites.

Next safe dependency: the light theme. No new PLC/editor/protocol functionality is
needed.
