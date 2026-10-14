# ADR-0001: Desktop stack

- Status: Accepted for the foundation under AGENTS.md
- Date: 2026-10-13
- Scope: FND-001, FND-003, FND-004, FND-005

## Context

PLC Lab needs a local desktop UI and an authoritative, testable simulation core. The engineering contract specifies Tauri 2, React, TypeScript, and Rust.

## Decision

Use Tauri 2 as the desktop host, React with strict TypeScript as the renderer, and Rust workspace crates for authoritative core behavior. Use Vite for frontend development and builds, npm workspaces for TypeScript packages, and Cargo workspaces for Rust. Keep IPC DTOs in an application contract crate; generate their TypeScript declarations from Rust with ts-rs. Commands remain thin adapters. Core crates must not depend on Tauri or React.

Monaco is the planned text editor. React Flow is reserved for appropriate graph editors. LAD will use a dedicated rung/grid document model. None of these editors is installed in Sprint 0.

## Alternatives

Electron would change the approved desktop stack. A TypeScript runtime would move authoritative semantics into the renderer. Multiple standalone repositories would make boundary checks and lockstep contract changes harder. These options are rejected.

## Consequences and migration

Native platform prerequisites are necessary. Commit npm and Cargo lockfiles and pin the Rust toolchain. Browser preview may display the shell, but must report that native IPC is unavailable. There is no existing application to migrate.

## Sprint boundary

Implement the host, shell, diagnostics definitions, and read-only core information command only. No PLC execution is authorized.
