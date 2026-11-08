// The Structured Text model's text as Monaco renders it in the real application. Monaco
// draws rows: with word wrap a long line occupies several rows, and only the row that
// starts a line carries its number in the gutter (continuation rows have an empty number
// at the same height). Rendered spaces are non-breaking spaces.
const EDITOR = '[data-testid="st-editor"]';

function rows(selector) {
  const root = document.querySelector(selector);
  if (!root) return null;
  const starts = new Set(
    [...root.querySelectorAll(".margin-view-overlays > div")]
      .filter((row) => row.querySelector(".line-numbers")?.textContent.trim())
      .map((row) => parseFloat(row.style.top)),
  );
  return [...root.querySelectorAll(".view-line")]
    .map((row) => ({
      top: parseFloat(row.style.top),
      text: row.textContent.replaceAll(" ", " "),
    }))
    .sort((a, b) => a.top - b.top)
    .map((row) => ({ ...row, start: starts.has(row.top) }));
}

/** The editor's text, one entry per model line, independent of the window width. */
export async function editorText(page, selector = EDITOR) {
  const list = await page.evaluate(rows, selector);
  if (!list) return null;
  const lines = [];
  for (const row of list)
    if (row.start || lines.length === 0) lines.push(row.text);
    else lines[lines.length - 1] += row.text;
  return lines.join("\n");
}

/** Rows that continue a wrapped line (where ArrowDown moves within one model line). */
export async function wrappedRows(page, selector = EDITOR) {
  const list = (await page.evaluate(rows, selector)) ?? [];
  return list.filter((row, index) => index > 0 && !row.start).length;
}
