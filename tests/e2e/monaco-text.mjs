// The Structured Text model's text as Monaco renders it in the real application. Monaco
// draws rows, and only the rows inside its viewport: with word wrap a long line occupies
// several rows, and only the row that starts a line carries its number in the gutter
// (continuation rows have an empty number at the same height). Row positions are absolute
// within the scrolled content, so a program taller than the editor is read by scrolling
// through it with the mouse wheel. Rendered spaces are non-breaking spaces. The window must
// be visible (animation frames run).

const EDITOR = '[data-testid="st-editor"]';

// Monaco removes deleted rows at once but draws inserted rows in its next animation frame;
// two frames later every pending render has run.
async function view(selector) {
  await new Promise((resolve) =>
    requestAnimationFrame(() => requestAnimationFrame(resolve)),
  );
  const root = document.querySelector(selector);
  const editor = root?.querySelector(".monaco-editor");
  const content = root?.querySelector(".lines-content");
  if (!editor || !content) return null;
  const starts = new Set(
    [...root.querySelectorAll(".margin-view-overlays > div")]
      .filter((row) => row.querySelector(".line-numbers")?.textContent.trim())
      .map((row) => parseFloat(row.style.top)),
  );
  const rows = [...root.querySelectorAll(".view-line")].map((row) => ({
    top: parseFloat(row.style.top),
    height: row.getBoundingClientRect().height,
    text: row.textContent.replaceAll(" ", " "),
  }));
  return {
    scrollTop: -parseFloat(content.style.top || "0"),
    height: editor.getBoundingClientRect().height,
    rows: rows.map((row) => ({ ...row, start: starts.has(row.top) })),
  };
}

// Every row of the program: the viewport, or all of it when the program is taller.
async function allRows(page, selector) {
  let shown = await page.evaluate(view, selector);
  if (!shown) return null;
  const seen = new Map(shown.rows.map((row) => [row.top, row]));
  const fits = (v) =>
    v.scrollTop === 0 &&
    v.rows.every((row) => row.top + 2 * row.height <= v.height);
  if (!fits(shown)) {
    // Monaco scrolls one notch (far less than a screen) per wheel event: up to the first
    // row, then down to the last, collecting the rows rendered on the way.
    const box = await page.locator(`${selector} .monaco-editor`).boundingBox();
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    const scroll = async (delta) => {
      const before = shown.scrollTop;
      await page.mouse.wheel(0, delta);
      shown = await page.evaluate(view, selector);
      for (const row of shown.rows) seen.set(row.top, row);
      return shown.scrollTop !== before;
    };
    for (let turn = 0; turn < 2000 && shown.scrollTop > 0; turn++)
      if (!(await scroll(-box.height / 2))) break;
    for (let turn = 0; turn < 2000; turn++)
      if (!(await scroll(box.height / 2))) break;
  }
  return [...seen.values()].sort((a, b) => a.top - b.top);
}

/** The editor's text, one entry per model line, independent of window size and wrap. */
export async function editorText(page, selector = EDITOR) {
  const rows = await allRows(page, selector);
  if (!rows) return null;
  const lines = [];
  for (const row of rows)
    if (row.start || lines.length === 0) lines.push(row.text);
    else lines[lines.length - 1] += row.text;
  return lines.join("\n");
}

/** Rows that continue a wrapped line (where ArrowDown moves within one model line). */
export async function wrappedRows(page, selector = EDITOR) {
  const rows = (await allRows(page, selector)) ?? [];
  return rows.filter((row, index) => index > 0 && !row.start).length;
}
