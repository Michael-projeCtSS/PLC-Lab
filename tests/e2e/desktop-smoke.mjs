// Native end-to-end smoke test of the real desktop application (WebView2 + Rust core).
// No transport mocks: the test drives the real UI, which calls the real backend.
//
// Usage: node tests/e2e/desktop-smoke.mjs [--exe <path>] [--build development|release]
//                                         [--out <artifacts subfolder>] [--launcher]
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { parseArgs } from "node:util";
import { chromium } from "playwright-core";
import { root } from "../../scripts/toolchain.mjs";

const { values } = parseArgs({
  options: {
    exe: { type: "string", default: "target/debug/plc-lab-desktop.exe" },
    build: { type: "string", default: "development" },
    out: { type: "string", default: "desktop-smoke" },
    // The executable is the root launcher: it starts the staged app and exits.
    launcher: { type: "boolean", default: false },
  },
});
if (process.platform !== "win32")
  throw new Error("The native smoke test requires Windows and WebView2.");

const SOLUTION = `PROGRAM Main
VAR_INPUT
    StartPB : BOOL;
    StopPB : BOOL;
    MotorFeedback : BOOL;
    OverloadOK : BOOL;
END_VAR
VAR_OUTPUT
    MotorContactor : BOOL;
    RunLamp : BOOL;
END_VAR
MotorContactor := (StartPB OR MotorContactor) AND StopPB AND OverloadOK;
RunLamp := MotorFeedback;
END_PROGRAM
`;

const directory = path.join(root, "artifacts", values.out);
await mkdir(directory, { recursive: true });
const steps = [];
const result = (state, extra = {}) =>
  writeFile(
    path.join(directory, "result.json"),
    `${JSON.stringify({ result: state, executable: values.exe, build: values.build, steps, ...extra }, null, 2)}\n`,
  );
await result("RUNNING");

async function freePort() {
  const server = net.createServer();
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const { port } = server.address();
  await new Promise((resolve) => server.close(resolve));
  return port;
}

const port = await freePort();
const stamp = Date.now();
let output = "";
let launchError = null;
const application = spawn(path.resolve(root, values.exe), [], {
  cwd: path.dirname(path.resolve(root, values.exe)),
  env: {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
    WEBVIEW2_USER_DATA_FOLDER: path.join(directory, `webview-${stamp}`),
    PLC_LAB_DATA_DIR: path.join(directory, `data-${stamp}`),
  },
  windowsHide: true,
  stdio: ["ignore", "pipe", "pipe"],
});
application.on("error", (error) => (launchError = error));
application.stdout.on("data", (c) => (output += c));
application.stderr.on("data", (c) => (output += c));

const step = async (name, work) => {
  const started = Date.now();
  await work();
  steps.push({ name, ms: Date.now() - started });
};

