// Native acceptance of user functions and function blocks (CP9-A, ADR-0044) in the real
// desktop application: Tauri window, WebView2 and the Rust backend. Real pointer and
// keyboard input, real PLC program, real process; nothing is mocked.
//
// The practice exercise "Reusable toggle function block": the brief names the required
// FUNCTION_BLOCK and FUNCTION, the starter is told which one is missing, the reference
// program compiles into the one PLC runtime, the Library tab shows the program's own
// blocks with their signatures, two instances of one block drive two lamps independently,
// instance memory is visible online, grading passes, then Polish, the light and dark
// themes, ST editing with Enter and Backspace, no black screen and a clean close.
//
// Usage: node tests/e2e/pou-session.mjs [--exe target/debug/plc-lab-desktop.exe]
// Without --exe it starts TEST_PLC_LAB.bat, as a user does.
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { mkdir, readFile, rm } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { chromium } from "playwright-core";
import { root } from "../../scripts/toolchain.mjs";
import { editorText } from "./monaco-text.mjs";

if (process.platform !== "win32")
  throw new Error("The function block session check requires Windows.");
const directory = path.join(root, "artifacts", "pou-session");
await mkdir(directory, { recursive: true });
const solution = await readFile(
  path.join(root, "tests/conformance/solutions/ex.toggle-block.st"),
  "utf8",
);
const port = await new Promise((resolve) => {
  const server = net.createServer();
  server.listen(0, "127.0.0.1", () => {
    const { port } = server.address();
    server.close(() => resolve(port));
  });
});
const exeIndex = process.argv.indexOf("--exe");
const exe =
  exeIndex > 0 && process.argv[exeIndex + 1]
    ? path.resolve(root, process.argv[exeIndex + 1])
    : null;
const stamp = Date.now();
const environment = {
  ...process.env,
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
  WEBVIEW2_USER_DATA_FOLDER: path.join(directory, `webview-${stamp}`),
  PLC_LAB_DATA_DIR: path.join(directory, `data-${stamp}`),
  PLC_LAB_NO_PAUSE: "1",
  PLC_LAB_NO_DIALOG: "1",
};
const launcher = exe ? path.basename(exe) : "TEST_PLC_LAB.bat";
const bat = path.join(root, "TEST_PLC_LAB.bat");
const session = exe
  ? spawn(exe, [], {
      cwd: path.join(root, "apps"),
      env: environment,
      stdio: ["ignore", "pipe", "pipe"],
    })
  : spawn(process.env.ComSpec ?? "cmd.exe", ["/d", "/s", "/c", `""${bat}""`], {
      cwd: path.join(root, "apps"),
      env: environment,
      stdio: ["ignore", "pipe", "pipe"],
      windowsVerbatimArguments: true,
    });
let output = "";
for (const stream of [session.stdout, session.stderr])
  stream.on("data", (chunk) => (output += chunk));
let exited = null;
session.on("exit", (code) => (exited = code ?? 1));

const passed = [];
let page;
const pageErrors = [];
async function check(name, run) {
  await run();
  assert.deepEqual(pageErrors, [], `${name}: uncaught JavaScript errors`);
  const shell = await page.evaluate(() => ({
    children: document.getElementById("root")?.childElementCount ?? 0,
    nav: !!document.querySelector("nav"),
  }));
  assert.ok(shell.children > 0 && shell.nav, `${name}: the window went blank`);
  passed.push(name);
  console.log(`CHECK PASS: ${name}`);
}
async function waitText(locator, pattern, timeout = 20_000) {
  const deadline = Date.now() + timeout;
  let text = "";
  while (Date.now() < deadline) {
    text = (await locator.innerText().catch(() => "")).trim();
    if (pattern.test(text)) return text;
    await delay(200);
  }
  throw new Error(`Expected ${pattern}, saw ${JSON.stringify(text)}`);
}

