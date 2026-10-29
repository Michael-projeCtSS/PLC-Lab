# ADR-0035: Typed interface localization and presentation preferences

- Status: Accepted for the student UX/localization task
- Date: 2026-10-29

## Decision

Use the English dictionary's stable keys as a TypeScript union. The Polish dictionary
must satisfy the same key set; tests also check matching interpolation parameters.
Missing Polish entries fall back to English at runtime. Components use one context;
they are not duplicated by language. User source, variable/tag names, vendor names,
runtime observations and knowledge-pack content are not rewritten by localization.

Persist only interface preferences in one versioned `plc-lab.interface.v1` record in
the desktop WebView's local storage: PL/EN, dark appearance, comfortable/compact density
and beginner explanations. This is the same presentation-storage facility already used
for Watch column visibility. There is no existing backend application-settings API.
Project/runtime settings remain in Rust. This creates no new IPC, filesystem privilege,
database or dependency. Do not add PLC-semantic settings to this record.

On first use, prefer Polish for a `pl` browser/Windows locale; otherwise use English.
Locale detection requires no asynchronous startup operation. Saved choices override
detection. Invalid/future records fall back to defaults and surface a warning in
Settings; storage failures keep the session usable and explicitly report that saving
failed. The WebView's normal persistent user-data profile retains the record across
application launches. Clearing that profile resets presentation preferences.

Basic Free Lab is a smaller presentation of the same workspace and backend data.
Advanced restores the full toolset; switching mode does not rebuild the PLC program or
change its state. Fault-finding and lesson workspaces retain their engineering views,
and an active fault/force warning is never removed by Basic mode. PLC mode and virtual
simulation time retain their existing, separate commands.

## Alternatives and limits

A localization framework is unnecessary for this bounded UI pass. Per-component
language state or separate Polish components would drift. Adding a Rust preferences
command now would expand the IPC contract for presentation-only data with no existing
settings API to reuse. A future central backend settings service can import this
versioned record explicitly if needed.

Only dark appearance is currently supported. A light/system theme requires verifying
Monaco, process SVGs and all engineering views together; no incomplete theme is exposed.
UI localization is intentionally separate from translating course/knowledge content and
structured backend diagnostics. See [coverage and terminology](../UI_LOCALIZATION.md).
