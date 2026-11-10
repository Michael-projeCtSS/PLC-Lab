// Native acceptance of the PID tuning screen (CP9-B, ADR-0046, ADR-0047, ADR-0049) in
// the real desktop application: Tauri window, WebView2 and the Rust backend. Real pointer
// and keyboard input, the real PID block in the one PLC runtime, the real heated vessel;
// nothing is mocked and every value read below is the backend's.
//
// The practice exercise "Heated vessel control with a PID loop": the P-only starter
// settles below the setpoint, the trend and the controller state show it, writing TI
// online removes the offset, the open door is compensated, the reference program holds
// MAN and goes to AUTO without a bump, a lowered output limit saturates the controller,
// grading passes, then Polish, the light and dark themes, ST editing with Enter and
// Backspace, no black screen and a clean close.
//
// Usage: node tests/e2e/pid-session.mjs [--exe target/debug/plc-lab-desktop.exe]
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
  throw new Error("The PID session check requires Windows.");
const directory = path.join(root, "artifacts", "pid-session");
await mkdir(directory, { recursive: true });
const solution = await readFile(
  path.join(root, "tests/conformance/solutions/ex.heater-pid.st"),
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
  const build = () =>
    page
      .getByRole("toolbar")
      .getByRole("button", { name: "Build & Load to PLC" })
      .click();
  // A value of the controller state as the backend reports it.
  const state = (name) =>
    page.locator(
      `section[aria-label="Controller state"] td[data-parameter="${name}"]`,
    );
  const number = async (name) =>
    Number.parseFloat((await state(name).innerText()).trim());
  async function waitValue(name, accept, what, timeout = 60_000) {
    const deadline = Date.now() + timeout;
    let value = Number.NaN;
    while (Date.now() < deadline) {
      value = await number(name).catch(() => Number.NaN);
      if (Number.isFinite(value) && accept(value)) return value;
      await delay(250);
    }
    throw new Error(`${name} never ${what}; last ${value}`);
  }
  const parameter = (label) =>
    page
      .locator('section[aria-label="Parameters"] tr')
      .filter({ has: page.getByRole("rowheader", { name: label }) });
  const write = async (label, text) => {
    const row = parameter(label);
    await row.getByRole("textbox").fill(text);
    await row.getByRole("button", { name: "Write" }).click();
  };
  const diagnostics = page.locator('section[aria-label="Diagnostics"]');
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

  await check("the PID exercise opens with its P-only starter", async () => {
    await page.getByRole("button", { name: "FREE LAB", exact: true }).click();
    await page.getByRole("button", { name: "PRACTICE", exact: true }).click();
    await page
      .getByRole("button", {
        name: "Start Heated vessel control with a PID loop",
      })
      .click();
    await editor.waitFor({ timeout: 60_000 });
    assert.match(await editorText(page), /Ti : TIME := T#0s;/);
  });

  await check(
    "the P-only loop runs and the Tuning tab shows it from the backend",
    async () => {
      await build();
      await page.getByText("Sim running").first().waitFor({ timeout: 30_000 });
      await page.getByLabel("Simulation speed").selectOption("1000");
      await views.getByRole("tab", { name: "Tuning" }).click();
      const chart = page.getByRole("img", { name: /^Trend of Loop/ });
      await chart.waitFor({ timeout: 20_000 });
      await waitText(diagnostics, /TI = 0 s: no integral action/);
      // The selector starts in MAN, but the starter does not use it yet.
      await page
        .getByRole("group", { name: "Selector S1" })
        .getByRole("button", { name: "AUTO" })
        .click();
      // P only: the vessel settles below 60 degC, at about 50.5 degC.
      await waitValue("PV", (v) => v > 49 && v < 52, "settled near 50.5");
      await delay(3000);
      const pv = await waitValue("PV", (v) => v > 49 && v < 52, "stayed");
      assert.ok(60 - pv > 7, `offset ${60 - pv}`);
      const error = await number("E");
      assert.ok(Math.abs(error - (60 - pv)) < 1, `E ${error} for PV ${pv}`);
      const points = (await chart.getAttribute("data-points")) ?? "";
      assert.match(points, /^[1-9]\d*,[1-9]\d*,[1-9]\d*$/, points);
      await page.screenshot({ path: path.join(directory, "p-only.png") });
    },
  );

  await check("writing TI online removes the offset", async () => {
    await write("Integral time TI", "20");
    await waitText(
      parameter("Integral time TI").locator('td[data-parameter="TI"]'),
      /^20 s$/,
    );
    await waitValue("PV", (v) => v > 59.3 && v < 60.7, "reached 60 degC");
    await waitText(diagnostics, /AUTO: MV = P \+ I \+ D/);
    assert.doesNotMatch(await diagnostics.innerText(), /TI = 0 s/);
    await page.screenshot({ path: path.join(directory, "pi.png") });
  });

  await check("the loop compensates the open door", async () => {
    await page
      .getByRole("button", { name: "Open the door of Vessel E-1" })
      .click();
    const low = await waitValue("PV", (v) => v < 59, "dropped with the door");
    await waitValue("PV", (v) => v > 59.3 && v < 60.7, "recovered", 90_000);
    // The integral took over the extra heat loss: more output than before.
    const mv = await number("MV");
    assert.ok(mv > 55, `MV ${mv} with the door open (PV fell to ${low})`);
    await page
      .getByRole("button", { name: "Close the door of Vessel E-1" })
      .click();
    await page.screenshot({ path: path.join(directory, "door.png") });
  });

  await check(
    "the reference program holds MAN and goes to AUTO without a bump",
    async () => {
      await views.getByRole("tab", { name: "Program", exact: true }).click();
      await paste(solution);
      await build();
      await page.getByText("Sim running").first().waitFor({ timeout: 30_000 });
      await page.getByLabel("Simulation speed").selectOption("100");
      await views.getByRole("tab", { name: "Tuning" }).click();
      const selector = page.getByRole("group", { name: "Selector S1" });
      await selector.getByRole("button", { name: "MAN" }).click();
      await waitText(diagnostics, /MAN: the output follows MV_MAN \(20\.0\)/);
      await waitValue("MV", (v) => v === 20, "held MV_MAN");
      await delay(2000);
      // Real time: the reading below is taken well within one second of AUTO.
      await selector.getByRole("button", { name: "AUTO" }).click();
      await waitText(diagnostics, /AUTO: MV = P \+ I \+ D/, 5000);
      const after = await number("MV");
      assert.ok(after >= 15 && after < 35, `MV jumped to ${after}`);
    },
  );

  await check("a lowered output limit saturates the controller", async () => {
    await page.getByLabel("Simulation speed").selectOption("1000");
    await write("Output high limit MV_MAX", "30");
    await waitValue("MV", (v) => v === 30, "held the new limit");
    await waitText(
      diagnostics,
      /The output is at its high limit MV_MAX \(30\.0\): the integral does not grow further \(anti-windup\)\./,
    );
    await page.getByText("Output at a limit", { exact: true }).waitFor();
    await write("Output high limit MV_MAX", "100");
    await waitValue("MV", (v) => v < 100 && v > 30, "left the old limit");
  });

  await check("grading passes with the tuned controller", async () => {
    await page.getByRole("button", { name: "Pause" }).click();
    await page.getByRole("button", { name: "Check solution" }).click();
    await inspector.getByRole("tab", { name: "Task", exact: true }).click();
    await result
      .getByText("Passed", { exact: true })
      .waitFor({ timeout: 60_000 });
    await waitText(result, /100\s*\/\s*100/);
  });

  await check("the tuning screen speaks Polish", async () => {
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("pl");
    await page.getByRole("button", { name: "Zamknij", exact: true }).click();
    await page
      .getByRole("tablist", { name: "Widoki laboratorium" })
      .getByRole("tab", { name: "Strojenie" })
      .click();
    await page.getByRole("region", { name: "Stan regulatora" }).waitFor();
    await page.getByRole("img", { name: /^Trend Loop/ }).waitFor();
    await waitText(
      page.locator('section[aria-label="Diagnostyka"]'),
      /AUTO: MV = P \+ I \+ D, z uchybu E/,
    );
    await page
      .getByRole("button", { name: "Otwórz drzwi: Vessel E-1" })
      .or(page.getByRole("button", { name: /^Otwórz drzwi: / }))
      .first()
      .waitFor();
    await page.screenshot({ path: path.join(directory, "tuning-pl.png") });
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
      await page.getByRole("region", { name: "Stan regulatora" }).waitFor();
    };
    await theme("light");
    await page.screenshot({ path: path.join(directory, "tuning-light.png") });
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
  console.log(`PID SESSION PASS (${passed.length} checks)`);
} catch (error) {
  console.error(`PID SESSION FAILED after: ${passed.join(", ")}`);
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
