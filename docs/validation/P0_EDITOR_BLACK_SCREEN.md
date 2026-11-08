# P0: the window turned black while editing Structured Text (2026-11-08)

## Reproduction

Real desktop application (WebView2 + Rust core), `tests/e2e/editor-keyboard.mjs` before
the fix: Free Lab project, a program with a syntax error on line 6, wait for the compiler
marker, then a real Backspace at the start of line 6.

```
Error: Compiler column is outside the source.
    at toEditorRange      (source-spans.ts)
    at markers / update   (monaco-host.ts, createStEditor().update)
    at StructuredTextEditor's synchronisation effect
    at React commitHookEffects (passive effect)
```

Afterwards the React root had no children and the page body was empty: the black window
the user saw. Ordinary typing did not trigger it; Enter or Backspace that change lines
did.

## Root cause

`StructuredTextEditor` ran `update(source, diagnostics, …)` on every source change. The
diagnostics came from the debounced compile (350 ms) of an **earlier** text. Enter and
Backspace change line numbers and line lengths, so a diagnostic's span could point
outside the new text, and `toEditorRange` (correctly strict) threw. The exception was
raised inside a React effect, and without an error boundary React unmounts the entire
tree. The defect existed since M2A (`90517c9`); it is not related to the theme or the
localization work.

The same effect had a second defect: React's source value can lag the Monaco model by a
keystroke (effects run after paint), and `update()` then called `model.setValue()` with
the older text, which undid the newer keystroke and reset the cursor and undo history.

## Fix

- `editor-sync.ts`: the Monaco model owns the text while editing. A source value the
  editor itself emitted earlier is an echo and never written back; only a value the
  editor never emitted (a program that was opened or replaced) goes into the model.
- `useLab` records the exact text each diagnostics set was compiled for
  (`diagnosticsSource`) and the program the PLC runs (`loadedSource`); the editor applies
  compiler and runtime markers only when that text equals the current text. The next
  compile result follows within a fraction of a second.
- Marker conversion uses `tryEditorRange`; a position that does not fit is skipped and
  recorded, never thrown out of the editor. `focus()` is guarded the same way.
- `ErrorBoundary` (page level inside the shell, and around the whole app) records the
  error, shows a Polish/English error screen with the latest program text and a copy
  button, and offers a fresh start; it never touches projects or the PLC.
- `error-log.ts`: a bounded log (50 entries) of render errors, uncaught exceptions,
  unhandled promise rejections and editor problems; messages and stacks only.

## Evidence

- `tests/e2e/editor-keyboard.mjs` (native, real keyboard): Backspace at the start of an
  error line, Enter before an error, Enter/Backspace at line starts, held Backspace,
  Delete and selection delete, undo/redo, a real clipboard paste (Ctrl+V), 200 rapid edits
  with diagnostics in between, compile and run, and autosave plus reopening the project.
  PASS in English and Polish; dark, light and system theme; Basic and Advanced mode. It
  runs in `npm run test:desktop` locally and in the Windows CI lane.
- `tests/e2e/dev-session-check.mjs` through `TEST_PLC_LAB.bat`: real editing during a RUN
  session and no uncaught JavaScript error during the whole session.
- Unit tests: `editor-sync.test.ts` (echo, external change, bounded history, stale
  positions) and `ErrorBoundary.test.tsx` (error screen, draft, Polish, retry).
