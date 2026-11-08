// Local check of the native development session that TEST_PLC_LAB.bat starts: the real
// Tauri window and Rust backend (no browser), started from another working directory.
// It verifies the Polish interface, a Free Lab project whose ST program uses VAR RETAIN and
// FIRST_SCAN, PLC RUN with a running simulation, online values from the backend, the
// alarm view, warm and cold restarts with their diagnostics, switching between the light
// and dark themes without touching the program, and a clean close. Data and
// WebView2 state stay in artifacts/dev-session-check. Run: node tests/e2e/dev-session-check.mjs
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { mkdir, readFile, rm } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { chromium } from "playwright-core";
import { root } from "../../scripts/toolchain.mjs";

if (process.platform !== "win32")
  throw new Error("The development session check requires Windows.");
const port = 9339;
const stamp = Date.now();
const directory = path.join(root, "artifacts", "dev-session-check");
await mkdir(directory, { recursive: true });
const PROGRAM = `PROGRAM Main
VAR_INPUT Button1 : BOOL; Button2 : BOOL; AutoMode : BOOL; PartPresent : BOOL; END_VAR
VAR_OUTPUT Lamp1 : BOOL; Lamp2 : BOOL; Lamp3 : BOOL; END_VAR
VAR RETAIN Count : INT := 0; END_VAR
VAR Scans : INT; Edge : R_TRIG; END_VAR
Edge(CLK := Button1);
IF Edge.Q THEN Count := Count + 1; END_IF;
IF FIRST_SCAN THEN Scans := 0; END_IF;
Scans := Scans + 1;
Lamp1 := Count > 0;
Lamp2 := FALSE;
Lamp3 := FALSE;
END_PROGRAM
`;

// `--exe <path>` runs the same checks against an already built development executable
// instead of TEST_PLC_LAB.bat (which builds and starts it), for example when Windows
// refuses a freshly relinked build but allows the one the native smoke ran.
const exeIndex = process.argv.indexOf("--exe");
const exe =
  exeIndex > 0 && process.argv[exeIndex + 1]
    ? path.resolve(root, process.argv[exeIndex + 1])
    : null;
const bat = path.join(root, "TEST_PLC_LAB.bat");
const environment = {
  ...process.env,
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
  WEBVIEW2_USER_DATA_FOLDER: path.join(directory, `webview-${stamp}`),
  PLC_LAB_DATA_DIR: path.join(directory, `data-${stamp}`),
  PLC_LAB_NO_PAUSE: "1",
};
const launcher = exe ? path.basename(exe) : "TEST_PLC_LAB.bat";
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
      // cmd /s /c needs the quoted path exactly as written.
      windowsVerbatimArguments: true,
    });
let output = "";
for (const stream of [session.stdout, session.stderr])
  stream.on("data", (chunk) => {
    output += chunk;
    process.stdout.write(chunk);
  });
