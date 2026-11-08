// Real keyboard editing in the Monaco Structured Text editor of the real desktop
// application (WebView2 + Rust core): Enter, Backspace, Delete, undo/redo, multiline
// paste and a long editing sequence, while compiler diagnostics for the previous text are
// still shown. The interface must stay rendered, no uncaught JavaScript error may occur,
// the text must be exactly what was typed and the program must still compile and run.
//
// Usage: node tests/e2e/editor-keyboard.mjs [--exe target/debug/plc-lab-desktop.exe]
//        [--language en|pl] [--theme dark|light|system] [--mode basic|advanced]
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { mkdir, rm, writeFile } from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { parseArgs } from "node:util";
import { chromium } from "playwright-core";
import { root } from "../../scripts/toolchain.mjs";
import { editorText, wrappedRows } from "./monaco-text.mjs";

const { values } = parseArgs({
  options: {
    exe: { type: "string", default: "target/debug/plc-lab-desktop.exe" },
    language: { type: "string", default: "en" },
    theme: { type: "string", default: "dark" },
    mode: { type: "string", default: "basic" },
  },
});
const directory = path.join(root, "artifacts", "editor-keyboard");
await mkdir(directory, { recursive: true });
const port = await new Promise((resolve) => {
  const server = net.createServer();
  server.listen(0, "127.0.0.1", () => {
    const { port } = server.address();
    server.close(() => resolve(port));
  });
});
const stamp = Date.now();
const application = spawn(path.resolve(root, values.exe), [], {
  cwd: os.tmpdir(),
  env: {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
    WEBVIEW2_USER_DATA_FOLDER: path.join(directory, `webview-${stamp}`),
    PLC_LAB_DATA_DIR: path.join(directory, `data-${stamp}`),
    PLC_LAB_NO_DIALOG: "1",
  },
  stdio: ["ignore", "pipe", "pipe"],
});
let output = "";
application.stdout.on("data", (c) => (output += c));
application.stderr.on("data", (c) => (output += c));

const errors = [];
const results = [];
let browser;
let page;
const step = async (name, run) => {
  const started = Date.now();
  await run();
  assert.deepEqual(errors, [], `${name}: uncaught JavaScript errors`);
  await rendered(name);
  results.push({ name, ms: Date.now() - started });
  console.log(`PASS ${name}`);
};
async function rendered(name) {
  const state = await page.evaluate(() => ({
    children: document.getElementById("root")?.childElementCount ?? 0,
    nav: !!document.querySelector("nav"),
    editor: !!document.querySelector(
      '[data-testid="st-editor"] .monaco-editor',
    ),
  }));
  assert.ok(
    state.children > 0 && state.nav,
    `${name}: the interface disappeared`,
  );
  return state;
}

// Short lines: no line wraps even in a small window, so ArrowDown moves by program lines.
const PROGRAM = [
  "PROGRAM Main",
  "VAR_INPUT",
  "Button1 : BOOL; Button2 : BOOL;",
  "AutoMode : BOOL; PartPresent : BOOL;",
  "END_VAR",
  "VAR_OUTPUT",
  "Lamp1 : BOOL; Lamp2 : BOOL; Lamp3 : BOOL;",
  "END_VAR",
  "Lamp1 := Button1;",
  "Lamp2 := AutoMode;",
  "Lamp3 := Button2 AND PartPresent;",
  "END_PROGRAM",
].join("\n");
const lineOf = (line) => PROGRAM.split("\n").indexOf(line) + 1;
const LAMP1 = lineOf("Lamp1 := Button1;");
const LAMP3 = lineOf("Lamp3 := Button2 AND PartPresent;");
const END = lineOf("END_PROGRAM");

