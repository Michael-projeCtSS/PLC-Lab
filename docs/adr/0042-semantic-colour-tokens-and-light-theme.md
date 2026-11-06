# ADR-0042: Semantic colour tokens, a light theme and a system theme

- Status: Accepted
- Date: 2026-11-06

## Context

The interface was dark only. Besides its base palette it used about sixty hard-coded
colours in the stylesheet and thirty in the process graphics, so a second theme would
have meant editing every rule. Two dark colours (tertiary text, fault text on raised
surfaces) were below WCAG AA contrast.

## Decision

- Every colour is a semantic CSS custom property: surfaces (`--bg-*`), text (`--text-*`),
  lines, status (`--st-*`), badge and notice tints, the process canvas, symbols, pipes,
  lamps, the drive display and the waveform. `:root` holds the dark values;
  `:root[data-theme="light"]` overrides them. Components use tokens through `style`
  properties, also inside SVG, so presentation attributes never carry `var()`.
- The dark values are the previous colours, except `--text-3` and `--st-fault`, which
  are lighter so they meet AA.
- The preference `theme` is `dark`, `light` or `system` in the existing version 1 record;
  saved `dark` records stay valid and the default stays dark. `system` follows the
  operating system's `prefers-color-scheme` and its later changes. The applied theme is
  set on `<html data-theme>` before paint.
- The Monaco editor has a `plc-lab-light` theme; a theme change calls
  `monaco.editor.setTheme`, so the model, its undo history and the selection stay.
- `scripts/theme-contrast.mjs` computes WCAG contrast for 57 token pairs in each theme
  (text 4.5:1, graphics 3:1). Tests require every pair to pass, no hard-coded colour
  outside the two token blocks, a definition for every token used and a light value for
  every base token. The native smoke checks that the light theme survives a restart;
  `tests/e2e/theme-screenshots.mjs` captures both themes at 1366x768, 1920x1080 and
  2560x1440 and reports overflowing or clipped text.

## Alternatives

- A second stylesheet: rejected, it would drift from the first.
- CSS `light-dark()`: rejected for now, the tokens stay readable as two blocks and the
  contrast check can read them directly.

## Consequences

A new colour needs a token with a dark and a light value; the guard test fails
otherwise. Device colours that stand for physical parts (lamp colours, the drive's
display) keep their meaning in both themes.
