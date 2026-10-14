# ADR-0008: Versioned local project persistence

- Status: Accepted for the foundation under AGENTS.md
- Date: 2026-10-13
- Scope: FND-001, FND-007

## Context

Projects will contain language source/documents, scene configuration, and deterministic run settings. Local learning must not require an account or database server.

## Decision

Plan a local .plclab JSON document with a required schemaVersion, project identity, language documents, process/network scenes, and simulation settings. Source documents are authoritative; compiled IR and traces are derived artifacts, separately versioned when persisted. Large timestamps use decimal strings on JSON boundaries.

Persistence belongs to a Rust application adapter, never plc-runtime or React. Validate size, nesting, identifiers, version, and semantic references before accepting a file. Future migrations must be explicit, tested, and preserve a recoverable original. Reject unknown newer schemas before editing or overwriting; do not silently drop fields. Use temporary-file write and atomic replacement where supported, with a documented failure policy.

## Alternatives

Unversioned JSON invites accidental data loss. A database server or cloud storage violates scope. Serializing UI component state couples saved projects to rendering. Reject these alternatives.

## Consequences and migration

The exact v1 schema, resource limits, path rules, and atomic-save portability need a specification before persistence implementation. No existing projects require migration.

## Sprint boundary

Document the decision only. Do not implement file commands, a database, or project load/save behavior.
