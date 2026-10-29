# ADR-0031: Saved and downloaded station configuration survive reopen

- Status: Accepted for 0.2.0-dev
- Date: 2026-10-29
- Extends: ADR-0029 (configured, downloaded and actual hardware)

## Context

ADR-0029 separated three layers: the project expects a configuration (saved offline), the
PLC has a configuration (the last download), and the physical station is what it is. The
first two were only apart while a workspace was open: the project file stored one
station, and reopening started the PLC with it, so an edit that was saved but never
downloaded silently became "the PLC's" configuration after a reopen or restart.

## Decision

- Project format 4 adds `downloadedStation` beside `station`. `station` is what the
  project expects (Advanced edits are saved there); `downloadedStation` is what was last
  downloaded into the PLC. It is validated like `station`. Format 3 files have none and
  migrate unchanged: the PLC then starts with `station`, as before.
- Opening a workspace starts the PLC with `downloadedStation` (or `station` when none is
  recorded) and records it explicitly, so later saves keep it. A download with a
  configuration sets both: the PLC has it and the project expects it. Saving the project
  changes only `station`.
- The Hardware view already compares the saved project configuration with the PLC's and
  offers Download and Discard; after a reopen or restart that state is the same as before
  closing. Grading (Check) evaluates the project's expected configuration, as before.

## Alternatives considered

Downloading saved edits automatically on reopen (rejected: it hides the difference a
commissioning engineer must see); storing the downloaded configuration outside the
project file (rejected: export, import and Save As would lose it).

## Consequences

Older application versions refuse format 4 files with the "newer format" message. The
loaded program itself is not persisted: a reopened free-lab project starts with no
program in the PLC until Build & Load (troubleshooting labs load their installed program,
as before).