let browser;
try {
  const endpoint = `http://127.0.0.1:${port}`;
  await step("native host exposes WebView2", async () => {
    const deadline = Date.now() + 60_000;
    while (Date.now() < deadline) {
      if (launchError) throw launchError;
      if (
        application.exitCode !== null &&
        !(values.launcher && application.exitCode === 0)
      )
        throw new Error(
          `Native host exited ${application.exitCode}: ${output}`,
        );
      try {
        const response = await fetch(`${endpoint}/json/version`, {
          signal: AbortSignal.timeout(1000),
        });
        if (response.ok) return;
      } catch {
        // not ready yet
      }
      await delay(250);
    }
    throw new Error(`WebView2 did not become ready.\n${output}`);
  });
  browser = await chromium.connectOverCDP(endpoint);
  let page;
  await step("web view target exists", async () => {
    const deadline = Date.now() + 20_000;
    while (!page && Date.now() < deadline) {
      page = browser.contexts().flatMap((c) => c.pages())[0];
      if (!page) await delay(100);
    }
    assert.ok(page, "The native application has no WebView2 page.");
  });
  const pageErrors = [];
  page.on("pageerror", (e) => pageErrors.push(e.message));
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

  await step("shell renders with primary navigation", async () => {
    await page
      .getByRole("navigation", { name: "Main" })
      .waitFor({ timeout: 30_000 });
    assert.equal(await page.title(), "PLC Lab");
    for (const name of [
      "HOME",
      "LEARN",
      "PRACTICE",
      "TROUBLESHOOT",
      "FREE LAB",
      "KNOWLEDGE",
      "PROGRESS",
    ])
      assert.equal(
        await page.getByRole("button", { name, exact: true }).count(),
        1,
        name,
      );
    assert.doesNotMatch(
      await page.locator("body").innerText(),
      /M4B|FOUNDATION|IPC v\d/,
    );
  });

  await step("core contract and knowledge pack", async () => {
    const info = await invoke("core_info", { contractVersion: 8 });
    assert.equal(info.ok.contractVersion, 8);
    assert.equal(info.ok.execution, "manual-and-continuous");
    assert.equal(info.ok.buildContext, values.build);
    const stale = await invoke("core_info", { contractVersion: 7 });
    assert.equal(stale.error.code, "APP-CONTRACT-VERSION");
    const unknown = await invoke("tick", { contractVersion: 8 });
    assert.ok(
      unknown.error,
      "no host command may let the web view drive cycles",
    );
    const status = await invoke("knowledge_status", { contractVersion: 8 });
    const pack = status.ok.packs.find((p) => p.id === "plclab.core");
    assert.ok(
      pack,
      `knowledge pack not discovered: ${JSON.stringify(status.ok.searchPaths)}`,
    );
    assert.notEqual(pack.status, "INVALID", pack.errors.join("; "));
    assert.ok(pack.exercises >= 2);
  });

  await step("practice exercise opens a workspace", async () => {
    await page.getByRole("button", { name: "PRACTICE", exact: true }).click();
    await page
      .getByRole("button", { name: /Start Motor start\/stop with seal-in/ })
      .click();
    await page
      .getByRole("button", { name: "Build & Load to PLC" })
      .waitFor({ timeout: 30_000 });
    await page
      .locator('[data-testid="st-editor"] .monaco-editor')
      .waitFor({ timeout: 30_000 });
  });

  await step(
    "edit, build and load with the real compiler and runtime",
    async () => {
      await page
        .locator('[data-testid="st-editor"] .monaco-editor .view-lines')
        .click();
      await page.keyboard.press("Control+A");
      await page.keyboard.press("Delete");
      await page.keyboard.insertText(SOLUTION);
      await page.getByRole("button", { name: "Build & Load to PLC" }).click();
      await page
        .getByText("Sim running", { exact: true })
        .waitFor({ timeout: 15_000 });
      await page.getByText("PLC RUN", { exact: true }).waitFor();
    },
  );

  await step("operator input drives the simulated machine", async () => {
    await page.getByRole("tab", { name: "Process" }).click();
    const button = page.getByRole("button", {
      name: "Start S1 (hold to press)",
    });
    await button.dispatchEvent("pointerdown", { pointerId: 1 });
    await delay(500);
    await button.dispatchEvent("pointerup", { pointerId: 1 });
    await page
      .locator('[aria-label="Conveyor motor M1: running"]')
      .waitFor({ timeout: 10_000 });
    await page.screenshot({
      path: path.join(directory, "workspace-process.png"),
    });
  });

  await step("causal inspector shows the cycle", async () => {
    await page.getByRole("tab", { name: "Cause & effect" }).click();
    await page
      .getByText(/PLC read/)
      .first()
      .waitFor();
    await page
      .getByText(/Next PLC input/)
      .first()
      .waitFor();
  });

  await step("isolated grading passes", async () => {
    await page.getByRole("button", { name: "Pause" }).click();
    await page.getByRole("button", { name: "Check solution" }).click();
    await page.getByRole("tab", { name: "Task" }).click();
    await page
      .getByLabel("Check result")
      .getByText("Passed", { exact: true })
      .waitFor({ timeout: 30_000 });
    await page.getByRole("tab", { name: "Program" }).click();
    await page.screenshot({
      path: path.join(directory, "workspace-program.png"),
    });
  });

  await step(
    "troubleshooting lab opens without revealing the cause",
    async () => {
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page
        .getByRole("button", { name: "TROUBLESHOOT", exact: true })
        .click();
      await page
        .getByRole("button", { name: /Start Conveyor will not start/ })
        .click();
      await page.getByText("Fault report").waitFor({ timeout: 30_000 });
      assert.doesNotMatch(
        await page.locator("body").innerText(),
        /contactTypeMismatch/,
      );
    },
  );

  await step("knowledge search returns results", async () => {
    await page.getByRole("button", { name: "KNOWLEDGE", exact: true }).click();
    await page.getByLabel("Search knowledge").fill("motor");
    await page.locator(".list .list-row").first().waitFor({ timeout: 10_000 });
  });

  assert.deepEqual(pageErrors, [], "the web view reported script errors");
  await page.screenshot({ path: path.join(directory, "final.png") });
  await result("PASS");
  console.log(`Native smoke PASS (${values.exe}, ${steps.length} steps)`);
} catch (error) {
  await result("FAIL", {
    error: error instanceof Error ? error.stack : String(error),
    output,
  });
  throw error;
} finally {
  await browser?.close().catch(() => undefined);
  if (application.exitCode === null) application.kill();
  if (values.launcher)
    spawnSync("taskkill", ["/IM", "PLC Lab Desktop.exe", "/T", "/F"], {
      stdio: "ignore",
    });
  await writeFile(path.join(directory, "native.log"), output);
}
