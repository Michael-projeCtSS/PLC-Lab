# ADR-0040: Read-only development browser presentation

Status: Accepted within the authorized local testing recovery task.

## Context

Smart App Control blocks the unsigned native application on the development host.
The existing browser entrypoint rendered native pages that attempted unavailable
Tauri IPC during initialization. Developers need useful UI iteration without
weakening host policy or duplicating Rust execution semantics.

## Decision

Keep the existing Tauri IPC client unchanged. Detect its availability at the UI
presentation boundary. Native sessions delegate library, lesson, article, catalogue
and search reads to that client. Browser development sessions use a typed public
projection of the existing knowledge source through a read-only Vite serve plugin.
The plugin is absent from production builds; the client refuses preview fetching
outside development. No native progress, project or runtime state is fabricated.

Use the actual pages, localization, Settings and Monaco widget. Browser-only workspace
composition provides unsaved text and navigation, with explicit native requirements
for engineering actions. A persistent banner explains the capability boundary.
Skip native open-request initialization entirely when Tauri is absent. The Rust
runtime, IR, IPC schema, native session and process/network models remain unchanged.

Run Vite in the BAT helper's Node process so Q/Enter or Ctrl+C owns shutdown. Bind
127.0.0.1:1420 with strict port selection; identify managed sessions by checkout path
hash, reuse retries without opening another browser, and leave unrelated listeners
alone. Validate Node/npm and locked dependencies without requiring Rust or Tauri.

For real native testing, prepare a verified unsigned distribution in a separate
Sandbox/VM session. Never launch it on this blocked host. Signed production builds
and their verification remain a separate, unchanged workflow.

## Alternatives and consequences

A browser runtime or mocked simulation would create a second semantic authority and
misleading test evidence. A host BAT that starts the blocked EXE does not solve the
policy restriction. Serving only a static landing page would not exercise real UI.

The public DTO projection needs maintenance when presentation schemas change. Existing
validators and real browser tests catch drift. Browser preferences are local to that
browser; projects, progress and editor text do not migrate to native sessions. No
stored data or runtime migration is required.
