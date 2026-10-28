// Native smoke test of the root Launcher.exe (Windows): the real launcher window, its
// command-line modes and the hand-over to the real staged application. Nothing is mocked:
// the window is driven with standard window messages (tests/e2e/launcher-window.ps1) and
// the application through its WebView2 DevTools endpoint.
//
// Usage: node tests/e2e/launcher-smoke.mjs [--out <artifacts subfolder>]
//
// Every launch uses fresh copies of the installation in folders whose paths contain spaces
// and starts from another working directory, so nothing may depend on either.
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { parseArgs } from "node:util";
import { chromium } from "playwright-core";
import { listFiles, PACK_NAME } from "../../scripts/knowledge-pack.mjs";
import { root } from "../../scripts/toolchain.mjs";

const { values } = parseArgs({
  options: { out: { type: "string", default: "launcher-window-smoke" } },
});
if (process.platform !== "win32")
  throw new Error("The launcher smoke test requires Windows.");

const APP = "PLC Lab Desktop.exe";
const LAUNCHER = "Launcher.exe";
const SYSTEM32 = path.join(process.env.SystemRoot ?? "C:/Windows", "System32");
const HELPER = path.join(root, "tests", "e2e", "launcher-window.ps1");
const PROJECT = "Launcher hand-over bench";
const CONTRACT = 8;

const version = /\[workspace\.package\][^[]*?\bversion\s*=\s*"([^"]+)"/s.exec(
  await readFile(path.join(root, "Cargo.toml"), "utf8"),
)?.[1];
assert.ok(version, "the Cargo workspace declares the product version");

const directory = path.join(root, "artifacts", values.out);
await mkdir(directory, { recursive: true });
const steps = [];
const result = (state, extra = {}) =>
  writeFile(
    path.join(directory, "result.json"),
    `${JSON.stringify({ result: state, version, steps, ...extra }, null, 2)}\n`,
  );
await result("RUNNING");
const step = async (name, work) => {
  const started = Date.now();
  await work();
  steps.push({ name, ms: Date.now() - started });
};

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

function within(promise, ms, what) {
  let timer;
  const timeout = new Promise((_, reject) => {
    timer = setTimeout(
      () => reject(new Error(`${what} did not finish within ${ms / 1000} s`)),
      ms,
    );
  });
  return Promise.race([promise, timeout]).finally(() => clearTimeout(timer));
}

/** The image subsystem of a PE file: 2 is a Windows GUI program (no console window). */
async function subsystem(file) {
  const bytes = await readFile(file);
  const header = bytes.readUInt32LE(0x3c);
  assert.equal(
    bytes.toString("latin1", header, header + 4),
    "PE\0\0",
    `${file} is not a PE image`,
  );
  return bytes.readUInt16LE(header + 24 + 68);
}

const stamp = Date.now();
const temporary = [];
async function freshFolder(prefix) {
  const folder = await mkdtemp(path.join(os.tmpdir(), prefix));
  temporary.push(folder);
  return folder;
}
const installation = await freshFolder("PLC Lab Launcher Test ");
for (const item of [LAUNCHER, "PLC Lab.exe", path.join("releases", "current")])
  await cp(path.join(root, item), path.join(installation, item), {
    recursive: true,
    errorOnExist: true,
  });
const bare = await freshFolder("PLC Lab Without Release ");
await cp(path.join(root, LAUNCHER), path.join(bare, LAUNCHER));
const shared = await freshFolder("PLC Lab Shared Projects ");
const packDir = path.join(
  installation,
  "releases",
  "current",
  "knowledge-packs",
  PACK_NAME,
);
const pack = JSON.parse(
  await readFile(path.join(packDir, "pack.json"), "utf8"),
);
const packFiles = listFiles(packDir).length;

// Each cold start gets its own DevTools port and WebView2 profile.
let port = await freePort();
let starts = 0;
const dataDir = path.join(directory, `data-${stamp}`);
const environment = () => ({
  ...process.env,
  PLC_LAB_DATA_DIR: dataDir,
  PLC_LAB_LAUNCHER_NO_DIALOG: "1",
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
  WEBVIEW2_USER_DATA_FOLDER: path.join(directory, `webview-${stamp}-${starts}`),
});
let output = "";
const launches = [];

function launch(folder, args = []) {
  const child = spawn(path.join(folder, LAUNCHER), args, {
    cwd: os.tmpdir(),
    env: environment(),
    stdio: ["ignore", "pipe", "pipe"],
  });
  const run = { child, stdout: "", stderr: "" };
  launches.push(run);
  child.stdout.on("data", (c) => {
    run.stdout += c;
    output += c;
  });
  child.stderr.on("data", (c) => {
    run.stderr += c;
    output += c;
  });
  run.exit = new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("exit", (code) => resolve(code));
  });
  return run;
}

