// WCAG 2.x contrast of the interface's colour tokens in the dark and light themes.
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { root } from "./toolchain.mjs";

const STYLES = path.join(root, "packages/ui/src/styles.css");

/** Token values of a block such as `:root {` or `:root[data-theme="light"] {`. */
export function tokens(css, selector) {
  const start = css.indexOf(`${selector} {`);
  if (start < 0) throw new Error(`no block ${selector}`);
  const body = css.slice(start, css.indexOf("\n}", start));
  return Object.fromEntries(
    [...body.matchAll(/--([\w-]+):\s*([^;]+);/g)].map((m) => [
      m[1],
      m[2].trim(),
    ]),
  );
}

/** The dark theme is `:root`; the light theme overrides it. */
export function themes(css = readFileSync(STYLES, "utf8")) {
  const dark = tokens(css, ":root");
  return {
    dark,
    light: { ...dark, ...tokens(css, ':root[data-theme="light"]') },
  };
}

function rgb(hex) {
  const h = hex.replace("#", "");
  const full =
    h.length === 3 ? [...h].map((c) => c + c).join("") : h.slice(0, 6);
  return [0, 2, 4].map((i) => parseInt(full.slice(i, i + 2), 16) / 255);
}
function luminance(hex) {
  const [r, g, b] = rgb(hex).map((c) =>
    c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4,
  );
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
export function contrast(a, b) {
  const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}

/** [foreground, background, minimum ratio, what it is]: text 4.5, large/UI 3.0. */
export const PAIRS = [
  ...["text-0", "text-1", "text-2", "text-3"].flatMap((t) =>
    ["bg-0", "bg-1", "bg-2", "bg-3"].map((b) => [t, b, 4.5, "text"]),
  ),
  ...[
    "accent",
    "st-active",
    "st-stopped",
    "st-warning",
    "st-fault",
    "st-bad",
    "st-comm",
    "st-auto",
    "st-manual",
    "st-normal",
  ].flatMap((t) => ["bg-1", "bg-2"].map((b) => [t, b, 4.5, "status text"])),
  ["st-running", "ok-tint", 4.5, "running badge"],
  ["st-stopped", "stopped-tint", 4.5, "stopped badge"],
  ["st-warning", "warn-tint", 4.5, "warning badge"],
  ["on-fault", "fault-fill", 4.5, "fault badge"],
  ["st-bad", "bad-tint", 4.5, "bad-quality badge"],
  ["st-comm", "comm-tint", 4.5, "communication badge"],
  ["on-forced", "st-forced", 4.5, "forced badge"],
  ["danger-text", "danger-tint", 4.5, "error notice"],
  ["warn-text", "warn-notice", 4.5, "warning notice"],
  ["ok-text", "ok-notice", 4.5, "success notice"],
  ["on-primary", "primary-bg", 4.5, "primary button"],
  ["event-error", "bg-1", 4.5, "error event"],
  ["event-warning", "bg-1", 4.5, "warning event"],
  ["diagram-text", "bg-0", 4.5, "diagram"],
  ["text-1", "canvas", 4.5, "process and topology text"],
  ["text-2", "canvas", 4.5, "process labels"],
  ["display-text", "display-bg", 4.5, "drive display"],
  ["symbol-stroke", "canvas", 3, "process symbol outline"],
  ["trace-num", "canvas", 3, "numeric trace"],
  ["st-active", "canvas", 3, "boolean trace"],
  ["line-strong", "bg-1", 1.3, "control borders (decorative)"],
];

export function report(css) {
  const all = themes(css);
  return Object.entries(all).flatMap(([theme, t]) =>
    PAIRS.map(([fg, bg, min, what]) => {
      if (!t[fg] || !t[bg])
        throw new Error(`${theme}: missing --${fg} or --${bg}`);
      return { theme, fg, bg, min, what, ratio: contrast(t[fg], t[bg]) };
    }),
  );
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const rows = report(readFileSync(STYLES, "utf8"));
  for (const r of rows)
    console.log(
      `${r.ratio >= r.min ? "PASS" : "FAIL"} ${r.theme.padEnd(5)} ${r.ratio.toFixed(2).padStart(5)} >= ${r.min}  --${r.fg} on --${r.bg} (${r.what})`,
    );
  if (rows.some((r) => r.ratio < r.min)) process.exitCode = 1;
}
