# Student interface and Polish localization

The interface uses typed English/Polish dictionaries in `packages/ui/src/locale/`.
Settings persists language, density and beginner explanations in a versioned WebView
record. Dark is the supported appearance. Changes apply immediately; native smoke also
verifies persistence through fresh processes. PLC semantics, project formats, protocols
and the IPC contract are unchanged.

## Exact content coverage

This is **partial product localization**. Complete content includes every approved prose
field, not only titles. The canonical English knowledge pack remains byte-identical.

| Category                                       | Complete Polish content | Polish title + summary |
| ---------------------------------------------- | ----------------------: | ---------------------: |
| Exercises, total                               |                   17/46 |                  46/46 |
| Practice (subset)                              |                   10/10 |                  10/10 |
| Troubleshooting cases (subset)                 |                     7/7 |                    7/7 |
| Commissioning (subset)                         |                    0/29 |                  29/29 |
| Lessons                                        |                   22/22 |                  22/22 |
| Fundamentals                                   |                    0/23 |                   0/23 |
| Instruction articles                           |                     7/7 |                    7/7 |
| Protocol articles                              |                     0/9 |                    0/9 |
| Troubleshooting articles (distinct from cases) |                    0/29 |                   0/29 |
| Vendor/family articles                         |                    0/24 |                   0/24 |

Lessons include objectives, section prose/diagrams and questions/options/explanations.
Practice/case content includes instructions, hints, objectives, scenario names and grading
titles/failure feedback. Cases include symptoms, diagnosis/repair labels and gated debriefs.
Instruction articles include interfaces and equivalence notes for TON, TOF, TP, R_TRIG,
F_TRIG, CTU and CTD. Source code remains original.

`npm run content:coverage` lists **2,125 missing display fields**. The checked-in
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
untranslated article/commissioning instructions. No second Polish pack is registered.

The desktop build, `content:validate`, `knowledge:validate` and `knowledge:verify` validate
the overlay. Tests enforce declared complete categories and report incomplete categories.
Canonical pack schemas, IDs and deterministic ZIP remain unchanged.

Polish search merges public localized prose from installed library/catalog entries with
backend English search. Accent-insensitive matching supports `opóźnione załączenie` and
`żywe zero`; aliases include `falownik` to `VFD`, `sterownik` to `PLC` and `wymuszenie` to
`force`. Untranslated bodies still require English terms or aliases: search coverage is
**partial**. Filters/ranking are deterministic. Hidden diagnosis/repair/debrief content
is never indexed. Projection cannot introduce absent repair/debrief data or references.
All seven cases have projection anti-leak tests; native smoke verifies real backend gates
before diagnosis and translated repair/debrief availability after successful verification.

## Interface and remaining gaps

The dictionary contains **368 keys in each language** (parity verified). Translated UI includes navigation, Home, exercise cards, lesson/knowledge/progress chrome,
Settings/About, task/investigation/results, primary PLC/simulation controls, variable
headings, force banner and causal explanations. All dictionary keys have Polish entries
with matching interpolation parameters; not every component uses the dictionary yet.

Basic Free Lab prioritizes Program, Process, variables and causal explanations. Advanced
retains all engineering tools. Lessons keep required views; active fault/force warnings
remain visible. Settings has focus handling and accessible selectors. Card wrapping,
readable article widths, section spacing, code/table scrolling and provenance layout
support longer Polish text without shrinking fonts.

Remaining localization gaps: full commissioning instructions/findings; fundamentals,
protocol/troubleshooting/vendor article titles and bodies; process scene/device names;
deep Hardware, Network, Watch, Force, variable detail and Diagnostic Buffer forms/messages.
These are tracked gaps, not permanent English exceptions. No accurate whole-product
percentage is claimed. Template source comments remain English because user/template
comments are not distinguished by rewriting code.

Error notices have a localized primary heading with expandable original detail.
Compile/interface/binding failures and diagnosis feedback have localized primary guidance.
Structured compiler/runtime messages, dynamic grading details and many operation-specific
errors remain English. The Rust backend and technical diagnostics are preserved.

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

Next safe dependency: full commissioning localization, then the remaining article
categories and engineering inspectors. No new PLC/editor/protocol functionality is needed.
