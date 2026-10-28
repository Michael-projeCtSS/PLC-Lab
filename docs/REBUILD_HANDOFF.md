# Rebuild handoff

The product rebuild on branch `claude/product-rebuild-v1` is complete and in release
freeze: only bug fixes, tests, cleanup, documentation and release corrections are in
scope. The authoritative description and evidence is the
[product rebuild report](PRODUCT_REBUILD_REPORT.md); the history is `git log` (pushed to
`origin`, never force-pushed).

## State

| Area                      | State                                                                                   |
| ------------------------- | --------------------------------------------------------------------------------------- |
| Workspace build           | All crates build; clippy clean with `-D warnings`                                       |
| Tests                     | Rust, frontend, architecture, conformance and knowledge checks pass in CI on both lanes |
| Native Windows validation | Hosted Windows CI: debug, release, root launcher, BAT and relocated-folder smoke runs   |
| Local native validation   | BLOCKED on the development machine by Smart App Control (see below)                     |
| Knowledge pack            | Committed pack and verified deterministic ZIP; installed-pack check passes              |

## Local environment limitation

Smart App Control on the development machine blocks some freshly compiled, unsigned
binaries (os error 4551): test executables, build scripts and proc-macro DLLs. It was
never disabled or bypassed. Local release builds and the local root-launcher check cannot
run; some test binaries cannot run locally after relinking. Hosted CI executes all of
them.

## Commands

`npm run validate` (or `RUN_ALL_CHECKS.bat`) runs the local checks;
`npm run knowledge:verify` and `npm run knowledge:install-check` validate the knowledge
archive; `gh run list --branch claude/product-rebuild-v1` lists CI runs. A failing CI
stage leaves `artifacts/ci/<lane>/<stage>.log` in the uploaded evidence.

## Next work

See the [roadmap](ROADMAP.md); signed distribution comes first.