let exited = null;
session.on("exit", (code) => {
  exited = code ?? 1;
});
const passed = [];
async function check(name, run) {
  await run();
  passed.push(name);
  console.log(`CHECK PASS: ${name}`);
}
async function waitText(locator, pattern, timeout = 15_000) {
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
    // The first start builds the development application; the window follows.
    const deadline = Date.now() + 15 * 60_000;
    for (;;) {
      if (exited !== null)
        throw new Error(`${launcher} ended (exit ${exited}) first`);
      try {
        const r = await fetch(`http://127.0.0.1:${port}/json/version`, {
          signal: AbortSignal.timeout(1000),
        });
        if (r.ok) break;
      } catch {
        /* the window is not there yet */
      }
      if (Date.now() > deadline)
        throw new Error("No native window within 15 minutes");
      await delay(500);
    }
  });
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  let page;
  for (let i = 0; i < 100 && !page; i++) {
    page = browser.contexts().flatMap((c) => c.pages())[0];
    if (!page) await delay(100);
  }
  assert.ok(page, "the native window has no WebView2 page");
  // In development the window first loads the Vite dev server, which may reload once
  // while it prepares modules: talk to the backend only once the shell has rendered.
  await page.getByRole("navigation").first().waitFor({ timeout: 120_000 });
  await page.waitForLoadState("load");
  const invoke = (command, request) =>
    page.evaluate(
      async ({ command, request }) => {
        try {
          return {
            ok: await window.__TAURI_INTERNALS__.invoke(command, { request }),
          };
        } catch (error) {
          return { error };
        }
      },
      { command, request },
    );

  await check("Rust backend answers over Tauri IPC", async () => {
    const info = await invoke("core_info", { contractVersion: 8 });
    assert.equal(info.ok?.contractVersion, 8, JSON.stringify(info));
    const knowledge = await invoke("knowledge_status", { contractVersion: 8 });
    assert.ok(knowledge.ok, JSON.stringify(knowledge.error));
  });

  await check("ST compiler accepts VAR RETAIN and FIRST_SCAN", async () => {
    const compiled = await invoke("compile", {
      contractVersion: 8,
      source: PROGRAM,
    });
    assert.ok(compiled.ok?.success, JSON.stringify(compiled));
    const count = compiled.ok.declarations.find((d) => d.name === "Count");
    assert.equal(count?.retained, true);
  });

  await check("Polish interface", async () => {
    await page.getByRole("button", { name: /^(Settings|Ustawienia)$/ }).click();
    await page
      .getByRole("combobox", { name: /Language|Język/ })
      .selectOption("pl");
    await page.getByRole("button", { name: "Zamknij", exact: true }).click();
    await page.getByRole("button", { name: "START", exact: true }).waitFor();
    await page.screenshot({ path: path.join(directory, "home-pl.png") });
    await page.getByRole("button", { name: "Ustawienia", exact: true }).click();
    await page.getByRole("combobox", { name: "Język" }).selectOption("en");
    await page.getByRole("button", { name: "Close", exact: true }).click();
  });

  await check(
    "Free Lab project with the retentive program in RUN",
    async () => {
      await page.getByRole("button", { name: "FREE LAB", exact: true }).click();
      await page.getByRole("button", { name: "New project" }).click();
      await page.getByLabel("Machine (scene)").selectOption("scene.io-basics");
      await page.getByRole("button", { name: "Create and open" }).click();
      await page
        .locator('[data-testid="st-editor"] .monaco-editor .view-lines')
        .click({ timeout: 60_000 });
      await page.keyboard.press("Control+A");
      await page.keyboard.press("Delete");
      await page.keyboard.insertText(PROGRAM);
      await page
        .getByRole("toolbar")
        .getByRole("button", { name: "Build & Load to PLC" })
        .click();
      await page
        .getByText("Sim running", { exact: true })
        .waitFor({ timeout: 30_000 });
    },
  );

  const detail = page.getByLabel("Online variable");
  const committed = detail.locator('dt:text-is("Committed") + dd');
  await check(
    "simulation advances and Watch shows backend values",
    async () => {
      await page.getByRole("button", { name: "ADVANCED", exact: true }).click();
      const monitoring = page.getByRole("tablist", { name: "Monitoring" });
      await monitoring.getByRole("tab", { name: "Variables" }).click();
      await page
        .getByRole("row")
        .filter({ hasText: "Scans" })
        .getByRole("button", { name: "Scans" })
        .click();
      const first = Number(await waitText(committed, /^\d+$/));
      const later = Number(await waitText(committed, /^\d+$/));
      await delay(1500);
      const after = Number(await waitText(committed, /^\d+$/));
      assert.ok(after > first || after > later, `Scans ${first} -> ${after}`);
      await monitoring.getByRole("tab", { name: /^Watch/ }).click();
      await page.getByRole("button", { name: "New table" }).click();
      await page.getByLabel("Find a signal").fill("Count");
      await page.getByRole("button", { name: /^Add Count/ }).click();
      await page
        .getByRole("table", { name: /^Watch table / })
        .getByText("Count")
        .first()
        .waitFor();
    },
  );

  await check("a part counted into the retentive variable", async () => {
    const views = page.getByRole("tablist", { name: "Workspace views" });
    await views.getByRole("tab", { name: "Process" }).click();
    const button = page.getByRole("button", {
      name: "Pushbutton S1 (hold to press)",
    });
    await button.dispatchEvent("pointerdown", { pointerId: 1 });
    await delay(500);
    await button.dispatchEvent("pointerup", { pointerId: 1 });
    const monitoring = page.getByRole("tablist", { name: "Monitoring" });
    await monitoring.getByRole("tab", { name: "Variables" }).click();
    await page
      .getByRole("row")
      .filter({ hasText: "Count" })
      .getByRole("button", { name: "Count" })
      .click();
    await waitText(committed, /^1$/);
    await detail.getByText("retentive (VAR RETAIN)").waitFor();
  });

  await check("alarm view is available", async () => {
    const views = page.getByRole("tablist", { name: "Workspace views" });
    await views.getByRole("tab", { name: /^Alarms/ }).click();
    await page
      .getByRole("heading", { name: "Active alarms" })
      .first()
      .waitFor();
  });

  const restart = async (kind) => {
    await page
      .getByRole("toolbar")
      .getByRole("button", { name: `${kind} restart`, exact: true })
      .click();
    const dialog = page.getByRole("dialog", {
      name: `${kind} restart the PLC?`,
    });
    await dialog
      .getByRole("button", { name: `${kind} restart`, exact: true })
      .click();
    await waitText(
      page.getByRole("status", { name: "PLC start state" }),
      new RegExp(`Last start: ${kind.toLowerCase()}`),
    );
  };
  await check("warm restart keeps the retentive count", async () => {
    await restart("Warm");
    await waitText(committed, /^1$/);
    await detail.getByText("value kept").waitFor();
  });
  await check("cold restart initializes it", async () => {
    await restart("Cold");
    await waitText(committed, /^0$/);
  });
  await check("diagnostic buffer records the starts", async () => {
    const views = page.getByRole("tablist", { name: "Workspace views" });
    await views.getByRole("tab", { name: /^Diagnostics/ }).click();
    const buffer = page.getByRole("table", { name: "Diagnostic buffer" });
    await buffer
      .getByText(/^Warm restart: /)
      .first()
      .waitFor();
    await buffer
      .getByText(/^Cold restart: /)
      .first()
      .waitFor();
    await buffer
      .getByText(/^First program scan after the cold start/)
      .first()
      .waitFor();
    await page.screenshot({ path: path.join(directory, "restarts.png") });
  });

  await check(
    "light and dark themes switch without touching the program",
    async () => {
      const lines = page.locator('[data-testid="st-editor"] .view-lines');
      const views = page.getByRole("tablist", { name: "Workspace views" });
      await views.getByRole("tab", { name: "Program" }).click();
      // Monaco lays out the program after the tab opens: compare a rendered program.
      // It renders spaces as non-breaking spaces, which \s matches.
      const before = await waitText(
        lines,
        /VAR\sRETAIN\sCount[\s\S]*END_PROGRAM/,
      );
      const theme = async (value, background, monaco) => {
        await page
          .getByRole("button", { name: "Settings", exact: true })
          .click();
        await page.getByRole("combobox", { name: "Theme" }).selectOption(value);
        await page.getByRole("button", { name: "Close", exact: true }).click();
        assert.equal(
          await page.evaluate(() => document.documentElement.dataset.theme),
          value,
        );
        assert.equal(
          await page.evaluate(
            () => getComputedStyle(document.body).backgroundColor,
          ),
          background,
        );
        await page.locator(`.monaco-editor.${monaco}`).first().waitFor();
        assert.equal(
          (await lines.innerText()).trim(),
          before,
          "program unchanged",
        );
      };
      await theme("light", "rgb(238, 241, 244)", "vs");
      await page.screenshot({
        path: path.join(directory, "workspace-light.png"),
      });
      await theme("dark", "rgb(14, 19, 23)", "vs-dark");
      await page.getByText("Sim running", { exact: true }).waitFor();
    },
  );

  await check("restart controls in Polish", async () => {
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("pl");
    await page.getByRole("button", { name: "Zamknij", exact: true }).click();
    await page
      .getByRole("toolbar")
      .getByRole("button", { name: "Restart ciepły", exact: true })
      .waitFor();
    await page.screenshot({ path: path.join(directory, "toolbar-pl.png") });
  });

  await check(
    "the window closes and the development session ends",
    async () => {
      await browser.close().catch(() => undefined);
      browser = undefined;
      // Without /F, taskkill posts WM_CLOSE: the same request as the close button.
      spawnSync("taskkill", ["/IM", "plc-lab-desktop.exe"], {
        stdio: "ignore",
      });
      const deadline = Date.now() + 60_000;
      while (exited === null && Date.now() < deadline) await delay(250);
      assert.equal(exited, 0, `${launcher} did not end cleanly`);
      if (exe) return;
      assert.match(output, /\[DEV\] Application closed successfully/);
      const log = await readFile(
        path.join(root, "artifacts/test-plc-lab/last-run.log"),
        "utf8",
      );
      assert.match(log, /Running|Finished|Compiling|VITE/i);
    },
  );
  console.log(`DEVELOPMENT SESSION CHECK PASS (${passed.length} checks)`);
} catch (error) {
  console.error(`DEVELOPMENT SESSION CHECK FAILED after: ${passed.join(", ")}`);
  console.error(error);
  process.exitCode = 1;
} finally {
  await browser?.close().catch(() => undefined);
  if (exited === null) {
    spawnSync("taskkill", ["/IM", "plc-lab-desktop.exe"], { stdio: "ignore" });
    await delay(3000);
    if (exited === null)
      spawnSync("taskkill", ["/T", "/F", "/PID", String(session.pid)], {
        stdio: "ignore",
      });
  }
  // A passing check leaves no browser profile or data behind; a failing one keeps both.
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
