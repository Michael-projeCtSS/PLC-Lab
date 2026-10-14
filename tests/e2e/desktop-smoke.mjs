import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { chromium } from "playwright-core";
import { root } from "../../scripts/toolchain.mjs";
import {
  CONTRACT_VERSION,
  CORE_INFO_COMMAND,
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
const application = spawn(
  path.join(root, "target/debug/plc-lab-desktop.exe"),
  [],
  {
    cwd: root,
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
      WEBVIEW2_USER_DATA_FOLDER: path.join(directory, `webview-${Date.now()}`),
    },
    windowsHide: true,
    stdio: ["ignore", "pipe", "pipe"],
  },
);
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
    assert.equal(await page.getByRole("button", { name }).isDisabled(), true);
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
  assert.equal(info.execution, "unavailable");
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
