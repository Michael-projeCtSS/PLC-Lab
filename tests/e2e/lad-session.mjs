// Native acceptance of the Ladder Diagram editor (CP9-C/CP9-D, ADR-0050, ADR-0051) in the
// real desktop application: Tauri window, WebView2 and the Rust backend. Real pointer and
// keyboard input; the ladder document is compiled by the Rust LAD frontend into the
// canonical IR and runs in the one PLC runtime; every online value is the runtime's own
// observation of one scan (ladder_state). Nothing is mocked.
//
// A Free Lab project on the motor scene switches to LAD. The self-holding rung is drawn
// from the palette with a deliberate wiring mistake (Stop as a normally closed contact);
// a second rung delays the run lamp with a TON box whose preset is first mistyped. The
// compiler's diagnostic marks the box and selects it; the preset is fixed. Loaded into
// the PLC, the motor does not start and the online view shows why (the Stop contact
// carries no power while S2 is not pressed); the contact type is corrected, the program
// reloaded, the motor runs, the timer counts and the lamp follows. The window is closed,
// the application started again, the project reopened and edited further (keyboard undo
// and redo, reload). Then Polish, the light and dark themes, ST editing with Enter and
// Backspace, no black screen and a clean close.
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

let session;
let output = "";
let exited = null;
/** Starts the application; the second start reuses the same data and WebView folders. */
function start() {
  output = "";
  exited = null;
  session = exe
    ? spawn(exe, [], {
        cwd: path.join(root, "apps"),
        env: environment,
        stdio: ["ignore", "pipe", "pipe"],
      })
    : spawn(
        process.env.ComSpec ?? "cmd.exe",
        ["/d", "/s", "/c", `""${bat}""`],
        {
          cwd: path.join(root, "apps"),
          env: environment,
          stdio: ["ignore", "pipe", "pipe"],
          windowsVerbatimArguments: true,
        },
      );
  for (const stream of [session.stdout, session.stderr])
    stream.on("data", (chunk) => (output += chunk));
  session.on("exit", (code) => (exited = code ?? 1));
}

const passed = [];
let browser;
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
async function waitAttribute(locator, name, pattern, timeout = 20_000) {
  const deadline = Date.now() + timeout;
  let value = null;
  while (Date.now() < deadline) {
    value = await locator.getAttribute(name).catch(() => null);
    if (value !== null && pattern.test(value)) return value;
    await delay(100);
  }
  throw new Error(`Expected ${name} ${pattern}, saw ${JSON.stringify(value)}`);
}

/** Waits for the window, connects over CDP and follows the page's errors. */
async function connect() {
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
  page = undefined;
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
}

/** Closes the window as the close button does and waits for a clean exit. */
async function close() {
  await browser.close().catch(() => undefined);
  browser = undefined;
  // Without /F, taskkill posts WM_CLOSE: the same request as the close button.
  spawnSync("taskkill", ["/IM", "plc-lab-desktop.exe"], { stdio: "ignore" });
  const deadline = Date.now() + 60_000;
  while (exited === null && Date.now() < deadline) await delay(250);
  assert.equal(exited, 0, `${launcher} did not end cleanly`);
  if (!exe) assert.match(output, /\[DEV\] Application closed successfully/);
}

