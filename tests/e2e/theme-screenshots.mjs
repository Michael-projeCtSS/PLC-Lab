// Screenshots of the real desktop application (WebView2 + Rust core) in the dark and the
// light theme at 1366x768, 1920x1080 and 2560x1440, in Polish: Home, a workspace with the
// program editor, the process view and, in Advanced mode, the hardware and commissioning
// views. The viewport is emulated over the DevTools protocol; layout problems that only
// appear at one size (overflow, clipped text) are reported as failures.
//
// Usage: node tests/e2e/theme-screenshots.mjs [--exe target/debug/plc-lab-desktop.exe]
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

const { values } = parseArgs({
  options: {
    exe: { type: "string", default: "target/debug/plc-lab-desktop.exe" },
  },
});
const SIZES = [
  [1366, 768],
  [1920, 1080],
  [2560, 1440],
];
const THEMES = ["dark", "light"];
const directory = path.join(root, "artifacts", "theme-screenshots");
await mkdir(directory, { recursive: true });

async function freePort() {
  const server = net.createServer();
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address();
  await new Promise((resolve) => server.close(resolve));
  return port;
}
const port = await freePort();
const stamp = Date.now();
const target = path.resolve(root, values.exe);
const application = spawn(target, [], {
  cwd: os.tmpdir(),
  env: {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
    WEBVIEW2_USER_DATA_FOLDER: path.join(directory, `webview-${stamp}`),
    PLC_LAB_DATA_DIR: path.join(directory, `data-${stamp}`),
    PLC_LAB_NO_DIALOG: "1",
  },
  stdio: "ignore",
});
const shots = [];
let browser;
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
    if (Date.now() > deadline) throw new Error("WebView did not start");
    await delay(200);
  }
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  const page = browser.contexts().flatMap((c) => c.pages())[0];
  assert.ok(page, "web view page");
  const cdp = await page.context().newCDPSession(page);
  await page.getByRole("navigation").waitFor({ timeout: 60_000 });

  const prepare = async (theme) => {
    await page.evaluate((theme) => {
      localStorage.setItem(
        "plc-lab.interface.v1",
        JSON.stringify({
          version: 1,
          language: "pl",
          theme,
          density: "comfortable",
          explanations: true,
        }),
      );
    }, theme);
    await page.reload();
    await page.getByRole("button", { name: "START", exact: true }).waitFor();
    assert.equal(
      await page.evaluate(() => document.documentElement.dataset.theme),
      theme,
    );
  };
  /** The page must not scroll sideways and visible text must not be cut off. */
  const layout = () =>
    page.evaluate(() => {
      const problems = [];
      const doc = document.documentElement;
      if (doc.scrollWidth > doc.clientWidth + 1)
        problems.push(
          `page overflows by ${doc.scrollWidth - doc.clientWidth}px`,
        );
      for (const el of document.querySelectorAll(
        "button, h1, h2, h3, h4, label",
      )) {
        const style = getComputedStyle(el);
        if (
          el.offsetParent !== null &&
          style.overflow === "hidden" &&
          style.textOverflow !== "ellipsis" &&
          el.scrollWidth > el.clientWidth + 2
        )
          problems.push(`clipped: ${el.textContent?.trim().slice(0, 40)}`);
      }
      return problems;
    });
  const capture = async (theme, [width, height], name) => {
    await delay(400);
    const file = `${theme}-${width}x${height}-${name}.png`;
    await page.screenshot({ path: path.join(directory, file) });
    shots.push({ theme, width, height, name, file, problems: await layout() });
  };

  for (const theme of THEMES) {
    await prepare(theme);
    for (const size of SIZES) {
      const [width, height] = size;
      await cdp.send("Emulation.setDeviceMetricsOverride", {
        width,
        height,
        deviceScaleFactor: 1,
        mobile: false,
      });
      await page.getByRole("button", { name: "START", exact: true }).click();
      await capture(theme, size, "home");
      await page
        .getByRole("button", { name: "ĆWICZENIA", exact: true })
        .click();
      await page
        .getByRole("button", { name: /^Rozpocznij: / })
        .first()
        .click();
      await page
        .locator('[data-testid="st-editor"] .monaco-editor')
        .waitFor({ timeout: 30_000 });
      await capture(theme, size, "program");
      await page.getByRole("tab", { name: "Proces", exact: true }).click();
      await capture(theme, size, "process");
      await page.getByRole("button", { name: "ZAAWANSOWANY" }).click();
      await page.getByRole("tab", { name: "Sprzęt", exact: true }).click();
      await capture(theme, size, "hardware");
      await page
        .getByRole("tab", { name: "Uruchamianie", exact: true })
        .click();
      await capture(theme, size, "commissioning");
      await page.getByRole("button", { name: "PODSTAWOWY" }).click();
    }
  }
  await cdp.send("Emulation.clearDeviceMetricsOverride");
} finally {
  await browser?.close().catch(() => undefined);
  spawnSync("taskkill", ["/PID", String(application.pid)], { stdio: "ignore" });
  await delay(1500);
  if (application.exitCode === null) application.kill();
}
await writeFile(
  path.join(directory, "screenshots.json"),
  `${JSON.stringify(shots, null, 2)}\n`,
);
// The screenshots stay; the run's browser profile and data do not.
for (const folder of [`webview-${stamp}`, `data-${stamp}`])
  for (let attempt = 0; attempt < 20; attempt++) {
    try {
      await rm(path.join(directory, folder), { recursive: true, force: true });
      break;
    } catch {
      await delay(250);
    }
  }
const problems = shots.filter((s) => s.problems.length);
for (const s of shots)
  console.log(
    `${s.problems.length ? "CHECK" : "OK   "} ${s.file}${s.problems.length ? `: ${s.problems.join("; ")}` : ""}`,
  );
console.log(`${shots.length} screenshots in ${path.relative(root, directory)}`);
if (problems.length) process.exitCode = 1;
