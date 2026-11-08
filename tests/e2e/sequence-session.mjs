// Native acceptance of the sequence inspector (CP8-C, ADR-0043) in the real desktop
// application: Tauri window, WebView2 and the Rust backend. Real pointer and keyboard
// input, real PLC program, real process; nothing is mocked.
//
// The practice exercise runs the reference ST program: initial step, refused start,
// filling, HOLD and RESUME, completion and history. The troubleshooting lab (inlet valve
// stuck closed) shows a step timeout, its alarm and the detailed evaluation. Then Polish,
// the light and dark themes, ST editing with Enter and Backspace, no black screen and a
// clean close.
//
// Usage: node tests/e2e/sequence-session.mjs [--exe target/debug/plc-lab-desktop.exe]
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
  throw new Error("The sequence session check requires Windows.");
const directory = path.join(root, "artifacts", "sequence-session");
await mkdir(directory, { recursive: true });
const solution = await readFile(
  path.join(root, "tests/conformance/solutions/ex.tank-sequence.st"),
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
async function attribute(locator, name) {
  return (await locator.getAttribute(name)) ?? "";
}
const percent = (label) => Number(/: ([\d.]+) %/.exec(label)?.[1] ?? NaN);

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
  const sequenceTab = () =>
    inspector.getByRole("tab", { name: "Sequence", exact: true }).click();
  const state = page.locator(".sequence-inspector .badge").first();
  const active = page.getByTestId("sequence-active");
  const explain = page.getByTestId("sequence-explain");
  const button = (label) =>
    page.getByRole("button", { name: `${label} (hold to press)` });
  const press = async (label, ms = 400) => {
    await button(label).dispatchEvent("pointerdown", { pointerId: 1 });
    await delay(ms);
    await button(label).dispatchEvent("pointerup", { pointerId: 1 });
  };
  const tank = page.locator('[aria-label^="Tank T-100: "]');
  const valve = page.locator('[aria-label^="Inlet valve XV-101: "]');

  await check("the sequence exercise opens with the inspector", async () => {
    // Free Lab projects have no documented sequence; the sequence lab is a practice
    // exercise on the tank scene.
    await page.getByRole("button", { name: "FREE LAB", exact: true }).click();
    await page.getByRole("button", { name: "PRACTICE", exact: true }).click();
    await page
      .getByRole("button", {
        name: "Start Batch sequence with permissives, interlocks and HOLD",
      })
      .click();
    await page
      .locator('[data-testid="st-editor"] .monaco-editor')
      .waitFor({ timeout: 60_000 });
    await sequenceTab();
    // No program in the PLC yet: nothing to observe.
    await waitText(state, /^NO STEP VARIABLE$/);
  });

  await check("the real ST program compiles, loads and runs", async () => {
    await page
      .locator('[data-testid="st-editor"] .monaco-editor')
      .click({ position: { x: 300, y: 12 } });
    await page.keyboard.press("Control+A");
    await page.keyboard.insertText(solution);
    await page
      .getByRole("toolbar")
      .getByRole("button", { name: "Build & Load to PLC" })
      .click();
    await page.getByText("Sim running").first().waitFor({ timeout: 30_000 });
    await waitText(state, /^IDLE$/);
    await waitText(active, /^S0 Idle$/);
  });

  await views.getByRole("tab", { name: "Process" }).click();

  await check("an unsatisfied permissive refuses the start", async () => {
    await button("Hold S3").dispatchEvent("pointerdown", { pointerId: 1 });
    await delay(300);
    await press("Start S1");
    await waitText(
      page.getByRole("note"),
      /Not permitted: Hold button released/,
    );
    await button("Hold S3").dispatchEvent("pointerup", { pointerId: 1 });
    await waitText(state, /^IDLE$/);
    await page.screenshot({ path: path.join(directory, "refused.png") });
  });

  await check(
    "Start runs the batch: transitions follow the real tank",
    async () => {
      const before = percent(await attribute(tank, "aria-label"));
      await press("Start S1");
      await waitText(active, /^S1 Open the inlet valve$/, 5000);
      await waitText(active, /^S2 Fill the tank$/, 10_000);
      await waitText(state, /^RUNNING$/);
      await delay(2000);
      const after = percent(await attribute(tank, "aria-label"));
      assert.ok(after > before, `tank level ${before} -> ${after}`);
      assert.match(await attribute(valve, "aria-label"), /: 100 % open/);
    },
  );

  await check(
    "HOLD stops the plant through the program; RESUME continues",
    async () => {
      await press("Hold S3");
      await waitText(state, /^HELD$/);
      const timer = page.getByTestId("sequence-timer");
      await delay(2500);
      assert.match(await attribute(valve, "aria-label"), /: 0 % open/);
      const paused = await timer.innerText();
      await delay(1500);
      assert.equal(await timer.innerText(), paused, "the step timer is paused");
      await waitText(active, /^S2 Fill the tank$/);
      await page.screenshot({ path: path.join(directory, "held.png") });
      await press("Start S1");
      await waitText(state, /^RUNNING$/);
      await waitText(active, /^S2 Fill the tank$/);
    },
  );

  await check("the batch completes and the history records it", async () => {
    await waitText(state, /^COMPLETED$/, 90_000);
    await page.getByRole("button", { name: "ADVANCED", exact: true }).click();
    await sequenceTab();
    const history = page.getByRole("region", { name: "Sequence history" });
    for (const line of [
      /Sequence completed/,
      /Resumed in S2 Fill the tank/,
      /Held in S2 Fill the tank/,
      /Start refused/,
      /S3 Close the inlet valve → S4 Drain the tank: Inlet valve left the open position/,
    ])
      await waitText(history, line);
    await page.screenshot({ path: path.join(directory, "completed.png") });
    await page.getByRole("button", { name: "BASIC", exact: true }).click();
  });

  await check(
    "a stuck inlet valve times out with its alarm and diagnostics",
    async () => {
      await page
        .getByRole("button", { name: "Back", exact: true })
        .first()
        .click();
      await page
        .getByRole("button", { name: "TROUBLESHOOT", exact: true })
        .click();
      await page
        .getByRole("button", {
          name: "Start The batch sequence stalls after Start",
        })
        .click();
      await page
        .locator('[data-testid="st-editor"] .monaco-editor')
        .waitFor({ timeout: 60_000 });
      await page.getByRole("button", { name: "Run", exact: true }).click();
      await page.getByText("Sim running").first().waitFor({ timeout: 30_000 });
      await sequenceTab();
      await waitText(state, /^IDLE$/);
      await views.getByRole("tab", { name: "Process" }).click();
      await press("Start S1");
      await waitText(active, /^S1 Open the inlet valve$/);
      await waitText(state, /^FAULTED$/, 10_000);
      await waitText(
        explain,
        /Fault in S1 Open the inlet valve: step timeout\./,
      );
      await waitText(
        page.getByTestId("sequence-timer"),
        /of 5\.0 s, 0\.0 s left/,
      );
      await page.getByRole("button", { name: "ADVANCED", exact: true }).click();
      await sequenceTab();
      const waiting = page.getByRole("region", { name: "Waiting for" });
      await waitText(
        waiting,
        /InletOpen = TRUE[\s\S]*now FALSE[\s\S]*input image/,
      );
      await waitText(
        page.getByRole("region", { name: "Related alarms" }),
        /Tank batch: step timeout/,
      );
      await views.getByRole("tab", { name: /^Alarms/ }).click();
      await page
        .getByRole("table", { name: "Active alarms" })
        .getByText("step timeout")
        .first()
        .waitFor();
      await page.screenshot({ path: path.join(directory, "timeout.png") });
    },
  );

  await check("the inspector speaks Polish", async () => {
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("pl");
    await page.getByRole("button", { name: "Zamknij", exact: true }).click();
    await page
      .getByRole("tablist", { name: "Inspektor" })
      .getByRole("tab", { name: "Sekwencja", exact: true })
      .click();
    await waitText(state, /^BŁĄD$/);
    await waitText(active, /^S1 Otwarcie zaworu dopływowego$/);
    await waitText(
      page.getByRole("region", { name: "Oczekiwanie na" }),
      /Zawór dopływowy zgłasza otwarcie/,
    );
    await page.screenshot({ path: path.join(directory, "sequence-pl.png") });
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
      await waitText(state, /^BŁĄD$/);
    };
    await theme("light");
    await page.screenshot({ path: path.join(directory, "sequence-light.png") });
    await theme("dark");
  });

  await check("ST editing with Enter and Backspace still works", async () => {
    await page
      .getByRole("tablist", { name: "Widoki laboratorium" })
      .getByRole("tab", { name: "Program", exact: true })
      .click();
    const editor = page.locator('[data-testid="st-editor"] .monaco-editor');
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
  console.log(`SEQUENCE SESSION PASS (${passed.length} checks)`);
} catch (error) {
  console.error(`SEQUENCE SESSION FAILED after: ${passed.join(", ")}`);
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