try {
  start();
  await check(`${launcher} opens the native window`, async () => {
    await connect();
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

  const views = () => page.getByRole("tablist", { name: "Workspace views" });
  const ladder = () => page.getByTestId("lad-editor");
  const element = (name) => ladder().getByRole("button", { name }).first();
  const status = () => page.locator(".lad-status");
  const problems = () => page.locator(".compile-problems");
  const tools = () => page.getByRole("toolbar", { name: "Ladder tools" });
  const where = (name) =>
    page
      .getByRole("group", { name: "Insert position" })
      .getByRole("button", { name, exact: true })
      .click();
  const insert = (name) =>
    tools()
      .getByRole("button", { name: `Insert ${name}` })
      .click();
  const contactVariable = (variable) =>
    page
      .getByRole("combobox", { name: "Variable of the contact" })
      .selectOption(variable);
  const coilVariable = (variable) =>
    page
      .getByRole("combobox", { name: "Variable of the coil" })
      .selectOption(variable);
  const typeIn = async (name, text) => {
    const input = page.getByRole("textbox", { name, exact: true });
    await input.fill(text);
    await input.press("Enter");
  };
  const declare = async (name, type, kind) => {
    await page.getByRole("textbox", { name: "New variable name" }).fill(name);
    await page
      .getByRole("combobox", { name: "Type of the new variable" })
      .selectOption(type);
    await page
      .getByRole("combobox", { name: "Kind of the new variable" })
      .selectOption(kind);
    await page.getByRole("button", { name: "Add variable" }).click();
  };
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
  const tab = (name) => views().getByRole("tab", { name, exact: true }).click();
  const buildAndLoad = async () => {
    await page
      .getByRole("toolbar")
      .getByRole("button", { name: "Build & Load to PLC" })
      .click();
    await page.getByText("Sim running").first().waitFor({ timeout: 30_000 });
    await waitText(
      status(),
      /^Online, scan \d+: power flow and values as the PLC computed them in that scan\.$/,
    );
  };
  const timer = (preset) =>
    ladder().getByRole("button", {
      name: new RegExp(`^TON RunDelay: preset ${preset}`),
    });

  await check("a Free Lab program switches to Ladder Diagram", async () => {
    await page.getByRole("button", { name: "FREE LAB", exact: true }).click();
    // An empty project list offers New project twice (header and empty state).
    await page.getByRole("button", { name: "New project" }).first().click();
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
    await ladder().waitFor({ timeout: 20_000 });
    // The scene's signals are declared.
    const variables = page.getByRole("region", {
      name: "Variables",
      exact: true,
    });
    for (const name of ["StartPB", "StopPB", "OverloadOK", "MotorContactor"])
      await variables.getByRole("rowheader", { name, exact: true }).waitFor();
    await waitText(status(), /^Offline: load this program into the PLC/);
  });

  await check(
    "the self-holding rung is drawn from the palette (with a wiring mistake)",
    async () => {
      await page.getByRole("button", { name: "+ Rung" }).click();
      // The backend compiler names what is still missing.
      await waitText(
        problems(),
        /PLC-LAD-003.*Rung 1: an element has no variable\./,
      );
      await element("Normally open contact (variable?)").click();
      await contactVariable("StartPB");
      await where("Parallel");
      await insert("Normally open contact");
      await contactVariable("MotorContactor");
      await element("Parallel block of 2 branches").click();
      await where("After");
      // The mistake: S2 is wired normally closed (TRUE while not pressed), so a
      // normally closed contact on StopPB never passes power.
      await insert("Normally closed contact");
      await contactVariable("StopPB");
      await insert("Normally open contact");
      await contactVariable("OverloadOK");
      await element("Coil (variable?)").click();
      await coilVariable("MotorContactor");
      await problems().waitFor({ state: "detached", timeout: 15_000 });
    },
  );

  await check(
    "a TON box is configured; a mistyped preset is diagnosed on the box",
    async () => {
      await declare("RunTime", "TIME", "local");
      await page.getByRole("button", { name: "+ Rung" }).click();
      await element("Normally open contact (variable?)").click();
      await contactVariable("MotorFeedback");
      await insert("TON on-delay timer");
      await typeIn("Instance name", "RunDelay");
      await typeIn("Preset time PT: literal", "5");
      // The compiler rejects 5 as a TIME and names the box; the box is marked.
      await waitText(
        problems(),
        /PLC-LAD-007.*Rung 2: TON RunDelay\.PT: '5' is not a TIME literal/,
      );
      await waitAttribute(timer("5"), "aria-invalid", /^true$/);
      // Choosing the problem selects the box.
      await element("Coil (variable?)").click();
      await problems()
        .getByRole("button", { name: /PLC-LAD-007/ })
        .click();
      await waitAttribute(timer("5"), "aria-pressed", /^true$/);
      await typeIn("Preset time PT: literal", "T#2s");
      await page
        .getByRole("combobox", { name: "Elapsed time ET to" })
        .selectOption("RunTime");
      await element("Coil (variable?)").click();
      await coilVariable("RunLamp");
      await problems().waitFor({ state: "detached", timeout: 15_000 });
      assert.equal(await timer("T#2s").getAttribute("aria-invalid"), null);
      await page.screenshot({ path: path.join(directory, "rungs.png") });
    },
  );

  await check(
    "online, the ladder shows why the motor does not start",
    async () => {
      await buildAndLoad();
      // The runtime's observation: S2 not pressed reads TRUE, so the normally
      // closed contact is open and the coil gets no power.
      await element("Normally closed contact StopPB: TRUE, no power").waitFor({
        timeout: 10_000,
      });
      await element("Coil MotorContactor: FALSE, no power").waitFor();
      await tab("Process");
      await motor("stopped");
      await press("Start S1");
      await delay(500);
      await motor("stopped");
      await tab("Program");
      await element("Normally closed contact StopPB: TRUE, no power").waitFor();
      await page.screenshot({ path: path.join(directory, "diagnosis.png") });
    },
  );

  await check(
    "the contact is corrected and reloaded: the motor runs, the timer counts, the lamp follows",
    async () => {
      await element(/^Normally closed contact StopPB/).click();
      await page
        .getByRole("combobox", { name: "Contact type" })
        .selectOption("normallyOpen");
      // An edited program is no longer the loaded one: no stale online values.
      await waitText(status(), /^Offline: load this program into the PLC/);
      await buildAndLoad();
      // S2 released reads TRUE: the contact is closed, but no power reaches it yet
      // (neither Start nor the self-holding branch passes power).
      await element("Normally open contact StopPB: TRUE, no power").waitFor({
        timeout: 10_000,
      });
      await tab("Process");
      await press("Start S1");
      await motor("running");
      await tab("Program");
      // The coil holds itself after Start was released.
      await element("Coil MotorContactor: TRUE, power flows").waitFor({
        timeout: 10_000,
      });
      await element("Normally open contact StartPB: FALSE, no power").waitFor();
      await element(
        "Normally open contact MotorContactor: TRUE, power flows",
      ).waitFor();
      await element(
        "Normally open contact StopPB: TRUE, power flows",
      ).waitFor();
      // ET counts up in the runtime until it reaches PT; then Q drives the lamp.
      const seen = new Set();
      const deadline = Date.now() + 15_000;
      for (;;) {
        const label = (await timer("T#2s").getAttribute("aria-label")) ?? "";
        const value = /value (T#[\w]+)/.exec(label)?.[1];
        if (value && value !== "T#0ms" && value !== "T#2s") seen.add(value);
        if (value === "T#2s") break;
        if (Date.now() > deadline)
          throw new Error(`The timer never reached PT: ${label}`);
        await delay(100);
      }
      assert.ok(
        seen.size >= 2,
        `ET should be seen counting, saw ${JSON.stringify([...seen])}`,
      );
      await element("Coil RunLamp: TRUE, power flows").waitFor({
        timeout: 10_000,
      });
      await page.screenshot({ path: path.join(directory, "online.png") });
      await tab("Process");
      await page
        .locator('[aria-label="Run lamp H1: on"]')
        .waitFor({ timeout: 10_000 });
      await press("Stop S2");
      await motor("stopped");
      await page
        .locator('[aria-label="Run lamp H1: off"]')
        .waitFor({ timeout: 10_000 });
      await tab("Program");
    },
  );

  await check(
    "the ladder survives closing and restarting the application",
    async () => {
      // Edits are saved with the project as they are made.
      await delay(1500);
      await close();
      start();
      await connect();
      await page.getByRole("button", { name: "FREE LAB", exact: true }).click();
      await page
        .locator(".list-row")
        .filter({ hasText: "Ladder motor" })
        .getByRole("button", { name: "Open", exact: true })
        .click();
      await ladder().waitFor({ timeout: 60_000 });
      await element(/^Normally open contact StopPB/).waitFor();
      await element(/^Normally open contact OverloadOK/).waitFor();
      await timer("T#2s").waitFor();
      await element(/^Coil RunLamp/).waitFor();
      await page
        .getByRole("region", { name: "Variables", exact: true })
        .getByRole("rowheader", { name: "RunTime", exact: true })
        .waitFor();
    },
  );

  await check(
    "editing resumes: keyboard undo and redo, then a reload",
    async () => {
      await timer("T#2s").click();
      await typeIn("Preset time PT: literal", "T#3s");
      await timer("T#3s").waitFor();
      await timer("T#3s").click();
      await page.keyboard.press("Control+z");
      await timer("T#2s").waitFor();
      await page.keyboard.press("Control+y");
      await timer("T#3s").waitFor();
      await buildAndLoad();
      await element("Normally open contact StopPB: TRUE, no power").waitFor({
        timeout: 10_000,
      });
      await waitAttribute(timer("T#3s"), "aria-label", /no power$/);
    },
  );

  await check("the ladder editor speaks Polish", async () => {
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("pl");
    await page.getByRole("button", { name: "Zamknij", exact: true }).click();
    await page.getByRole("group", { name: "Język programu" }).waitFor();
    await page
      .getByRole("listitem", { name: "Szczebel 1", exact: true })
      .waitFor();
    await element(/^Styk normalnie otwarty StartPB/).waitFor();
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
      await page
        .getByRole("listitem", { name: "Szczebel 1", exact: true })
        .waitFor();
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
  await close();
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
  if (page && !page.isClosed())
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