async function finished(run, what) {
  const code = await within(run.exit, 30_000, what);
  await delay(100);
  return { code, stdout: run.stdout, stderr: run.stderr };
}

function windowAction(pid, action, { name, value } = {}) {
  const args = [
    "-NoProfile",
    "-NonInteractive",
    "-File",
    HELPER,
    "-ProcessId",
    String(pid),
    "-Action",
    action,
  ];
  if (name) args.push("-Name", name);
  if (value) args.push("-Value", value);
  const run = spawnSync(
    path.join(SYSTEM32, "WindowsPowerShell", "v1.0", "powershell.exe"),
    args,
    { encoding: "utf8", windowsHide: true, timeout: 90_000 },
  );
  if (run.error) throw run.error;
  if (run.status !== 0)
    throw new Error(
      `Window ${action} ${name ?? ""} failed: ${run.stderr || run.stdout}`,
    );
  return run.stdout.trim();
}
const readWindow = (pid) => JSON.parse(windowAction(pid, "read"));
const buttonStates = (view) =>
  Object.fromEntries(view.buttons.map((b) => [b.name, b.enabled]));

function instances(image = APP) {
  const listing =
    spawnSync(
      path.join(SYSTEM32, "tasklist.exe"),
      ["/FO", "CSV", "/NH", "/FI", `IMAGENAME eq ${image}`],
      { encoding: "utf8" },
    ).stdout ?? "";
  return listing
    .split(/\r?\n/)
    .filter((line) =>
      line.toLowerCase().startsWith(`"${image.toLowerCase()}",`),
    ).length;
}

let browser;
async function connect() {
  const endpoint = `http://127.0.0.1:${port}`;
  const deadline = Date.now() + 60_000;
  for (;;) {
    try {
      const response = await fetch(`${endpoint}/json/version`, {
        signal: AbortSignal.timeout(1000),
      });
      if (response.ok) break;
    } catch {
      // not ready yet
    }
    if (Date.now() > deadline)
      throw new Error(`PLC Lab did not start.\n${output}`);
    await delay(250);
  }
  browser = await chromium.connectOverCDP(endpoint);
  let page;
  while (!page && Date.now() < deadline + 20_000) {
    page = browser.contexts().flatMap((c) => c.pages())[0];
    if (!page) await delay(100);
  }
  assert.ok(page, "PLC Lab has no WebView2 page");
  await page
    .getByRole("navigation", { name: "Main" })
    .waitFor({ timeout: 30_000 });
  return page;
}

const invoke = (page, command, request) =>
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
    { command, request: { contractVersion: CONTRACT, ...request } },
  );

const workspaceShows = (page, name) =>
  page
    .locator('[role="toolbar"][aria-label="Workspace controls"]')
    .getByText(name, { exact: true })
    .waitFor({ timeout: 30_000 });

/** Prepare a cold start: a new DevTools port and profile, and nothing running. */
async function coldStart() {
  assert.equal(instances(), 0, `${APP} is already running`);
  port = await freePort();
  starts += 1;
}

async function closeApplication() {
  await browser?.close().catch(() => undefined);
  browser = undefined;
  assert.equal(instances(), 1, `${APP} runs before the close request`);
  // Without /F, taskkill posts WM_CLOSE: the same request as the window's close button.
  spawnSync(path.join(SYSTEM32, "taskkill.exe"), ["/IM", APP], {
    stdio: "ignore",
  });
  const deadline = Date.now() + 30_000;
  while (instances() > 0) {
    if (Date.now() > deadline)
      throw new Error(`${APP} did not exit after a close request`);
    await delay(250);
  }
}

const diagnosticsOf = async (folder) => {
  const run = await finished(
    launch(folder, ["--diagnostics"]),
    "--diagnostics",
  );
  assert.equal(run.code, 0, run.stderr);
  return run.stdout;
};
const includesAll = (text, lines, what) => {
  for (const line of lines)
    assert.ok(text.includes(line), `${what} lacks "${line}":\n${text}`);
};

