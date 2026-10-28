# ADR-0022: Versioned knowledge packs with an in-memory search index

- Status: Accepted for the product rebuild
- Date: 2026-10-27
- Input: rebuild assignment (knowledge pack system), audit `feedback.md`

## Context

Lessons, exercises, scenes, platform profiles and reference knowledge must be data, not
React views or Rust code, and must be distributable and replaceable without a new
application build. Searching must work offline on a few thousand items.

## Decision

- A **knowledge pack** is a folder with `pack.json` (pack format 1) and typed content
  folders (`fundamentals`, `instructions`, `vendors`, `protocols`, `troubleshooting`,
  `lessons`, `exercises`, `scenes`, `sources`, `schemas`). Content is JSON with Markdown
  section bodies.
- `knowledge-core` loads packs from host-provided roots, validates strictly (unknown
  fields are errors), cross-validates references and isolates failures per file and per
  pack. Symbolic links are not followed; depth, file count and size are limited.
- Platform simulation profiles are carried by family articles (`simulation`).
- Provenance is mandatory for technical articles: source records with verification
  status `verified` or `unverified`.
- Search uses an in-memory BM25-style index rebuilt on load. **No database**: the corpus
  is small, read-only at runtime and rebuilt in milliseconds; a database server or an
  embedded SQL engine would add dependencies and migration surface without benefit.
- The release archive `PLC_Lab_Knowledge_Pack_v1.zip` is built deterministically by
  `scripts/knowledge-pack.mjs` and verified byte for byte against the source pack.

## Alternatives considered

- SQLite with FTS5: rejected (dependency, schema migrations, no need for writes).
- Content compiled into the binary: rejected (no independent distribution or updates).
- Markdown files with front matter: rejected (weak typing, harder strict validation).

## Consequences

Content authors validate with JSON Schemas and `npm run knowledge:validate`; the Rust
loader remains authoritative. New pack formats require a new `packFormat` and a loader
version check (packs with an unsupported format are rejected with a reason).
