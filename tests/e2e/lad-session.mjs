// Native acceptance of the Ladder Diagram editor (CP9-C, ADR-0050) in the real desktop
// application: Tauri window, WebView2 and the Rust backend. Real pointer and keyboard
// input; the ladder document is compiled by the Rust LAD frontend into the canonical IR
// and runs in the one PLC runtime; nothing is mocked.
//
// A Free Lab project on the motor scene switches to LAD; the self-holding rung is drawn
// with contacts, a parallel branch and coils; the backend compiler reports what is
// missing; the program loads, runs and drives the real motor; contacts and coils show
// the values the PLC reports; the ladder is saved with the project; then Polish, the
// light and dark themes, ST editing with Enter and Backspace, no black screen and a
// clean close.
//
// Usage: node tests/e2e/lad-session.mjs [--exe target/debug/plc-lab-desktop.exe]
// Without --exe it starts TEST_PLC_LAB.bat, as a user does.
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { mkdir, rm } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { chromium } from "playwright-core";
import { root } from "../../scripts/toolchain.mjs";
import { editorText } from "./monaco-text.mjs";

if (process.platform !== "win32")
  throw new Error("The LAD session check requires Windows.");
const directory = path.join(root, "artifacts", "lad-session");
await mkdir(directory, { recursive: true });
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

  const views = page.getByRole("tablist", { name: "Workspace views" });
  const ladder = page.getByTestId("lad-editor");
  const choose = async (label, variable) => {
    await ladder.getByRole("button", { name: label }).first().click();
    const select = page
      .getByRole("combobox", { name: "Variable of the contact" })
      .or(page.getByRole("combobox", { name: "Variable of the coil" }));
    await select.selectOption(variable);
  };
  const tool = (name) =>
    page.getByRole("toolbar", { name: "Ladder tools" }).getByRole("button", {
      name,
    });
  const press = async (label) => {
    const button = page.getByRole("button", {
      name: `${label} (hold to press)`,
    });
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
  const motor = (state) =>
    page
      .locator(`[aria-label="Conveyor motor M1: ${state}"]`)
      .waitFor({ timeout: 10_000 });
  const problems = page.locator(".compile-problems");

  await check("a Free Lab program switches to Ladder Diagram", async () => {
    await page.getByRole("button", { name: "FREE LAB", exact: true }).click();
    await page.getByRole("button", { name: "New project" }).click();
    await page.getByLabel("Project name").fill("Ladder motor");
    await page
      .getByLabel("Machine (scene)")
      .selectOption("scene.motor-start-stop");
    await page.getByRole("button", { name: "Create and open" }).click();
    const languages = page.getByRole("group", { name: "Program language" });
    await languages.waitFor({ timeout: 60_000 });
    await languages
      .getByRole("button", { name: "Ladder Diagram (LAD)" })
      .click();
    await page
      .getByRole("button", { name: "Replace with a new LAD program" })
      .click();
    await ladder.waitFor({ timeout: 20_000 });
    // The scene's signals are declared.
    const variables = page.getByRole("region", { name: "Variables" });
    for (const name of ["StartPB", "StopPB", "OverloadOK", "MotorContactor"])
      await variables.getByRole("rowheader", { name }).waitFor();
  });

  await check(
    "the self-holding rung is drawn with contacts, a parallel branch and a coil",
    async () => {
      await page.getByRole("button", { name: "+ Rung" }).click();
      // The backend compiler names what is still missing.
      await waitText(
        problems,
        /PLC-LAD-003.*Rung 1: an element has no variable\./,
      );
      await choose("Normally open contact (variable?)", "StartPB");
      await tool("+ NO in parallel").click();
      await page
        .getByRole("combobox", { name: "Variable of the contact" })
        .selectOption("MotorContactor");
      await ladder
        .getByRole("button", { name: "Parallel block of 2 branches" })
        .click();
      await tool("+ NO in series").click();
      await page
        .getByRole("combobox", { name: "Variable of the contact" })
        .selectOption("StopPB");
      await tool("+ NO in series").click();
      await page
        .getByRole("combobox", { name: "Variable of the contact" })
        .selectOption("OverloadOK");
      await choose("Coil (variable?)", "MotorContactor");
      await page.getByRole("button", { name: "+ Rung" }).click();
      await choose("Normally open contact (variable?)", "MotorFeedback");
      await choose("Coil (variable?)", "RunLamp");
      // Everything is declared and connected: the compiler has nothing to report.
      await problems.waitFor({ state: "detached", timeout: 15_000 });
      await page.screenshot({ path: path.join(directory, "rungs.png") });
    },
  );

  await check(
    "the ladder program compiles into the PLC and runs the motor",
    async () => {
      await page
        .getByRole("toolbar")
        .getByRole("button", { name: "Build & Load to PLC" })
        .click();
      await page.getByText("Sim running").first().waitFor({ timeout: 30_000 });
      await waitText(
        page.locator(".lad-status"),
        /^Online: each contact and coil shows/,
      );
      // Online values come from the PLC: Stop is not pressed (NC wiring, TRUE).
      await ladder
        .getByRole("button", { name: "Normally open contact StopPB: TRUE" })
        .waitFor({ timeout: 10_000 });
      await ladder
        .getByRole("button", { name: "Coil MotorContactor: FALSE" })
        .waitFor();
      await views.getByRole("tab", { name: "Process" }).click();
      await motor("stopped");
      await press("Start S1");
      await motor("running");
      await views.getByRole("tab", { name: "Program", exact: true }).click();
      // The coil holds itself after Start was released.
      await ladder
        .getByRole("button", { name: "Coil MotorContactor: TRUE" })
        .waitFor({ timeout: 10_000 });
      await ladder
        .getByRole("button", { name: "Normally open contact StartPB: FALSE" })
        .waitFor();
      await page.screenshot({ path: path.join(directory, "online.png") });
      await views.getByRole("tab", { name: "Process" }).click();
      await press("Stop S2");
      await motor("stopped");
    },
  );

  await check("the ladder program is saved with the project", async () => {
    await delay(1500);
    await page
      .getByRole("button", { name: "Back", exact: true })
      .first()
      .click();
    await page
      .locator(".list-row")
      .filter({ hasText: "Ladder motor" })
      .getByRole("button", { name: "Open", exact: true })
      .click();
    await ladder.waitFor({ timeout: 60_000 });
    await ladder
      .getByRole("button", { name: /^Normally open contact OverloadOK/ })
      .waitFor();
    await ladder.getByRole("button", { name: /^Coil RunLamp/ }).waitFor();
  });

  await check("the ladder editor speaks Polish", async () => {
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("pl");
    await page.getByRole("button", { name: "Zamknij", exact: true }).click();
    await page.getByRole("group", { name: "Język programu" }).waitFor();
    await page.getByRole("region", { name: "Szczebel 1" }).waitFor();
    await ladder
      .getByRole("button", { name: /^Styk normalnie otwarty StartPB/ })
      .waitFor();
    await page.screenshot({ path: path.join(directory, "ladder-pl.png") });
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
      await page.getByRole("region", { name: "Szczebel 1" }).waitFor();
    };
    await theme("light");
    await page.screenshot({ path: path.join(directory, "ladder-light.png") });
    await theme("dark");
  });

  await check("ST editing with Enter and Backspace still works", async () => {
    // A new ST program replaces the ladder; the editor is the ST editor again.
    await page
      .getByRole("group", { name: "Język programu" })
      .getByRole("button", { name: "Tekst strukturalny (ST)" })
      .click();
    await page
      .getByRole("button", { name: "Zastąp nowym programem ST" })
      .click();
    const editor = page.locator('[data-testid="st-editor"] .monaco-editor');
    await editor.waitFor({ timeout: 20_000 });
    const before = await editorText(page);
    assert.match(before, /VAR_INPUT[\s\S]*StartPB : BOOL;/);
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
  console.log(`LAD SESSION PASS (${passed.length} checks)`);
} catch (error) {
  console.error(`LAD SESSION FAILED after: ${passed.join(", ")}`);
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
                text: document.body.innerText.slice(0, 600),
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