let projectFile;
try {
  await step(
    "root launchers are Windows GUI programs without a console",
    async () => {
      for (const file of [
        path.join(root, LAUNCHER),
        path.join(root, "PLC Lab.exe"),
        path.join(root, "releases", "current", APP),
      ])
        assert.equal(await subsystem(file), 2, `${file} PE subsystem`);
    },
  );

  await step(
    "diagnostics describe the installation from another folder",
    async () => {
      const report = await diagnosticsOf(installation);
      includesAll(
        report,
        [
          `Launcher version: ${version}`,
          `Release version: ${version}`,
          `Installation folder: `,
          path.basename(installation),
          "Application running: no",
          "[OK] Application:",
          "[OK] Release manifest:",
          "[OK] Knowledge:",
          "[OK] WebView2:",
          "[OK] Project folder:",
          `${pack.title} ${pack.version} - loaded - ${packFiles} files`,
          "Recent project: none",
        ],
        "the diagnostics",
      );
      assert.doesNotMatch(report, /different versions/);
    },
  );

  await step("a corrupt release manifest is reported", async () => {
    const manifest = path.join(
      installation,
      "releases",
      "current",
      "release-manifest.json",
    );
    const original = await readFile(manifest);
    await writeFile(manifest, "{ corrupt");
    try {
      includesAll(
        await diagnosticsOf(installation),
        ["[WARNING] Release manifest: The release manifest is unreadable"],
        "the diagnostics",
      );
    } finally {
      await writeFile(manifest, original);
    }
  });

  await step("a missing release is explained and nothing starts", async () => {
    const start = await finished(launch(bare, ["--start"]), "--start");
    assert.equal(start.code, 1);
    includesAll(
      start.stderr,
      ["PLC Lab has not been built yet", "BUILD_PLC_LAB_RELEASE.bat"],
      "the error",
    );
    includesAll(
      await diagnosticsOf(bare),
      ["[MISSING] Application:", "[MISSING] Knowledge:"],
      "the diagnostics",
    );
    const gui = launch(bare);
    const view = readWindow(gui.child.pid);
    includesAll(
      view.texts.join("\n"),
      [
        "Status: Not installed: the application release is missing.",
        "\u2715  Application - not built: run BUILD_PLC_LAB_RELEASE.bat",
      ],
      "the window",
    );
    const buttons = buttonStates(view);
    assert.equal(buttons["Start PLC Lab"], false);
    assert.equal(buttons["Open Project..."], false);
    assert.equal(buttons.Diagnostics, true);
    windowAction(gui.child.pid, "screenshot", {
      value: path.join(directory, "launcher-missing-release.png"),
    });
    windowAction(gui.child.pid, "close");
    assert.equal((await finished(gui, "the launcher window")).code, 0);
    assert.equal(instances(), 0);
  });

  await step("unusable project files and arguments are refused", async () => {
    await writeFile(path.join(shared, "broken.plclab"), "{ not a project");
    await writeFile(path.join(shared, "notes.txt"), "{}");
    for (const [args, code, text] of [
      [
        [path.join(shared, "broken.plclab")],
        1,
        "is not a valid PLC Lab project",
      ],
      [[path.join(shared, "missing.plclab")], 1, "no longer exists"],
      [
        [path.join(shared, "notes.txt")],
        1,
        "is not a PLC Lab project (.plclab) file",
      ],
      [["--bogus"], 2, "Usage: Launcher.exe"],
      [["--start", "extra"], 2, "Usage: Launcher.exe"],
    ]) {
      const run = await finished(launch(installation, args), args.join(" "));
      assert.equal(run.code, code, `${args.join(" ")}: ${run.stderr}`);
      includesAll(run.stderr, [text], `the error for ${args.join(" ")}`);
    }
    assert.equal(instances(), 0, "nothing was started");
  });

  let page;
  await step(
    "the launcher window shows the installation and copies diagnostics",
    async () => {
      await coldStart();
      const gui = launch(installation);
      const pid = gui.child.pid;
      const view = readWindow(pid);
      assert.equal(view.title, "PLC Lab");
      const texts = view.texts.join("\n");
      includesAll(
        texts,
        [
          `Version ${version}`,
          "Status: Ready",
          "No project opened yet",
          `${pack.title} v${pack.version} \u00b7 ${packFiles} files \u00b7 loaded`,
          `\u2713  Application - PLC Lab Desktop ${version}`,
          `\u2713  Knowledge pack - ${pack.title} v${pack.version}`,
          "\u2713  WebView2 Runtime ",
          "\u2713  Project folder - ",
        ],
        "the window",
      );
      assert.deepEqual(buttonStates(view), {
        "Start PLC Lab": true,
        "Open Recent": false,
        "Open Project...": true,
        "Manage Knowledge Packs": true,
        Diagnostics: true,
        "Open Data Folder": true,
      });
      windowAction(pid, "screenshot", {
        value: path.join(directory, "launcher-ready.png"),
      });
      windowAction(pid, "invoke", { name: "Diagnostics" });
      windowAction(pid, "screenshot", {
        name: "PLC Lab diagnostics",
        value: path.join(directory, "launcher-diagnostics.png"),
      });
      const report = JSON.parse(windowAction(pid, "diagnostics"));
      includesAll(
        report.text,
        [`Launcher version: ${version}`, "[OK] Application:"],
        "the diagnostics window",
      );
      assert.equal(
        report.clipboard.replace(/\r\n/g, "\n"),
        report.text.replace(/\r\n/g, "\n"),
        "Copy Diagnostics puts the report on the clipboard",
      );

      // Start PLC Lab: the application starts and the launcher closes.
      windowAction(pid, "invoke", { name: "Start PLC Lab" });
      assert.equal((await finished(gui, "the launcher window")).code, 0);
      page = await connect();
      const info = await invoke(page, "core_info", {});
      assert.equal(info.ok.contractVersion, CONTRACT);
      assert.equal(info.ok.buildContext, "release");
      const status = await invoke(page, "knowledge_status", {});
      const loaded = status.ok.packs.find((p) => p.id === pack.id);
      assert.equal(loaded?.status, "LOADED");
      assert.ok(
        loaded.path.includes(path.basename(installation)),
        `the application loaded ${loaded.path}`,
      );
      assert.equal(instances(), 1);
    },
  );

  await step("a project file opens in the running application", async () => {
    const created = await invoke(page, "create_project", {
      name: PROJECT,
      exercise: null,
      scene: "scene.conveyor-drive",
      platform: "generic.iec",
    });
    assert.ok(created.ok, JSON.stringify(created.error));
    const exported = await invoke(page, "export_project", {
      id: created.ok.id,
    });
    assert.ok(exported.ok, JSON.stringify(exported.error));
    projectFile = path.join(shared, "Line 3 bench.plclab");
    await writeFile(projectFile, exported.ok.text);
    const run = await finished(
      launch(installation, [projectFile]),
      "the hand-over",
    );
    assert.equal(run.code, 0, run.stderr);
    await workspaceShows(page, PROJECT);
    await delay(2_000);
    assert.equal(instances(), 1, "the running application opened the project");
  });

  await step(
    "Open Recent hands the recent project to the running application",
    async () => {
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page.getByRole("button", { name: "HOME", exact: true }).waitFor();
      const gui = launch(installation);
      const pid = gui.child.pid;
      const view = readWindow(pid);
      includesAll(
        view.texts.join("\n"),
        [
          PROJECT,
          "Status: PLC Lab is already running: Start brings its window to the front.",
        ],
        "the window",
      );
      assert.equal(buttonStates(view)["Open Recent"], true);
      windowAction(pid, "screenshot", {
        value: path.join(directory, "launcher-running.png"),
      });
      windowAction(pid, "invoke", { name: "Open Recent" });
      assert.equal((await finished(gui, "the launcher window")).code, 0);
      await workspaceShows(page, PROJECT);
      await delay(2_000);
      assert.equal(instances(), 1);
    },
  );

  await step("a second start activates the running instance", async () => {
    const run = await finished(launch(installation, ["--start"]), "--start");
    assert.equal(run.code, 0, run.stderr);
    await delay(3_000);
    assert.equal(instances(), 1);
    const info = await invoke(page, "core_info", {});
    assert.equal(
      info.ok.contractVersion,
      CONTRACT,
      "the first instance answers",
    );
    includesAll(
      await diagnosticsOf(installation),
      ["Application running: yes", `Recent project: ${PROJECT}`],
      "the diagnostics",
    );
    await closeApplication();
  });

  await step("Open Project starts PLC Lab with the chosen file", async () => {
    await coldStart();
    const gui = launch(installation);
    const pid = gui.child.pid;
    readWindow(pid);
    windowAction(pid, "invoke", { name: "Open Project..." });
    windowAction(pid, "open-file", { value: projectFile });
    assert.equal((await finished(gui, "the launcher window")).code, 0);
    page = await connect();
    await workspaceShows(page, PROJECT);
    await closeApplication();
  });

  await step(
    "a project argument starts PLC Lab with that project",
    async () => {
      await coldStart();
      const run = await finished(
        launch(installation, [projectFile]),
        "the start",
      );
      assert.equal(run.code, 0, run.stderr);
      page = await connect();
      await workspaceShows(page, PROJECT);
      await page.screenshot({
        path: path.join(directory, "project-opened.png"),
      });
      await closeApplication();
    },
  );

  await result("PASS");
  console.log(`Launcher smoke PASS (${steps.length} steps)`);
} catch (error) {
  await result("FAIL", {
    error: error instanceof Error ? error.stack : String(error),
    output,
  });
  throw error;
} finally {
  await browser?.close().catch(() => undefined);
  for (const run of launches) if (run.child.exitCode === null) run.child.kill();
  if (instances() > 0)
    spawnSync(path.join(SYSTEM32, "taskkill.exe"), ["/IM", APP, "/T", "/F"], {
      stdio: "ignore",
    });
  await writeFile(path.join(directory, "native.log"), output);
  // Terminated processes may hold their files for a moment.
  for (const folder of temporary)
    for (let attempt = 0; attempt < 20; attempt++) {
      try {
        await rm(folder, { recursive: true, force: true });
        break;
      } catch {
        await delay(250);
      }
    }
}
