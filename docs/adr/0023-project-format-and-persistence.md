# ADR-0023: `.plclab` project format v1 and local persistence

- Status: Accepted for the product rebuild (refines ADR-0008)
- Date: 2026-10-27

## Context

Learners must keep their work across sessions: programs, hardware configuration, watch
lists and troubleshooting investigations. Engine state is large, derived and
deterministic; storing it would couple files to internal representations.

## Decision

- One JSON file per project (`plclab-project`, `formatVersion` 1) stored in the
  application data folder, written atomically (temporary file + rename).
- Stored: identity, name, timestamps, kind (free lab or exercise id + revision), platform
  and scene references, station configuration, ST program documents, watch list,
  simulation settings, exercise state (attempts, best score, diagnoses, evidence,
  repairs). Not stored: runtime memory, plant state, traces.
- Strict parsing (unknown fields rejected), explicit migration from version 0, rejection
  of newer versions with an actionable message, validation limits (names, program size,
  watch count, cycle and speed ranges, station validity, 4 MiB file size).
- Progress (lessons, exercises) is a separate `progress.json`.
- Import/export use the same format; imported projects get a new identity.

## Alternatives considered

Snapshotting engine state (rejected: fragile, large, not deterministic across versions);
a database (rejected: unnecessary for single-user local files).

## Consequences

Opening a project always starts a fresh deterministic session; troubleshooting repairs
are reapplied from the stored investigation. Format changes require a new version and a
migration with tests in `project-model`.
