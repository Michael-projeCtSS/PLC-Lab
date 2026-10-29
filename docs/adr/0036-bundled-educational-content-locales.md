# ADR-0036: Source-guarded educational content overlays

- Status: Accepted within the requested content localization scope
- Date: 2026-10-30

## Decision

Keep the canonical English pack, IDs, schemas, deterministic ZIP and Rust loader
unchanged. Bundle `knowledge/locales/pl.json` into the UI as a supplemental presentation
resource for `plclab.core` version `1.0.0`. It is not a second pack or new runtime.

Entries identify a canonical item/file and approved display paths, with the exact English
original and Polish replacement. The strict schema/validator rejects unknown structure,
IDs/files, duplicate paths, stale originals, executable/configuration fields, changed
numeric values/code examples and missing required-interface identifiers. Declared complete
categories are enforced; every remaining field is reported. Desktop builds and knowledge
validation/ZIP verification run this validator. No dependency or IPC field is added.

Replace only existing strings with matching sources at presentation boundaries. Preserve
original objects; language switches re-project without refetching, saving translated
projects, recompiling or changing simulation state. Missing/stale/custom content retains
English. User code, comments and names remain unchanged. Exercise toolbars display the
localized exercise title without changing the stored project name.

Projection cannot add properties or array entries. Repairs/debriefs remain absent until
Rust returns them. Search uses installed items and public title/summary/objective/section
prose, excluding hidden diagnosis choices, repairs, hints and debriefs. Existing English
backend search remains available. This protects presentation sequencing, not encryption
of educational assets.

## Alternatives and migration

Localized canonical objects would change schemas and Rust DTOs. Another pack would
duplicate IDs. Sidecars inside the pack would change strict discovery and ZIP semantics.
A sibling overlay is the smallest bounded change. Its bytes are bundled into Tauri and
covered by distribution hash verification; the English pack remains independently usable.

Future importable locales require an explicit pack-discovery extension and migration of
this versioned format. This pass does not translate arbitrary third-party packs or load
locale files dynamically. See `docs/UI_LOCALIZATION.md` for coverage and limitations.