let browser;
try {
  await check(`${launcher} opens the native window`, async () => {
    const deadline = Date.now() + 15 * 60_000;
    for (;;) {
      if (exited !== null)
        throw new Error(`${launcher} ended (exit ${exited}) first: ${output}`);
      try {
        const r = await fetch(`http://127.0.0.1:${port}/json/version`, {
          signal: AbortSignal.timeout(1000),
        });
        if (r.ok) break;
      } catch {
        /* the window is not there yet */
      }
      if (Date.now() > deadline) throw new Error("No native window");
      await delay(500);
    }
    browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
    for (let i = 0; i < 100 && !page; i++) {
      page = browser.contexts().flatMap((c) => c.pages())[0];
      if (!page) await delay(100);
    }
    assert.ok(page, "no WebView2 page");
    await page.getByRole("navigation").first().waitFor({ timeout: 120_000 });
    await page.waitForLoadState("load");
    page.on("pageerror", (e) => pageErrors.push(e.stack ?? String(e)));
    page.on("console", (m) => {
      if (m.type() === "error") pageErrors.push(`console.error: ${m.text()}`);
    });
    // A known interface language for the labels used below; Polish is checked later.
    await page.evaluate(() =>
      localStorage.setItem(
        "plc-lab.interface.v1",
        JSON.stringify({
          version: 1,
          language: "en",
          theme: "dark",
          density: "comfortable",
          explanations: true,
        }),
      ),
    );
    await page.reload();
    await page.getByRole("navigation").first().waitFor();
  });

  const inspector = page.getByRole("tablist", { name: "Inspector" });
  const views = page.getByRole("tablist", { name: "Workspace views" });
  const editor = page.locator('[data-testid="st-editor"] .monaco-editor');
  const result = page.getByLabel("Check result");
  const lamp = (n, on) =>
    page.locator(`[aria-label="Lamp H${n}: ${on ? "on" : "off"}"]`);
  const press = async (label) => {
    const button = page.getByRole("button", {
      name: `${label} (hold to press)`,
    });
    // Each gesture waits until the backend reports the button's new state, as an
    // operator sees it: a gesture made while another request is in flight is not sent.
    await button.dispatchEvent("pointerdown", { pointerId: 1 });
    await button
      .and(page.locator('[aria-pressed="true"]'))
      .waitFor({ timeout: 10_000 });
    await delay(400);
    await button.dispatchEvent("pointerup", { pointerId: 1 });
    await button
      .and(page.locator('[aria-pressed="false"]'))
      .waitFor({ timeout: 10_000 });
  };
  // Replace the document in the editor by pasting, as a student copies a program in:
  // typed text would be auto-indented line by line, and a typed "(" over a selection
  // surrounds it.
  const paste = async (text) => {
    await editor.click({ position: { x: 300, y: 12 } });
    await page.keyboard.press("Control+A");
    await page.keyboard.press("Delete");
    const clipboard = await page
      .evaluate(async (value) => {
        await navigator.clipboard.writeText(value);
        return true;
      }, text)
      .catch(() => false);
    assert.ok(clipboard, "the WebView clipboard is unavailable");
    await page.keyboard.press("Control+V");
    await delay(300);
    assert.equal(await editorText(page), text);
  };
  const messages = (pattern) =>
    page.locator(".ws-dock").getByText(pattern).count();
  const lamps = async (h1, h2, h3) => {
    for (const [n, on] of [
      [1, h1],
      [2, h2],
      [3, h3],
    ])
      await lamp(n, on).waitFor({ timeout: 10_000 });
  };

  await check(
    "the function block exercise opens with its required blocks",
    async () => {
      await page.getByRole("button", { name: "FREE LAB", exact: true }).click();
      await page.getByRole("button", { name: "PRACTICE", exact: true }).click();
      await page
        .getByRole("button", { name: "Start Reusable toggle function block" })
        .click();
      await editor.waitFor({ timeout: 60_000 });
      await inspector.getByRole("tab", { name: "Task", exact: true }).click();
      await page
        .getByText("Required functions and function blocks")
        .waitFor({ timeout: 10_000 });
      const rows = page.getByTestId("required-pous").getByRole("row");
      assert.deepEqual(await rows.allInnerTexts(), [
        "Toggle\tFUNCTION BLOCK",
        "Both\tFUNCTION",
      ]);
    },
  );

  await check("the starter is told which function is missing", async () => {
    await page.getByRole("button", { name: "Check solution" }).click();
    await waitText(result, /Write FUNCTION Both\./, 30_000);
    assert.doesNotMatch(await result.innerText(), /Write FUNCTION_BLOCK/);
    await page.screenshot({ path: path.join(directory, "starter.png") });
  });

  await check("the program with its own blocks compiles and runs", async () => {
    await paste(solution);
    await page
      .getByRole("toolbar")
      .getByRole("button", { name: "Build & Load to PLC" })
      .click();
    await page.getByText("Sim running").first().waitFor({ timeout: 30_000 });
  });

  await check(
    "the Library tab shows the program's blocks and the standard blocks",
    async () => {
      await inspector
        .getByRole("tab", { name: "Library", exact: true })
        .click();
      const toggle = page.getByTestId("lib-Toggle");
      await waitText(toggle, /Toggle/);
      await toggle.locator("summary").click();
      await waitText(toggle, /Toggle\(Press : BOOL\) => On : BOOL/);
      await waitText(
        toggle,
        /A reusable toggle: each press of Press flips On\./,
      );
      await waitText(toggle, /The pushbutton\./);
      await waitText(toggle, /Toggle1\(Press := PressVar, On => OnOut\);/);
      const both = page.getByTestId("lib-Both");
      await both.locator("summary").click();
      await waitText(both, /Both\(A : BOOL, B : BOOL\) : BOOL/);
      await waitText(both, /TRUE when both inputs are TRUE\./);
      await waitText(both, /Result := Both\(A, B\);/);
      const ton = page.getByTestId("lib-TON");
      await ton.locator("summary").click();
      await waitText(ton, /TON\(IN : BOOL, PT : TIME\) => Q : BOOL, ET : TIME/);
      await page
        .getByRole("searchbox", { name: "Search the library" })
        .fill("bo");
      await waitText(both, /Both/);
      assert.equal(await toggle.count(), 0, "the search hides Toggle");
      await page.screenshot({ path: path.join(directory, "library.png") });
      await page
        .getByRole("searchbox", { name: "Search the library" })
        .fill("");
    },
  );

  await check(
    "two instances of one block drive two lamps independently",
    async () => {
      await views.getByRole("tab", { name: "Process" }).click();
      await lamps(false, false, false);
      await press("Pushbutton S1");
      await lamps(true, false, false);
      await press("Pushbutton S2");
      // Both(Lamp1, Lamp2) is TRUE.
      await lamps(true, true, true);
      await press("Pushbutton S1");
      await lamps(false, true, false);
      await page.screenshot({ path: path.join(directory, "lamps.png") });
    },
  );

  await check("instance memory is visible online", async () => {
    // The starter's check opened Messages; the variables are next to it.
    await page
      .getByRole("tablist", { name: "Monitoring" })
      .getByRole("tab", { name: "Variables" })
      .click();
    await page.getByLabel("Filter variables").fill("Toggle");
    const row = (name) =>
      page.getByRole("row").filter({
        has: page.getByText(name, { exact: true }),
      });
    await waitText(row("Toggle1.On"), /FALSE/);
    await waitText(row("Toggle2.On"), /TRUE/);
    await waitText(row("Toggle2.Press"), /FALSE/);
    await page.getByLabel("Filter variables").fill("");
  });

  await check("grading passes with the blocks", async () => {
    await page.getByRole("button", { name: "Pause" }).click();
    await page.getByRole("button", { name: "Check solution" }).click();
    await inspector.getByRole("tab", { name: "Task", exact: true }).click();
    await result
      .getByText("Passed", { exact: true })
      .waitFor({ timeout: 30_000 });
    await waitText(result, /100\s*\/\s*100/);
  });

  const programAt = solution.indexOf("PROGRAM Main");
  const blocksText = `${solution.slice(0, programAt).trimEnd()}\n`;
  const mainText = solution.slice(programAt);
  const bar = page.getByTestId("document-bar");
  const tree = (name) =>
    page.getByRole("button", { name: `▤ ${name} (ST)`, exact: true });

  await check(
    "the blocks move into a project library document and still run",
    async () => {
      await views.getByRole("tab", { name: "Program", exact: true }).click();
      await page
        .getByRole("button", { name: "+ New library document" })
        .click();
      await waitText(bar, /^Library 1/);
      await bar.getByRole("button", { name: "Rename" }).click();
      await page.getByLabel("Document name").fill("Blocks");
      await bar.getByRole("button", { name: "Save" }).click();
      await waitText(bar, /^Blocks/);
      await paste(blocksText);
      await tree("Main").click();
      await waitText(bar, /^Main/);
      await paste(mainText);
      // The live compile finds the blocks in their document.
      await inspector
        .getByRole("tab", { name: "Library", exact: true })
        .click();
      const toggle = page.getByTestId("lib-Toggle");
      if (!(await toggle.evaluate((e) => e.open)))
        await toggle.locator("summary").click();
      await waitText(toggle, /Line 2 of the library document Blocks/);
      // Build & Load compiles both documents into the PLC.
      await page
        .getByRole("tablist", { name: "Monitoring" })
        .getByRole("tab", { name: /^Messages/ })
        .click();
      const loads = await messages("Program Main loaded into the PLC.");
      await page
        .getByRole("toolbar")
        .getByRole("button", { name: "Build & Load to PLC" })
        .click();
      await page
        .locator(".ws-dock")
        .getByText("Program Main loaded into the PLC.")
        .nth(loads)
        .waitFor({ timeout: 30_000 });
      await page.getByText("Sim running").first().waitFor({ timeout: 30_000 });
      await views.getByRole("tab", { name: "Process" }).click();
      const h1 = page.locator('[aria-label^="Lamp H1: "]');
      const before = await h1.getAttribute("aria-label");
      await press("Pushbutton S1");
      await page
        .locator(
          `[aria-label="Lamp H1: ${before?.endsWith(": on") ? "off" : "on"}"]`,
        )
        .waitFor({ timeout: 10_000 });
      // Grading uses the library too.
      await page.getByRole("button", { name: "Pause" }).click();
      const passes = await messages(/Check: 100 \/ 100 points — passed\./);
      await page.getByRole("button", { name: "Check solution" }).click();
      await page
        .locator(".ws-dock")
        .getByText(/Check: 100 \/ 100 points — passed\./)
        .nth(passes)
        .waitFor({ timeout: 30_000 });
      await views.getByRole("tab", { name: "Program", exact: true }).click();
      await tree("Blocks").click();
      await page.screenshot({ path: path.join(directory, "documents.png") });
    },
  );

  await check("the library document is saved with the project", async () => {
    await delay(1500);
    await page
      .getByRole("button", { name: "Back", exact: true })
      .first()
      .click();
    await page
      .getByRole("button", { name: "Start Reusable toggle function block" })
      .click();
    await editor.waitFor({ timeout: 60_000 });
    await views.getByRole("tab", { name: "Program", exact: true }).click();
    await tree("Blocks").click();
    await waitText(bar, /^Blocks/);
    await delay(300);
    assert.equal(await editorText(page), blocksText);
    await tree("Main").click();
    await waitText(bar, /^Main/);
    await delay(300);
    assert.equal(await editorText(page), mainText);
  });

  await check("the exercise and library speak Polish", async () => {
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("pl");
    await page.getByRole("button", { name: "Zamknij", exact: true }).click();
    const polish = page.getByRole("tablist", { name: "Inspektor" });
    await polish.getByRole("tab", { name: "Zadanie", exact: true }).click();
    await page
      .getByText("Blok funkcyjny przełącznika do wielokrotnego użytku")
      .first()
      .waitFor();
    await page.getByText("Wymagane funkcje i bloki funkcyjne").waitFor();
    assert.deepEqual(
      await page.getByTestId("required-pous").getByRole("row").allInnerTexts(),
      ["Toggle\tBLOK FUNKCYJNY", "Both\tFUNKCJA"],
    );
    await polish.getByRole("tab", { name: "Biblioteka", exact: true }).click();
    await page
      .getByRole("region", { name: "Biblioteka użytkownika (ten program)" })
      .waitFor();
    await waitText(page.getByTestId("lib-Toggle"), /BLOK FUNKCYJNY/);
    await page.getByText("Biblioteka projektu", { exact: true }).waitFor();
    await page.screenshot({ path: path.join(directory, "library-pl.png") });
  });

  await check("light and dark themes", async () => {
    const theme = async (value) => {
      await page
        .getByRole("button", { name: "Ustawienia", exact: true })
        .click();
      await page.getByRole("combobox", { name: "Motyw" }).selectOption(value);
      await page.getByRole("button", { name: "Zamknij", exact: true }).click();
      assert.equal(
        await page.evaluate(() => document.documentElement.dataset.theme),
        value,
      );
      await waitText(page.getByTestId("lib-Both"), /Both/);
    };
    await theme("light");
    await page.screenshot({ path: path.join(directory, "library-light.png") });
    await theme("dark");
  });

  await check("ST editing with Enter and Backspace still works", async () => {
    await page
      .getByRole("tablist", { name: "Widoki laboratorium" })
      .getByRole("tab", { name: "Program", exact: true })
      .click();
    await editor.waitFor();
    const before = await editorText(page);
    await editor.click({ position: { x: 300, y: 12 } });
    await page.keyboard.press("Control+End");
    await page.keyboard.press("Enter");
    await page.keyboard.type("(* checked *)");
    await page.keyboard.press("Enter");
    await delay(300);
    assert.equal(await editorText(page), `${before}\n(* checked *)\n`);
    for (let i = 0; i < "(* checked *)".length + 2; i++)
      await page.keyboard.press("Backspace");
    await delay(300);
    assert.equal(await editorText(page), before);
  });

  await check(
    "no black screen and no uncaught error in the session",
    async () => {
      assert.deepEqual(pageErrors, []);
    },
  );

  // The window is gone afterwards, so this last check does not inspect the page.
  await browser.close().catch(() => undefined);
  browser = undefined;
  // Without /F, taskkill posts WM_CLOSE: the same request as the close button.
  spawnSync("taskkill", ["/IM", "plc-lab-desktop.exe"], { stdio: "ignore" });
  const deadline = Date.now() + 60_000;
  while (exited === null && Date.now() < deadline) await delay(250);
  assert.equal(exited, 0, `${launcher} did not end cleanly`);
  if (!exe) assert.match(output, /\[DEV\] Application closed successfully/);
  passed.push("the window closes cleanly");
  console.log("CHECK PASS: the window closes cleanly");
  console.log(`POU SESSION PASS (${passed.length} checks)`);
} catch (error) {
  console.error(`POU SESSION FAILED after: ${passed.join(", ")}`);
  console.error(error);
  console.error(`uncaught: ${JSON.stringify(pageErrors)}`);
  if (page)
    console.error(
      `page: ${JSON.stringify(
        page.isClosed()
          ? { closed: true }
          : await page
              .evaluate(() => ({
                url: location.href,
                root: document.getElementById("root")?.childElementCount,
                text: document.body.innerText.slice(0, 400),
              }))
              .catch((e) => ({ unavailable: String(e) })),
      )}`,
    );
  if (page)
    await page
      .screenshot({ path: path.join(directory, "failure.png") })
      .catch(() => undefined);
  process.exitCode = 1;
} finally {
  await browser?.close().catch(() => undefined);
  if (exited === null) {
    spawnSync("taskkill", ["/IM", "plc-lab-desktop.exe"], { stdio: "ignore" });
    await delay(1500);
  }
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