try {
  const deadline = Date.now() + 60_000;
  for (;;) {
    try {
      const r = await fetch(`http://127.0.0.1:${port}/json/version`, {
        signal: AbortSignal.timeout(1000),
      });
      if (r.ok) break;
    } catch {
      /* WebView starts asynchronously. */
    }
    if (application.exitCode !== null)
      throw new Error(`application exited ${application.exitCode}: ${output}`);
    if (Date.now() > deadline) throw new Error("WebView did not start");
    await delay(200);
  }
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  page = browser.contexts().flatMap((c) => c.pages())[0];
  assert.ok(page, "web view page");
  page.on("pageerror", (e) => errors.push(e.stack ?? String(e)));
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(`console.error: ${m.text()}`);
  });
  await page.getByRole("navigation").first().waitFor({ timeout: 60_000 });
  await page.evaluate(
    ({ language, theme }) =>
      localStorage.setItem(
        "plc-lab.interface.v1",
        JSON.stringify({
          version: 1,
          language,
          theme,
          density: "comfortable",
          explanations: true,
        }),
      ),
    { language: values.language, theme: values.theme },
  );
  await page.reload();
  await page.getByRole("navigation").first().waitFor();
  const pl = values.language === "pl";

  const editor = page.locator('[data-testid="st-editor"] .monaco-editor');
  const lines = page.locator('[data-testid="st-editor"] .view-lines');
  // The model's text as Monaco renders it, independent of word wrap.
  const text = () => editorText(page);
  const errorMarker = page.locator('[data-testid="st-editor"] .squiggly-error');
  const goto = async (line, column) => {
    assert.equal(
      await wrappedRows(page),
      0,
      "a wrapped line would make ArrowDown stop inside a program line",
    );
    await lines.click();
    await page.keyboard.press("Control+Home");
    for (let i = 1; i < line; i++) await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Home");
    // Monaco's first Home goes to the first non-blank character; columns from there.
    for (let i = 1; i < column; i++) await page.keyboard.press("ArrowRight");
  };

  await step("Free Lab project with the editor open", async () => {
    await page
      .getByRole("button", {
        name: pl ? "LABORATORIUM" : "FREE LAB",
        exact: true,
      })
      .click();
    await page
      .getByRole("button", { name: pl ? "Nowy projekt" : "New project" })
      .first()
      .click();
    await page
      .getByLabel(pl ? "Maszyna (scena)" : "Machine (scene)")
      .selectOption("scene.io-basics");
    await page
      .getByRole("button", { name: pl ? "Utwórz i otwórz" : "Create and open" })
      .click();
    await editor.waitFor({ timeout: 60_000 });
    if (values.mode === "advanced")
      await page
        .getByRole("button", {
          name: pl ? "ZAAWANSOWANY" : "ADVANCED",
          exact: true,
        })
        .click();
    await lines.click();
    await page.keyboard.press("Control+A");
    await page.keyboard.press("Delete");
    await page.keyboard.insertText(PROGRAM);
    assert.equal(await text(), PROGRAM);
  });

  await step(
    "a long line wraps on screen and Backspace joins the next line to it",
    async () => {
      await goto(END, 1);
      const long = `(* ${"wrapped ".repeat(25).trim()} *)`;
      await page.keyboard.insertText(long);
      await page.keyboard.press("Enter");
      await delay(100);
      assert.ok((await wrappedRows(page)) > 0, "the long line did not wrap");
      assert.equal(
        await text(),
        PROGRAM.replace("END_PROGRAM", `${long}\nEND_PROGRAM`),
      );
      await page.keyboard.press("Backspace");
      await delay(100);
      assert.equal(
        await text(),
        PROGRAM.replace("END_PROGRAM", `${long}END_PROGRAM`),
      );
      await page.keyboard.press("Control+A");
      await page.keyboard.insertText(PROGRAM);
      assert.equal(await text(), PROGRAM);
    },
  );

  // A syntax error on the last statement, then wait for the compiler's marker.
  const broken = PROGRAM.replace(
    "Lamp3 := Button2 AND PartPresent;",
    "Lamp3 := Button2 AND PartPresent AND ;",
  );
  const withError = async () => {
    await lines.click();
    await page.keyboard.press("Control+A");
    await page.keyboard.insertText(broken);
    await errorMarker.first().waitFor({ timeout: 15_000 });
  };

  await step(
    "Backspace at the start of the line with a compiler error",
    async () => {
      await withError();
      await goto(LAMP3, 1);
      await page.keyboard.press("Backspace");
      await delay(100);
      assert.equal(
        await text(),
        broken.replace("Lamp2 := AutoMode;\nLamp3", "Lamp2 := AutoMode;Lamp3"),
      );
    },
  );

  await step(
    "Enter in the middle of a line before a compiler error",
    async () => {
      await withError();
      await goto(LAMP3, 10);
      await page.keyboard.press("Enter");
      await delay(100);
      assert.equal(
        await text(),
        broken.replace(
          "Lamp3 := Button2 AND PartPresent AND ;",
          "Lamp3 := \nButton2 AND PartPresent AND ;",
        ),
      );
    },
  );

  await step(
    "Enter on an empty line and Backspace at a line start",
    async () => {
      await lines.click();
      await page.keyboard.press("Control+A");
      await page.keyboard.insertText(PROGRAM);
      await goto(LAMP1, 1);
      await page.keyboard.press("Enter");
      await page.keyboard.press("ArrowUp");
      await page.keyboard.press("Enter");
      await page.keyboard.press("Backspace");
      await page.keyboard.press("ArrowDown");
      await page.keyboard.press("Backspace");
      await delay(100);
      assert.equal(await text(), PROGRAM);
    },
  );

  await step("Delete, held Backspace and selection delete", async () => {
    await goto(LAMP3, 1);
    await page.keyboard.press("End");
    for (let i = 0; i < 16; i++) await page.keyboard.press("Backspace");
    await page.keyboard.press("Home");
    await page.keyboard.press("Shift+End");
    await page.keyboard.press("Delete");
    await page.keyboard.press("Delete");
    await delay(100);
    assert.equal(
      await text(),
      PROGRAM.replace("Lamp3 := Button2 AND PartPresent;\n", ""),
    );
  });

  await step("undo and redo", async () => {
    // Undo exactly the deletions of the previous step, then redo and undo again.
    let undone = 0;
    while ((await text()) !== PROGRAM && undone < 20) {
      await page.keyboard.press("Control+Z");
      undone++;
      await delay(50);
    }
    assert.equal(await text(), PROGRAM, `after ${undone} undo steps`);
    assert.ok(undone > 0);
    await page.keyboard.press("Control+Y");
    await delay(100);
    assert.notEqual(await text(), PROGRAM, "redo restored a deletion");
    await page.keyboard.press("Control+Z");
    await delay(100);
    assert.equal(await text(), PROGRAM);
  });

  await step("multiline paste with the clipboard", async () => {
    await goto(END, 1);
    const pasted = "Lamp2 := Button1 OR AutoMode;\nLamp1 := Button2;\n";
    const clipboard = await page
      .evaluate(async (value) => {
        await navigator.clipboard.writeText(value);
        return true;
      }, pasted)
      .catch(() => false);
    assert.ok(clipboard, "the WebView clipboard is unavailable");
    await page.keyboard.press("Control+V");
    await delay(150);
    assert.equal(
      await text(),
      PROGRAM.replace("END_PROGRAM", `${pasted}END_PROGRAM`),
    );
  });

  await step(
    "200 rapid edits with compiler diagnostics in between",
    async () => {
      await lines.click();
      await page.keyboard.press("Control+A");
      await page.keyboard.insertText(broken);
      await errorMarker.first().waitFor({ timeout: 15_000 });
      await goto(LAMP3, 1);
      for (let i = 0; i < 50; i++) {
        await page.keyboard.press("Enter");
        await page.keyboard.type("X;");
        await page.keyboard.press("Backspace");
        await page.keyboard.press("Backspace");
        await page.keyboard.press("Backspace");
        if (i % 10 === 0) await delay(400);
      }
      await delay(500);
      assert.equal(await text(), broken);
    },
  );

  await step("the program still compiles and runs", async () => {
    await lines.click();
    await page.keyboard.press("Control+A");
    await page.keyboard.insertText(PROGRAM);
    // Basic mode has the one-click Build & Load that also starts the PLC and simulation.
    if (values.mode === "advanced")
      await page
        .getByRole("button", { name: pl ? "PODSTAWOWY" : "BASIC", exact: true })
        .click();
    assert.equal(await text(), PROGRAM, "the mode switch kept the program");
    await page
      .getByRole("toolbar")
      .getByRole("button", {
        name: pl ? "Kompiluj i wgraj do PLC" : "Build & Load to PLC",
      })
      .click();
    await page
      .getByText(pl ? "Symulacja: działa" : "Sim running", { exact: false })
      .first()
      .waitFor({ timeout: 30_000 });
    assert.equal(await text(), PROGRAM);
  });
  await page.screenshot({
    path: path.join(
      directory,
      `${values.language}-${values.theme}-${values.mode}.png`,
    ),
  });
  await step(
    "the last edit is autosaved and the reopened project shows it",
    async () => {
      // A final edit, then the autosave delay (800 ms) without further typing.
      await goto(END, 1);
      await page.keyboard.type("(* saved *)");
      await page.keyboard.press("Enter");
      const edited = PROGRAM.replace("END_PROGRAM", "(* saved *)\nEND_PROGRAM");
      await delay(2000);
      assert.equal(await text(), edited);
      await page
        .getByRole("button", { name: pl ? "Wstecz" : "Back", exact: true })
        .first()
        .click();
      await page
        .getByRole("button", { name: pl ? "Otwórz" : "Open", exact: true })
        .first()
        .click();
      await editor.waitFor({ timeout: 60_000 });
      const deadline = Date.now() + 10_000;
      while ((await text()) !== edited && Date.now() < deadline)
        await delay(200);
      assert.equal(await text(), edited, "the reopened project lost the edit");
    },
  );
  console.log(`EDITOR KEYBOARD PASS (${results.length} steps)`);
} catch (error) {
  const state = page
    ? await page
        .evaluate(() => ({
          rootChildren: document.getElementById("root")?.childElementCount,
          body: document.body.innerText.slice(0, 200),
        }))
        .catch((e) => ({ unavailable: String(e) }))
    : null;
  await writeFile(
    path.join(directory, "failure.json"),
    `${JSON.stringify({ error: String(error), errors, state, results }, null, 2)}\n`,
  );
  if (page)
    await page
      .screenshot({ path: path.join(directory, "failure.png") })
      .catch(() => undefined);
  console.error(`EDITOR KEYBOARD FAIL after ${results.length} steps`);
  console.error(error);
  console.error(`uncaught errors: ${JSON.stringify(errors, null, 2)}`);
  console.error(`root: ${JSON.stringify(state)}`);
  process.exitCode = 1;
} finally {
  await browser?.close().catch(() => undefined);
  spawnSync("taskkill", ["/PID", String(application.pid)], { stdio: "ignore" });
  await delay(1500);
  if (application.exitCode === null) application.kill();
  if (!process.exitCode)
    for (const folder of [`webview-${stamp}`, `data-${stamp}`])
      for (let attempt = 0; attempt < 20; attempt++) {
        try {
          await rm(path.join(directory, folder), {
            recursive: true,
            force: true,
          });
          break;
        } catch {
          await delay(250);
        }
      }
}
