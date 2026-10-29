// Native end-to-end smoke test of the real desktop application (WebView2 + Rust core).
// No transport mocks: the test drives the real UI, which calls the real backend.
//
// Usage: node tests/e2e/desktop-smoke.mjs [--exe <path>] [--build development|release]
//                                         [--out <artifacts subfolder>] [--bat] [--relocate]
//
// Every launch uses another working directory than the executable's folder.
// --exe       the executable relative to the product root; "PLC Lab.exe" is the user's
//             entry point (the application itself, ADR-0030).
// --bat       start through RUN_PLC_LAB.bat (the optional fallback) instead of --exe.
// --relocate  first copy the staged distribution (PLC Lab.exe, resources/, the BAT) into a
//             fresh folder whose path contains spaces and start from that copy.
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { cp, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { parseArgs } from "node:util";
import { chromium } from "playwright-core";
import {
  APPLICATION,
  distributionEntries,
  verifyDistribution,
} from "../../scripts/distribution.mjs";
import { taskListContainsApplication } from "../../scripts/human-test.mjs";
import { root } from "../../scripts/toolchain.mjs";

const { values } = parseArgs({
  options: {
    exe: { type: "string", default: "target/debug/plc-lab-desktop.exe" },
    build: { type: "string", default: "development" },
    out: { type: "string", default: "desktop-smoke" },
    bat: { type: "boolean", default: false },
    relocate: { type: "boolean", default: false },
  },
});
// The batch file starts PLC Lab.exe and exits.
const detached = values.bat;
if (process.platform !== "win32")
  throw new Error("The native smoke test requires Windows and WebView2.");

// Human-visible text that names the cause of ts.motor-wont-start.
const LAB_GUIDE = "Stop button contact replaced by a normally open block";
const LAB_REPAIR = "Fit a normally closed contact block to the Stop button";
const LAB_ROOT_CAUSE = "received a normally open contact block";

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
    `${JSON.stringify({ result: state, executable: values.exe, build: values.build, bat: values.bat, relocate: values.relocate, steps, ...extra }, null, 2)}\n`,
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
// The folder the application is started from: the repository or a relocated copy.
let installation = root;
if (values.relocate) {
  installation = await mkdtemp(
    path.join(os.tmpdir(), "PLC Lab Product Test With Spaces "),
  );
  for (const item of await distributionEntries(root))
    await cp(path.join(root, item), path.join(installation, item), {
      recursive: true,
      errorOnExist: true,
    });
}
// A release runs from the staged distribution: one application executable, no second one.
if (values.build === "release") {
  const problems = await verifyDistribution(installation);
  assert.deepEqual(problems, [], `distribution in ${installation}`);
}
const target = values.bat
  ? path.join(installation, "RUN_PLC_LAB.bat")
  : path.resolve(installation, values.exe);
// The process that owns the window: PLC Lab.exe for the BAT and the root executable.
const image = values.bat ? APPLICATION : path.basename(target);
const environment = {
  ...process.env,
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
  WEBVIEW2_USER_DATA_FOLDER: path.join(directory, `webview-${stamp}`),
  PLC_LAB_DATA_DIR: path.join(directory, `data-${stamp}`),
  PLC_LAB_NO_PAUSE: "1",
  // A startup problem is printed and exits instead of waiting in a message box.
  PLC_LAB_NO_DIALOG: "1",
};
const application = values.bat
  ? spawn(
      process.env.ComSpec ?? "cmd.exe",
      ["/d", "/s", "/c", `""${target}""`],
      {
        cwd: os.tmpdir(),
        env: environment,
        windowsHide: true,
        windowsVerbatimArguments: true,
        stdio: ["ignore", "pipe", "pipe"],
      },
    )
  : spawn(target, [], {
      // The application must not depend on the working directory.
      cwd: os.tmpdir(),
      env: environment,
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
        !(detached && application.exitCode === 0)
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

  const changeLanguage = async (language) => {
    await page.getByRole("button", { name: /^(Settings|Ustawienia)$/ }).click();
    await page
      .getByRole("combobox", { name: /Language|Język/ })
      .selectOption(language);
    await page
      .getByRole("button", {
        name: language === "pl" ? "Zamknij" : "Close",
        exact: true,
      })
      .click();
  };
  await step(
    "Settings language and preferences persist after reloading the application",
    async () => {
      await changeLanguage("pl");
      await page.getByRole("button", { name: "START", exact: true }).waitFor();
      await page.screenshot({ path: path.join(directory, "home-pl.png") });
      await page
        .getByRole("button", { name: "Ustawienia", exact: true })
        .click();
      await page.getByLabel("Gęstość interfejsu").selectOption("compact");
      await page.getByLabel("Objaśnienia dla początkujących").uncheck();
      await page.screenshot({ path: path.join(directory, "settings-pl.png") });
      await page.reload();
      await page.getByRole("button", { name: "START", exact: true }).waitFor();
      assert.equal(
        await page.locator(".app").getAttribute("data-density"),
        "compact",
      );
      assert.equal(
        await page.locator(".app").getAttribute("data-explanations"),
        "false",
      );
      await page
        .getByRole("button", { name: "Ustawienia", exact: true })
        .click();
      await page.getByLabel("Gęstość interfejsu").selectOption("comfortable");
      await page.getByLabel("Objaśnienia dla początkujących").check();
      await page.getByLabel("Język").selectOption("en");
      await page.getByRole("button", { name: "Close", exact: true }).click();
      await page.reload();
      await page.getByRole("button", { name: "HOME", exact: true }).waitFor();
    },
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
    assert.equal(pack.status, "LOADED", pack.errors.join("; "));
    assert.ok(pack.exercises >= 17, `${pack.exercises} exercises`);
    // Paths may differ in case or short-name form; the folder layout must match.
    if (values.build === "release")
      assert.ok(
        pack.path
          .toLowerCase()
          .includes(
            `${path.sep}${path.join("resources", "knowledge-packs")}${path.sep}`.toLowerCase(),
          ),
        `the release loaded its pack from ${pack.path}, not from resources/`,
      );
    if (values.relocate)
      assert.ok(
        pack.path.includes(path.basename(installation)),
        `the relocated app loaded ${pack.path}`,
      );
  });

  await step("practice exercise opens a workspace", async () => {
    await page.getByRole("button", { name: "PRACTICE", exact: true }).click();
    await page
      .getByRole("button", { name: /Start Motor start\/stop with seal-in/ })
      .click();
    await page
      .getByRole("toolbar")
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
      await page
        .getByRole("toolbar")
        .getByRole("button", { name: "Build & Load to PLC" })
        .click();
      await page
        .getByText("Sim running", { exact: true })
        .waitFor({ timeout: 15_000 });
      await page.getByText("PLC RUN", { exact: true }).waitFor();
    },
  );

  await step("PLC and simulation controls explain themselves", async () => {
    const plcGroup = page.getByRole("group", { name: "PLC mode switch" });
    for (const [locator, text] of [
      [plcGroup.getByRole("button", { name: "RUN" }), "PLC mode switch RUN"],
      [plcGroup.getByRole("button", { name: "STOP" }), "keeps running"],
      [
        page.getByRole("button", { name: "Step", exact: true }),
        "exactly one PLC cycle",
      ],
      [
        page.getByRole("button", { name: "Stop", exact: true }),
        "not the PLC STOP switch",
      ],
    ])
      assert.match(
        (await locator.getAttribute("title")) ?? "",
        new RegExp(text),
      );
  });

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
    "troubleshooting lab: symptom, diagnosis, repair, verification and debrief",
    async () => {
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page
        .getByRole("button", { name: "TROUBLESHOOT", exact: true })
        .click();
      await page
        .getByRole("button", { name: /Start Conveyor will not start/ })
        .click();
      await page.getByText("Fault report").waitFor({ timeout: 30_000 });
      // Nothing visible names the cause before the diagnosis.
      const before = await page.locator("body").innerText();
      for (const secret of [
        "contactTypeMismatch",
        LAB_GUIDE,
        LAB_REPAIR,
        LAB_ROOT_CAUSE,
        "What was wrong",
      ])
        assert.ok(
          !before.includes(secret),
          `visible before diagnosis: ${secret}`,
        );
      await page.getByText("Helpful concepts").waitFor();
      await page.getByText(/unlock the repair step/).waitFor();
      // Symptom: Start does nothing.
      await page.getByRole("button", { name: "Run", exact: true }).click();
      await page.getByRole("tab", { name: "Process" }).click();
      const start = page.getByRole("button", {
        name: "Start S1 (hold to press)",
      });
      await start.dispatchEvent("pointerdown", { pointerId: 1 });
      await delay(600);
      await start.dispatchEvent("pointerup", { pointerId: 1 });
      await delay(1500);
      await page
        .locator('[aria-label="Conveyor motor M1: stopped"]')
        .waitFor({ timeout: 10_000 });
      await page.getByRole("button", { name: "Pause" }).click();
      // Diagnosis, repair and verification.
      await page.getByRole("tab", { name: "Investigation" }).click();
      await page.getByLabel("Root cause").selectOption("stop-contact-no");
      await page.getByRole("button", { name: "Submit diagnosis" }).click();
      await page
        .locator(".notice.success", { hasText: "(attempt 1)" })
        .waitFor({ timeout: 10_000 });
      assert.ok(
        !(await page.locator("body").innerText()).includes(LAB_GUIDE),
        "the guide stays hidden until verification",
      );
      await page.getByRole("button", { name: LAB_REPAIR }).click();
      await page
        .getByRole("button", { name: /^✓ Fit a normally closed contact block/ })
        .waitFor({ timeout: 10_000 });
      await page.getByRole("button", { name: "Verify repair" }).click();
      await page
        .getByLabel("Check result")
        .getByText("Passed", { exact: true })
        .waitFor({ timeout: 30_000 });
      const debrief = page.getByLabel("Debrief");
      for (const heading of [
        "What was wrong",
        "Evidence that pointed to it",
        "Correct repair",
        "How to reason about it",
      ])
        await debrief.getByText(heading, { exact: true }).waitFor();
      await debrief.getByText(new RegExp(LAB_ROOT_CAUSE)).waitFor();
      await debrief.getByRole("button", { name: LAB_GUIDE }).waitFor();
      await page.screenshot({
        path: path.join(directory, "troubleshooting-debrief.png"),
      });
    },
  );

  await step(
    "commissioning lab: the checklist reports what the installation shows",
    async () => {
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page
        .getByRole("button", { name: "COMMISSION", exact: true })
        .click();
      await page
        .getByRole("button", { name: /Start Commissioning: no input responds/ })
        .click();
      await page
        .getByRole("tab", { name: "Commissioning" })
        .click({ timeout: 30_000 });
      await page
        .getByLabel("I/O field supply: FAULT")
        .waitFor({ timeout: 15_000 });
      await page.getByLabel("Sensor and button operation: UNTESTED").waitFor();
      const body = await page.locator("body").innerText();
      for (const secret of [
        "Replace the blown fuse in the input module",
        "fieldPowerMissing",
        "What was wrong",
      ])
        assert.ok(
          !body.includes(secret),
          `visible before diagnosis: ${secret}`,
        );
      await page.screenshot({
        path: path.join(directory, "commissioning-checklist.png"),
      });
    },
  );

  await step(
    "network lab: the traffic shows what the gateway answers",
    async () => {
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page
        .getByRole("button", { name: "COMMISSION", exact: true })
        .click();
      await page
        .getByRole("button", {
          name: /Start Network commissioning: the gateway rejects/,
        })
        .click();
      await page
        .getByRole("button", { name: "Run", exact: true })
        .click({ timeout: 30_000 });
      await page.getByRole("tab", { name: "Network" }).click();
      const traffic = page.getByRole("table", { name: "Modbus traffic" });
      await traffic
        .getByText(/exception 0Bh: gateway target device failed to respond/)
        .first()
        .waitFor({ timeout: 15_000 });
      await page.getByRole("button", { name: "li402" }).click();
      await page
        .getByRole("region", { name: "Connection li402" })
        .getByText("40101 … 40102")
        .waitFor();
      const body = await page.locator("body").innerText();
      for (const secret of ["What was wrong", "Gateway rejects the unit ID"])
        assert.ok(
          !body.includes(secret),
          `visible before diagnosis: ${secret}`,
        );
      await page.getByRole("button", { name: "Pause" }).click();
      await page.screenshot({
        path: path.join(directory, "network-traffic.png"),
      });
    },
  );

  await step(
    "hardware lab: configured versus detected, module inspector and signal mapping",
    async () => {
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page
        .getByRole("button", { name: "COMMISSION", exact: true })
        .click();
      await page
        .getByRole("button", {
          name: /Start Hardware: the level measurement is invalid/,
        })
        .click();
      await page
        .getByRole("button", { name: "Run", exact: true })
        .click({ timeout: 30_000 });
      await page.getByRole("tab", { name: "Hardware", exact: true }).click();
      const compare = page.getByRole("region", {
        name: "Configured versus actual",
      });
      await compare.getByText("Wrong module type").waitFor({ timeout: 15_000 });
      await compare
        .getByText("Digital input 8 x 24 V DC (sinking)")
        .first()
        .waitFor();
      // Select the module in slot 4 from the keyboard and open its diagnostics.
      await page
        .getByRole("list", { name: "Rack" })
        .getByRole("button", { name: /^Slot 4/ })
        .focus();
      await page.keyboard.press("Enter");
      await page
        .getByRole("tablist", { name: "Module" })
        .getByRole("tab", { name: "Diagnostics" })
        .click();
      await page.getByRole("region", { name: "Module diagnostics" }).waitFor();
      const views = page.getByRole("tablist", { name: "Station views" });
      await views.getByRole("tab", { name: "Actual (detected)" }).click();
      await page
        .getByRole("list", { name: "Detected station" })
        .getByText("DI8.24VDC.SINK")
        .waitFor();
      await views.getByRole("tab", { name: "Signals" }).click();
      const signals = page.getByRole("region", { name: "Signal mapping" });
      await signals.getByText("LevelRaw", { exact: true }).waitFor();
      await signals.getByText("module fault").first().waitFor();
      const body = await page.locator("body").innerText();
      for (const secret of [
        "What was wrong",
        "wrongModuleInstalled",
        "Module of another type in a configured slot",
        "Replace the module in slot 4",
      ])
        assert.ok(
          !body.includes(secret),
          `visible before diagnosis: ${secret}`,
        );
      // A safe edit in Advanced mode is a backend project save: the view then reports
      // that the project differs from the PLC, and Discard restores it.
      const mode = page.getByRole("group", { name: "Interface mode" });
      await mode.getByRole("button", { name: "ADVANCED" }).click();
      await views.getByRole("tab", { name: /^Compare/ }).click();
      await page
        .getByRole("list", { name: "Rack" })
        .getByRole("button", { name: /^Slot 5/ })
        .click();
      await page
        .getByRole("tablist", { name: "Module" })
        .getByRole("tab", { name: "Configuration" })
        .click();
      await page.getByLabel("Move to slot").selectOption("7");
      const differs = page.getByText(/differs from the one loaded in the PLC/);
      await differs.waitFor({ timeout: 10_000 });
      await views.getByRole("tab", { name: "Configured (project)" }).click();
      await page
        .getByRole("list", { name: "Configured station" })
        .getByRole("button", { name: /^Slot 7 ao-r0s5/ })
        .waitFor();
      await page.getByRole("button", { name: "Discard changes" }).click();
      await differs.waitFor({ state: "detached", timeout: 10_000 });
      await mode.getByRole("button", { name: "BASIC" }).click();
      await page.getByRole("button", { name: "Pause" }).click();
      await page.screenshot({
        path: path.join(directory, "hardware-compare.png"),
      });
    },
  );

  await step(
    "engineering tools: watch table, variable detail, forcing and diagnostic buffer",
    async () => {
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page.getByRole("button", { name: "FREE LAB", exact: true }).click();
      await page.getByRole("button", { name: "New project" }).click();
      await page
        .getByLabel("Machine (scene)")
        .selectOption("scene.hardware-station");
      await page.getByRole("button", { name: "Create and open" }).click();
      await page
        .getByRole("toolbar")
        .getByRole("button", { name: "Build & Load to PLC" })
        .click({ timeout: 30_000 });
      await page.getByRole("button", { name: "ADVANCED", exact: true }).click();
      const monitoring = page.getByRole("tablist", { name: "Monitoring" });
      // A watch table row of a canonical signal and its online detail.
      await monitoring.getByRole("tab", { name: /^Watch/ }).click();
      await page.getByRole("button", { name: "New table" }).click();
      await page.getByLabel("Find a signal").fill("LevelRaw");
      await page.getByRole("button", { name: /^Add LevelRaw/ }).click();
      const table = page.getByRole("table", { name: /^Watch table / });
      await table.getByText("LevelRaw").first().waitFor({ timeout: 15_000 });
      await table.getByRole("button", { name: "Details" }).click();
      await page
        .getByLabel("Online variable")
        .getByText("plc.LevelRaw")
        .waitFor({ timeout: 10_000 });
      // A forced output: the banner warns until all forces are removed.
      await monitoring.getByRole("tab", { name: "Variables" }).click();
      await page
        .getByRole("row")
        .filter({ hasText: "InletValve" })
        .getByTitle("Force TRUE")
        .click({ timeout: 15_000 });
      const banner = page.getByRole("alert", { name: "Forces active" });
      await banner.waitFor({ timeout: 10_000 });
      await banner.getByRole("button", { name: "Show forces" }).click();
      await banner
        .getByRole("table", { name: "Active forces" })
        .getByText(/InletValve/)
        .first()
        .waitFor({ timeout: 10_000 });
      await page.screenshot({
        path: path.join(directory, "forces-active.png"),
      });
      await banner.getByRole("button", { name: "Remove all forces" }).click();
      await banner.waitFor({ state: "detached", timeout: 10_000 });
      // The CPU diagnostic buffer shows the download and the start as events.
      await page
        .getByRole("tablist", { name: "Workspace views" })
        .getByRole("tab", { name: /^Diagnostics/ })
        .click();
      const buffer = page.getByRole("table", { name: "Diagnostic buffer" });
      await buffer
        .getByText("Program downloaded into the CPU.")
        .first()
        .waitFor({ timeout: 10_000 });
      await buffer
        .getByText(/changed from STOP to RUN/)
        .first()
        .waitFor();
      await page.getByRole("button", { name: "Pause" }).click();
      await page.screenshot({
        path: path.join(directory, "engineering-tools.png"),
      });
    },
  );

  await step("project round trip through the real backend", async () => {
    const created = await invoke("create_project", {
      contractVersion: 8,
      name: "Smoke round trip",
      exercise: null,
      scene: "scene.conveyor-drive",
      platform: "generic.iec",
    });
    assert.ok(created.ok, JSON.stringify(created.error));
    const exported = await invoke("export_project", {
      contractVersion: 8,
      id: created.ok.id,
    });
    assert.ok(exported.ok, JSON.stringify(exported.error));
    assert.match(exported.ok.fileName, /\.plclab$/);
    const imported = await invoke("import_project", {
      contractVersion: 8,
      text: exported.ok.text,
    });
    assert.ok(imported.ok, JSON.stringify(imported.error));
    assert.notEqual(imported.ok.id, created.ok.id);
    assert.equal(imported.ok.scene, "scene.conveyor-drive");
    assert.deepEqual(imported.ok.programs, created.ok.programs);
    assert.deepEqual(imported.ok.station, created.ok.station);
    const corrupt = await invoke("import_project", {
      contractVersion: 8,
      text: "{ not a project",
    });
    assert.ok(corrupt.error, "a corrupt file must be rejected");
    const list = await invoke("projects", { contractVersion: 8 });
    const ids = list.ok.projects.map((p) => p.id);
    assert.ok(ids.includes(created.ok.id) && ids.includes(imported.ok.id));
  });

  await step(
    "free lab workspace fits 1366x768, 1920x1080 and 2560x1440",
    async () => {
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page.getByRole("button", { name: "FREE LAB", exact: true }).click();
      await page.getByRole("button", { name: "New project" }).click();
      await page
        .getByLabel("Machine (scene)")
        .selectOption("scene.conveyor-drive");
      await page.getByRole("button", { name: "Create and open" }).click();
      await page
        .getByRole("toolbar")
        .getByRole("button", { name: "Build & Load to PLC" })
        .waitFor({ timeout: 30_000 });
      await changeLanguage("pl");
      assert.equal(
        await page.getByRole("tab", { name: "Sprzęt", exact: true }).count(),
        0,
      );
      assert.equal(
        await page.getByRole("tab", { name: "Przebiegi", exact: true }).count(),
        0,
      );
      await page
        .getByRole("button", { name: "ZAAWANSOWANY", exact: true })
        .click();
      await page.getByRole("tab", { name: "Sprzęt", exact: true }).waitFor();
      await page.getByRole("tab", { name: "Przebiegi", exact: true }).waitFor();
      await page
        .getByRole("button", { name: "PODSTAWOWY", exact: true })
        .click();
      const cdp = await page.context().newCDPSession(page);
      for (const [width, height] of [
        [1366, 768],
        [1920, 1080],
        [2560, 1440],
      ]) {
        await cdp.send("Emulation.setDeviceMetricsOverride", {
          width,
          height,
          deviceScaleFactor: 1,
          mobile: false,
        });
        await delay(300);
        const layout = await page.evaluate(() => {
          const box = (selector) => {
            const r = document.querySelector(selector)?.getBoundingClientRect();
            return r && { w: r.width, h: r.height, right: r.right };
          };
          return {
            scrollWidth: document.documentElement.scrollWidth,
            clientWidth: document.documentElement.clientWidth,
            toolbar: box('[role="toolbar"]'),
            editor: box('[data-testid="st-editor"]'),
            inspector: box(".ws-right"),
          };
        });
        assert.ok(
          layout.scrollWidth <= layout.clientWidth,
          `${width}x${height}: horizontal overflow ${JSON.stringify(layout)}`,
        );
        for (const part of ["toolbar", "editor", "inspector"]) {
          const b = layout[part];
          assert.ok(
            b && b.w > 0 && b.h > 0 && b.right <= width + 1,
            `${width}x${height}: ${part} not fully visible ${JSON.stringify(layout)}`,
          );
        }
        await page.screenshot({
          path: path.join(directory, `free-lab-${width}x${height}.png`),
        });
      }
      await cdp.send("Emulation.clearDeviceMetricsOverride");
      await cdp.detach();
      await changeLanguage("en");
      await page.getByRole("tab", { name: "Process" }).click();
      await page
        .locator('[aria-label^="Conveyor drive U1 / motor M1:"]')
        .waitFor({ timeout: 10_000 });
      await page.getByRole("button", { name: "Back", exact: true }).click();
    },
  );

  await step("knowledge search returns results", async () => {
    await page.getByRole("button", { name: "KNOWLEDGE", exact: true }).click();
    await page.getByLabel("Search knowledge").fill("motor");
    await page.locator(".list .list-row").first().waitFor({ timeout: 10_000 });
  });

  assert.deepEqual(pageErrors, [], "the web view reported script errors");
  await page.screenshot({ path: path.join(directory, "final.png") });

  await step("a second launch activates the running instance", async () => {
    const second = values.bat
      ? spawn(
          process.env.ComSpec ?? "cmd.exe",
          ["/d", "/s", "/c", `""${target}""`],
          {
            cwd: os.tmpdir(),
            env: environment,
            windowsHide: true,
            windowsVerbatimArguments: true,
            stdio: "ignore",
          },
        )
      : spawn(target, [], {
          cwd: os.tmpdir(),
          env: environment,
          windowsHide: true,
          stdio: "ignore",
        });
    const code = await new Promise((resolve, reject) => {
      const timer = setTimeout(
        () => reject(new Error("the second launch did not exit")),
        30_000,
      );
      second.once("error", reject);
      second.once("exit", (c) => {
        clearTimeout(timer);
        resolve(c);
      });
    });
    assert.equal(code, 0, "the second launch exits cleanly");
    // A second launch hands over and exits at once; give a second window time to appear.
    await delay(5_000);
    const instances = (
      spawnSync(
        path.join(
          process.env.SystemRoot ?? "C:/Windows",
          "System32",
          "tasklist.exe",
        ),
        ["/FO", "CSV", "/NH", "/FI", `IMAGENAME eq ${image}`],
        { encoding: "utf8" },
      ).stdout ?? ""
    )
      .split(/\r?\n/)
      .filter((line) =>
        line.toLowerCase().startsWith(`"${image.toLowerCase()}",`),
      );
    assert.equal(
      instances.length,
      1,
      `${image} instances: ${instances.join(" | ")}`,
    );
    const info = await invoke("core_info", { contractVersion: 8 });
    assert.equal(
      info.ok.contractVersion,
      8,
      "the first instance still answers",
    );
  });

  if (!values.bat)
    await step(
      "a second launch with a project file opens it in the running instance",
      async () => {
        const created = await invoke("create_project", {
          contractVersion: 8,
          name: "Handoff source",
          exercise: null,
          scene: "scene.io-basics",
          platform: "generic.iec",
        });
        assert.ok(created.ok, JSON.stringify(created.error));
        const exported = await invoke("export_project", {
          contractVersion: 8,
          id: created.ok.id,
        });
        assert.ok(exported.ok, JSON.stringify(exported.error));
        const project = JSON.parse(exported.ok.text);
        project.name = "Handed over project";
        const file = path.join(
          directory,
          `handoff ${stamp}`,
          "Handed over project.plclab",
        );
        await mkdir(path.dirname(file), { recursive: true });
        await writeFile(file, JSON.stringify(project));
        // PLC Lab.exe "<file.plclab>": the running instance receives the typed request.
        const second = spawn(target, [file], {
          cwd: os.tmpdir(),
          env: environment,
          windowsHide: true,
          stdio: "ignore",
        });
        const code = await new Promise((resolve, reject) => {
          const timer = setTimeout(
            () => reject(new Error("the project launch did not exit")),
            30_000,
          );
          second.once("error", reject);
          second.once("exit", (c) => {
            clearTimeout(timer);
            resolve(c);
          });
        });
        assert.equal(code, 0, "the project launch hands over and exits");
        await page
          .getByText("Handed over project", { exact: true })
          .first()
          .waitFor({ timeout: 15_000 });
        await page
          .getByRole("toolbar")
          .getByRole("button", { name: "Build & Load to PLC" })
          .waitFor({ timeout: 30_000 });
      },
    );

  await step("application exits on a close request", async () => {
    await browser.close().catch(() => undefined);
    browser = undefined;
    const tasklist = path.join(
      process.env.SystemRoot ?? "C:/Windows",
      "System32",
      "tasklist.exe",
    );
    const running = () =>
      taskListContainsApplication(
        spawnSync(
          tasklist,
          ["/FO", "CSV", "/NH", "/FI", `IMAGENAME eq ${image}`],
          {
            encoding: "utf8",
          },
        ).stdout ?? "",
        image,
      );
    assert.ok(running(), `${image} is not running before the close request`);
    // Without /F, taskkill posts WM_CLOSE to the window: the same request as its close button.
    spawnSync("taskkill", ["/IM", image], { stdio: "ignore" });
    const deadline = Date.now() + 30_000;
    while (running()) {
      if (Date.now() > deadline)
        throw new Error(`${image} did not exit after a close request`);
      await delay(250);
    }
    if (!detached) {
      while (application.exitCode === null && Date.now() < deadline)
        await delay(50);
      assert.equal(application.exitCode, 0, `${image} exit code`);
    }
  });

  if (!values.bat)
    await step(
      "language and density survive closing and restarting the real application",
      async () => {
        for (const expected of ["en", "pl"]) {
          const restarted = spawn(target, [], {
            cwd: os.tmpdir(),
            env: environment,
            windowsHide: true,
            stdio: "ignore",
          });
          let startError;
          restarted.once("error", (error) => {
            startError = error;
          });
          const exited = new Promise((resolve) =>
            restarted.once("exit", resolve),
          );
          let current;
          try {
            const deadline = Date.now() + 30_000;
            while (true) {
              if (startError) throw startError;
              if (restarted.exitCode !== null)
                throw new Error(`Restart exited ${restarted.exitCode}`);
              try {
                const response = await fetch(
                  `http://127.0.0.1:${port}/json/version`,
                  { signal: AbortSignal.timeout(1000) },
                );
                if (response.ok) break;
              } catch {
                /* WebView starts asynchronously. */
              }
              if (Date.now() > deadline)
                throw new Error("Restarted WebView did not become ready");
              await delay(100);
            }
            current = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
            const view = current.contexts().flatMap((c) => c.pages())[0];
            assert.ok(view);
            await view
              .getByRole("button", {
                name: expected === "en" ? "HOME" : "START",
                exact: true,
              })
              .waitFor();
            if (expected === "pl") {
              assert.equal(
                await view.locator(".app").getAttribute("data-density"),
                "compact",
              );
              assert.equal(
                await view.locator(".app").getAttribute("data-explanations"),
                "false",
              );
            }
            await view
              .getByRole("button", {
                name: expected === "en" ? "Settings" : "Ustawienia",
                exact: true,
              })
              .click();
            await view
              .getByRole("combobox", { name: /Language|Język/ })
              .selectOption(expected === "en" ? "pl" : "en");
            await view
              .getByLabel(
                expected === "en" ? "Gęstość interfejsu" : "UI density",
              )
              .selectOption(expected === "en" ? "compact" : "comfortable");
            await view
              .getByLabel(
                expected === "en"
                  ? "Objaśnienia dla początkujących"
                  : "Beginner explanations",
              )
              .setChecked(expected !== "en");
            await current.close();
            current = undefined;
            const close = spawnSync(
              "taskkill",
              ["/PID", String(restarted.pid)],
              { encoding: "utf8" },
            );
            assert.equal(close.status, 0, close.stderr);
            const code = await Promise.race([
              exited,
              delay(10_000).then(() => {
                throw new Error("Restarted app did not close");
              }),
            ]);
            assert.equal(code, 0);
          } finally {
            await current?.close().catch(() => undefined);
            if (restarted.exitCode === null) restarted.kill();
          }
        }
      },
    );

  await result("PASS");
  console.log(
    `Native smoke PASS (${path.relative(root, target) || target}, ${steps.length} steps)`,
  );
} catch (error) {
  await result("FAIL", {
    error: error instanceof Error ? error.stack : String(error),
    output,
  });
  throw error;
} finally {
  await browser?.close().catch(() => undefined);
  if (application.exitCode === null) application.kill();
  if (detached)
    spawnSync("taskkill", ["/IM", APPLICATION, "/T", "/F"], {
      stdio: "ignore",
    });
  await writeFile(path.join(directory, "native.log"), output);
  if (values.relocate) {
    // The terminated process may hold its files for a moment.
    for (let attempt = 0; attempt < 20; attempt++) {
      try {
        await rm(installation, { recursive: true, force: true });
        break;
      } catch {
        await delay(250);
      }
    }
  }
}
