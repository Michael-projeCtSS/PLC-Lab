# Knowledge pack

Knowledge, curriculum and simulation content is versioned data in a **knowledge pack**.
The canonical source is `knowledge/PLC_Lab_Knowledge_Pack_v1/`; the release archive is
`releases/knowledge/PLC_Lab_Knowledge_Pack_v1.zip` (+ `.sha256`).

## Layout (pack format 1)

```
PLC_Lab_Knowledge_Pack_v1/
  pack.json            manifest (format, packFormat 1, id, version, license, ...)
  schemas/             JSON Schemas for every content type (authoring and CI)
  sources/             provenance records
  fundamentals/        topic articles
  instructions/        standard instruction articles
  vendors/             vendor and platform-family articles (+ simulation profiles)
  protocols/           industrial protocol articles
  troubleshooting/     troubleshooting guides
  lessons/             curriculum lessons
  exercises/           practice and troubleshooting exercises
  scenes/              simulation scenes
```

Content is JSON; section bodies use a Markdown subset (paragraphs, lists, emphasis,
inline code, code blocks, tables, links shown as text).

## Loading and validation

`knowledge-core` discovers packs in the search roots (each root is a pack or a folder of
packs). It never follows symbolic links and limits nesting depth (6), file count (5 000)
and file size (1 MiB). Every file is parsed with strict deserializers (unknown fields are
errors). Cross-validation checks that exercises reference existing scenes and profiles
and pass `lab-model` validation, and that articles cite known sources. A malformed file
or pack is reported with path and reason in About > Knowledge packs and never prevents
other content from loading. Duplicate identifiers across packs are rejected.

Search roots: `knowledge-packs/` next to the executable, `knowledge-packs/` in the user's
application data folder (install additional packs there), and the repository `knowledge/`
folder in development builds only.

## Search (ADR-0022)

An in-memory BM25-style index is rebuilt on load over titles, summaries, tags, sections,
facts, lessons and exercises. Hyphenated and dotted terms such as `4-20`, `s7-1500` and
`1.0` are indexed whole and in parts. Filters: category (fundamental, vendor specific,
PLC Lab simulation, troubleshooting), vendor and document type. No database is used.

## Provenance policy

- Text is original. Manuals are not copied; facts are summarized in original words.
- Every technical fact can cite source records with title, document number (if known),
  official URL, retrieval date, applicability, notes, confidence and **verification**:
  `verified` (official location and document identity checked when written) or
  `unverified` (identity from the author's knowledge; URL is a vendor portal entry).
- Simplified behavior is marked **SIMULATION APPROXIMATION** in text and in the
  `fidelity` statement; unverified material is labeled.

## Tooling

```
npm run knowledge:validate   # schema validation and identifier uniqueness
npm run knowledge:zip        # deterministic archive (sorted entries, fixed timestamps)
npm run knowledge:verify     # archive = pack byte for byte, schemas pass, no drift
npm run knowledge:install-check  # extract the archive to a folder with spaces and load it as the app does
```

`tests/conformance/tests/knowledge_pack.rs` loads the real pack with the Rust loader and
requires zero errors and warnings, resolvable references, expected search ranking, every
practice exercise solvable by its reference solution (and not by its starter), and every
troubleshooting exercise failing with the fault present and passing after the repair.
`tests/conformance/tests/knowledge_install.rs` drives the application adapter against the
`PLC_LAB_KNOWLEDGE_ROOT` folder (set by `knowledge:install-check`): discovery, indexing,
typical searches, vendor and troubleshooting pages and every catalog entry with its links.

## Contents of v1.0.0

16 fundamentals, 7 instructions, 8 vendors with 16 platform families and 9 simulation
profiles, 9 protocols, 13 troubleshooting guides, 20 lessons, 15 exercises (9 practice, 6
troubleshooting), 4 scenes, 24 source records.
