import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { chromium } from "playwright-core";
import { root } from "../../scripts/toolchain.mjs";
import { interactiveRuntimeSmoke } from "./runtime-smoke.mjs";
import {
  CONTRACT_VERSION,
  CORE_INFO_COMMAND,
  COMPILE_ST_COMMAND,
} from "../../packages/ipc/src/generated.ts";

if (process.platform !== "win32")
  throw new Error(
    "Native smoke test currently requires Windows/WebView2. See TEST_STRATEGY.md.",
  );
const directory = path.join(root, "artifacts", "desktop-smoke");
await mkdir(directory, { recursive: true });
await writeFile(path.join(directory, "result.json"), '{"result":"RUNNING"}\n');
const listener = net.createServer();
await new Promise((resolve, reject) => {
  listener.once("error", reject);
  listener.listen(0, "127.0.0.1", resolve);
});
const address = listener.address();
assert.ok(address && typeof address === "object");
const port = address.port;
await new Promise((resolve, reject) =>
  listener.close((error) => (error ? reject(error) : resolve())),
);

let output = "";
let application;
try {
  application = spawn(path.join(root, "target/debug/plc-lab-desktop.exe"), [], {
    cwd: root,
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
      WEBVIEW2_USER_DATA_FOLDER: path.join(directory, `webview-${Date.now()}`),
    },
    windowsHide: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
} catch (error) {
  const failure = {
    result: "FAIL",
    phase: "native-launch",
    applicationStarted: false,
    error: error instanceof Error ? error.message : String(error),
    code: error.code ?? null,
  };
  await writeFile(
    path.join(directory, "result.json"),
    `${JSON.stringify(failure, null, 2)}\n`,
  );
  await writeFile(path.join(directory, "native.log"), failure.error);
  throw error;
}
let launchError;
application.on("error", (error) => {
  launchError = error;
});
application.stdout.on("data", (chunk) => {
  output += chunk.toString();
});
application.stderr.on("data", (chunk) => {
  output += chunk.toString();
});
let browser;
try {
  const endpoint = `http://127.0.0.1:${port}`;
  const deadline = Date.now() + 60_000;
  let ready = false;
  let readinessFailure = "WebView2 has not exposed its debugging endpoint.";
  while (Date.now() < deadline) {
    if (launchError) throw launchError;
    if (application.exitCode !== null)
      throw new Error(`Native host exited ${application.exitCode}: ${output}`);
    try {
      const response = await fetch(`${endpoint}/json/version`, {
        signal: AbortSignal.timeout(1000),
      });
      if (response.ok) {
        ready = true;
        break;
      }
      readinessFailure = `Debugging endpoint returned HTTP ${response.status}.`;
    } catch (error) {
      if (!(error instanceof Error)) throw error;
      const code = error.cause?.code;
      if (
        !["ECONNREFUSED", "ECONNRESET"].includes(code) &&
        !["TimeoutError", "AbortError"].includes(error.name)
      )
        throw error;
      readinessFailure = `${error.message} (${code ?? error.name})`;
    }
    await delay(250);
  }
  assert.ok(
    ready,
    `WebView2 failed to become ready: ${readinessFailure}\n${output}`,
  );
  browser = await chromium.connectOverCDP(endpoint);
  const page = browser.contexts()[0]?.pages()[0];
  assert.ok(page, "Native app has no WebView2 page.");
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page
    .getByTestId("core-status")
    .getByText("CORE CONNECTED")
    .waitFor({ timeout: 20_000 });
  assert.equal(await page.title(), "PLC Lab");
  for (const name of [
    "Project",
    "Program",
    "Variables",
    "Process",
    "Network",
    "Timeline",
    "Diagnostics",
  ]) {
    assert.equal(await page.getByRole("region", { name }).count(), 1);
  }
  for (const name of ["Run", "Pause", "Stop", "Single Scan"]) {
    assert.equal(
      await page.getByRole("button", { name, exact: true }).isDisabled(),
      true,
    );
  }
  const info = await page.evaluate(
    ({ command, version }) =>
      window.__TAURI_INTERNALS__.invoke(command, {
        request: { contractVersion: version },
      }),
    { command: CORE_INFO_COMMAND, version: CONTRACT_VERSION },
  );
  const manifest = JSON.parse(
    await readFile(path.join(root, "package.json"), "utf8"),
  );
  assert.equal(info.appVersion, manifest.version);
  assert.equal(info.coreVersion, manifest.version);
  assert.equal(info.contractVersion, CONTRACT_VERSION);
  assert.equal(info.buildContext, "development");
  assert.equal(info.execution, "manual-single-scan");
  const rejection = await page.evaluate(
    async ({ command, version }) => {
      try {
        await window.__TAURI_INTERNALS__.invoke(command, {
          request: { contractVersion: version + 1 },
        });
        return null;
      } catch (error) {
        return error;
      }
    },
    { command: CORE_INFO_COMMAND, version: CONTRACT_VERSION },
  );
  assert.equal(rejection.code, "APP-IPC-001");
  const runRejection = await page.evaluate(async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke("run");
      return null;
    } catch (error) {
      return String(error);
    }
  });
  assert.ok(runRejection, "Unexpected execution command exposed.");
  // Actual Monaco editing and actual Rust compiler IPC, with no transport mock.
  await page
    .locator('[data-testid="st-editor"] .monaco-editor')
    .waitFor({ timeout: 30_000 });
  const compileButton = page.getByRole("button", {
    name: "Compile",
    exact: true,
  });
  assert.equal(await compileButton.count(), 1);
  await compileButton.click();
  await page
    .getByTestId("compile-status")
    .getByText("COMPILE SUCCESS", { exact: true })
    .waitFor();
  const declarationPanel = page.getByTestId("compiled-symbols");
  for (const name of ["Start", "Stop", "Motor"])
    assert.equal(
      await declarationPanel.getByText(name, { exact: true }).count(),
      1,
    );
  assert.equal(
    await declarationPanel.getByLabel("Runtime value unavailable").count(),
    3,
  );
  const editSource = async (source) => {
    const input = page.locator('[data-testid="st-editor"] [role="textbox"]');
    await input.focus();
    await input.press("Control+a");
    await page.keyboard.insertText(source);
    await page
      .getByTestId("compile-status")
      .getByText("NOT COMPILED", { exact: true })
      .waitFor();
  };
  const invalidSource =
    "PROGRAM Main\nVAR_OUTPUT Motor : BOOL; END_VAR\n(* ą ż – 😀 *) Motor := Missing;\nEND_PROGRAM";
  await editSource(invalidSource);
  await compileButton.click();
  await page
    .getByTestId("compile-status")
    .getByText("COMPILE FAILED", { exact: true })
    .waitFor();
  await page
    .getByTestId("compiler-diagnostics")
    .getByText(/PLC-ST-006/)
    .waitFor();
  await page
    .locator('[data-testid="st-editor"] .squiggly-error')
    .first()
    .waitFor();
  const markerBounds = await page
    .locator('[data-testid="st-editor"] .squiggly-error')
    .first()
    .boundingBox();
  const wordBounds = await page
    .locator('[data-testid="st-editor"] .view-lines')
    .evaluate((lines) => {
      const walker = document.createTreeWalker(lines, NodeFilter.SHOW_TEXT);
      for (let node = walker.nextNode(); node; node = walker.nextNode()) {
        const index = node.textContent.indexOf("Missing");
        if (index < 0) continue;
        const range = document.createRange();
        range.setStart(node, index);
        range.setEnd(node, index + 7);
        const rect = range.getBoundingClientRect();
        return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
      }
      return null;
    });
  assert.ok(
    markerBounds && wordBounds,
    "Actual error word and marker must be rendered.",
  );
  assert.ok(
    Math.abs(markerBounds.x - wordBounds.x) <= 2,
    "Unicode error marker starts on the wrong character.",
  );
  assert.ok(
    Math.abs(markerBounds.width - wordBounds.width) <= 2,
    "Error marker covers the wrong source range.",
  );
  assert.ok(
    markerBounds.y < wordBounds.y + wordBounds.height &&
      markerBounds.y + markerBounds.height > wordBounds.y,
    "Error marker is on the wrong line.",
  );
  const invalidCompilation = await page.evaluate(
    ({ command, version, source }) =>
      window.__TAURI_INTERNALS__.invoke(command, {
        request: { contractVersion: version, source },
      }),
    {
      command: COMPILE_ST_COMMAND,
      version: CONTRACT_VERSION,
      source: invalidSource,
    },
  );
  assert.equal(invalidCompilation.success, false);
  const compilerDiagnostic = invalidCompilation.diagnostics.find(
    (d) => d.code === "PLC-ST-006",
  );
  assert.ok(compilerDiagnostic);
  assert.equal(compilerDiagnostic.span.start.line, 3);
  const expectedScalarColumn =
    Array.from("(* ą ż – 😀 *) Motor := ").length + 1;
  assert.equal(compilerDiagnostic.span.start.column, expectedScalarColumn);
  const tonSource =
    "PROGRAM Main\nVAR_INPUT Start : BOOL; END_VAR\nVAR_OUTPUT Motor : BOOL; END_VAR\nVAR Delay : TON; END_VAR\nDelay(PT := T#300ms, Q => Motor, IN := Start);\nEND_PROGRAM";
  await editSource(tonSource);
  await compileButton.click();
  await page
    .getByTestId("compile-status")
    .getByText("COMPILE SUCCESS", { exact: true })
    .waitFor();
  assert.equal(
    await page
      .getByTestId("compiled-symbols")
      .getByText("Delay", { exact: true })
      .count(),
    1,
  );
  assert.equal(
    await page
      .getByTestId("compiled-symbols")
      .getByText("FB Instance", { exact: true })
      .count(),
    1,
  );
  assert.equal(
    await page.locator('[data-testid="st-editor"] .squiggly-error').count(),
    0,
  );
  const defaultSource = (
    await readFile(
      path.join(root, "packages/ui/src/useStCompilation.ts"),
      "utf8",
    )
  ).match(/DEFAULT_ST_SOURCE = `([\s\S]*?)`;/)?.[1];
  assert.ok(defaultSource);
  await editSource(defaultSource);
  await compileButton.click();
  await page
    .getByTestId("compile-status")
    .getByText("COMPILE SUCCESS", { exact: true })
    .waitFor();
  for (const name of ["Run", "Pause", "Stop", "Single Scan"])
    assert.equal(
      await page.getByRole("button", { name, exact: true }).isDisabled(),
      true,
    );
  const runtimeSession = await interactiveRuntimeSmoke({
    page,
    editSource,
    compileButton,
    tonSource,
    defaultSource,
    directory,
  });
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth > window.innerWidth,
  );
  assert.equal(overflow, false, "Desktop shell overflows the native window.");
  const diagnosticsBounds = await page
    .getByRole("region", { name: "Diagnostics" })
    .boundingBox();
  assert.ok(diagnosticsBounds);
  assert.ok(
    diagnosticsBounds.y + diagnosticsBounds.height <=
      (await page.evaluate(() => window.innerHeight)),
    "Diagnostics panel is clipped at the default window size.",
  );
  await page.screenshot({ path: path.join(directory, "desktop.png") });
  assert.deepEqual(errors, []);
  const report = {
    result: "PASS",
    applicationStarted: true,
    realIpc: info,
    invalidContractRejected: rejection.code,
    executionControlsDisabled: true,
    panels: 7,
    pageErrors: errors,
    runtimeSession,
    structuredText: {
      editorPresent: true,
      defaultSourceCompiled: true,
      declarationsDisplayed: true,
      invalidSourceDiagnostic: compilerDiagnostic.code,
      unicodeMarkerCorrect: true,
      tonSourceCompiled: true,
      obsoleteMarkersCleared: true,
      runtimeValuesUnavailableBeforeLoad: true,
    },
  };
  await writeFile(
    path.join(directory, "result.json"),
    `${JSON.stringify(report, null, 2)}\n`,
  );
  console.log(JSON.stringify(report, null, 2));
} catch (error) {
  await writeFile(
    path.join(directory, "result.json"),
    `${JSON.stringify({ result: "FAIL", error: error instanceof Error ? error.message : String(error) }, null, 2)}\n`,
  );
  throw error;
} finally {
  try {
    if (browser) await browser.close();
  } finally {
    application.kill();
    await writeFile(path.join(directory, "native.log"), output);
  }
}
