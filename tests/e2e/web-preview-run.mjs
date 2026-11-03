// Actual BAT lifecycle + real browser UI, then error conditions and a checkout path with spaces.
import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import {
  copyFile,
  cp,
  mkdir,
  mkdtemp,
  realpath,
  rm,
  writeFile,
} from "node:fs/promises";
import { createServer } from "node:http";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { root } from "../../scripts/toolchain.mjs";

const url = "http://127.0.0.1:1420";
const output = path.join(root, "artifacts/web-preview-launcher");
await mkdir(output, { recursive: true });
const steps = [];
const children = [];
function launch(directory, { args = "--no-open", env = {} } = {}) {
  const child =
    process.platform === "win32"
      ? spawn(
          process.env.ComSpec ?? "cmd.exe",
          [
            "/d",
            "/c",
            `call "${path.join(directory, "TEST_PLC_LAB_WEB.bat")}" ${args}`,
          ],
          {
            cwd: path.dirname(directory),
            env: { ...process.env, PLC_LAB_NO_PAUSE: "1", ...env },
            windowsVerbatimArguments: true,
            windowsHide: true,
            stdio: ["pipe", "pipe", "pipe"],
          },
        )
      : spawn(
          process.execPath,
          [path.join(directory, "scripts/web-preview.mjs"), "--no-open"],
          {
            cwd: directory,
            env: { ...process.env, ...env },
            stdio: ["pipe", "pipe", "pipe"],
          },
        );
  let logs = "";
  child.stdout.on("data", (data) => {
    logs += data;
  });
  child.stderr.on("data", (data) => {
    logs += data;
  });
  child.on("error", (error) => {
    logs += error.message;
  });
  const done = new Promise((resolve) =>
    child.on("close", (code) => resolve(code)),
  );
  const item = { child, done, logs: () => logs };
  children.push(item);
  return item;
}
async function health() {
  try {
    const response = await fetch(`${url}/__plc_lab_preview__/status`, {
      signal: AbortSignal.timeout(600),
    });
    return await response.json();
  } catch {
    return null;
  }
}
async function ready(item) {
  const deadline = Date.now() + 90000;
  while (Date.now() < deadline) {
    if (item.child.exitCode !== null)
      throw new Error(`BAT exited early: ${item.logs()}`);
    const state = await health();
    if (state?.application === "PLC Lab UI preview" && state.managed)
      return state;
    await delay(150);
  }
  throw new Error(`Preview startup timed out: ${item.logs()}`);
}
async function stop(item) {
  if (item.child.exitCode !== null) return;
  item.child.stdin.write("q\r\n");
  const result = await Promise.race([
    item.done,
    delay(15000).then(() => "TIMEOUT"),
  ]);
  assert.equal(result, 0, item.logs());
  assert.match(item.logs(), /stopped cleanly/);
  assert.equal(
    await health(),
    null,
    "the owner must release the listening port",
  );
}
async function exited(item) {
  const code = await Promise.race([
    item.done,
    delay(30000).then(() => "TIMEOUT"),
  ]);
  assert.notEqual(code, "TIMEOUT", item.logs());
  return code;
}
if (await health())
  throw new Error(
    "Stop the existing preview (Q then Enter) before the launcher lifecycle test. The test does not stop other sessions.",
  );
let fixture;
let occupied;
const started = new Date().toISOString();
function hostEvidence() {
  if (process.platform !== "win32") return null;
  const script =
    "$ErrorActionPreference='Stop'; $blocks=@(Get-WinEvent -FilterHashtable @{LogName='Microsoft-Windows-CodeIntegrity/Operational';StartTime=([datetime]$env:PLC_WEB_TEST_STARTED);Id=3077,3033} -ErrorAction SilentlyContinue | Where-Object {$_.Message -match 'PLC Lab.exe|plc-lab-desktop.exe'}); $processes=@(Get-CimInstance Win32_Process | Where-Object {$_.Name -in @('PLC Lab.exe','plc-lab-desktop.exe','Launcher.exe')} | Select-Object -ExpandProperty ProcessId); $hash=$null; if(Test-Path -LiteralPath $env:PLC_WEB_TEST_EXE){$hash=(Get-FileHash -LiteralPath $env:PLC_WEB_TEST_EXE -Algorithm SHA256).Hash}; $sac='UNAVAILABLE'; if(Get-Command Get-MpComputerStatus -ErrorAction SilentlyContinue){try{$sac=[string](Get-MpComputerStatus -ErrorAction Stop).SmartAppControlState}catch{$sac='UNAVAILABLE'}}; @{nativeProcesses=$processes;codeIntegrityBlocks=$blocks.Count;rootHash=$hash;smartAppControl=$sac} | ConvertTo-Json -Compress";
  return JSON.parse(
    execFileSync(
      "powershell.exe",
      ["-NoProfile", "-NonInteractive", "-Command", script],
      {
        encoding: "utf8",
        windowsHide: true,
        env: {
          ...process.env,
          PSModulePath: `${process.env.SystemRoot}\\System32\\WindowsPowerShell\\v1.0\\Modules;${process.env.ProgramFiles}\\WindowsPowerShell\\Modules`,
          PLC_WEB_TEST_STARTED: started,
          PLC_WEB_TEST_EXE: path.join(root, "PLC Lab.exe"),
        },
      },
    ),
  );
}
const before = hostEvidence();
try {
  const primary = launch(root);
  await ready(primary);
  steps.push(
    "actual BAT from a different working directory starts the real Vite server",
  );
  const post = await fetch(`${url}/__plc_lab_preview__/content`, {
    method: "POST",
  });
  assert.equal(post.status, 405);
  const retry = launch(root, { args: "" });
  assert.equal(await exited(retry), 0);
  assert.match(retry.logs(), /no duplicate browser window was opened/);
  steps.push(
    "retry reuses the checkout session and never opens another browser",
  );
  const browserTest = spawn(
    process.execPath,
    [path.join(root, "tests/e2e/web-preview-smoke.mjs")],
    { cwd: root, env: process.env, stdio: "inherit" },
  );
  assert.equal(
    await new Promise((resolve) => browserTest.on("close", resolve)),
    0,
  );
  steps.push("real PL/EN browser UI smoke and live update checks");
  await stop(primary);
  steps.push("Q then Enter stops cleanly and releases port 1420");
  await writeFile(path.join(output, "startup.log"), primary.logs());

  occupied = createServer((_request, response) =>
    response.end("another application's port"),
  );
  await new Promise((resolve, reject) => {
    occupied.once("error", reject);
    occupied.listen(1420, "127.0.0.1", resolve);
  });
  const conflict = launch(root);
  assert.notEqual(await exited(conflict), 0);
  assert.match(conflict.logs(), /Port 1420|port 1420/);
  assert.doesNotMatch(conflict.logs(), /Default browser launch requested/);
  assert.equal(await (await fetch(url)).text(), "another application's port");
  await new Promise((resolve) => occupied.close(resolve));
  occupied = null;
  steps.push(
    "occupied port fails clearly without stopping the other server or opening a browser",
  );

  if (process.platform === "win32") {
    const pathKey =
      Object.keys(process.env).find((key) => key.toUpperCase() === "PATH") ??
      "PATH";
    const missingNode = launch(root, {
      env: { [pathKey]: path.join(process.env.SystemRoot, "System32") },
    });
    assert.notEqual(await exited(missingNode), 0);
    assert.match(missingNode.logs(), /Node.js is missing/);
    steps.push("actual BAT diagnoses missing Node");
    fixture = await mkdtemp(path.join(output, "checkout with spaces "));
    const nodeOnly = path.join(fixture, "node-only");
    await mkdir(nodeOnly);
    // An unchanged existing Node executable is used only to test missing npm, not PLC execution.
    await copyFile(process.execPath, path.join(nodeOnly, "node.exe"));
    const missingNpm = launch(root, {
      env: {
        [pathKey]: `${nodeOnly};${path.join(process.env.SystemRoot, "System32")}`,
      },
    });
    assert.notEqual(await exited(missingNpm), 0);
    assert.match(missingNpm.logs(), /npm is missing beside Node.js/);
    steps.push("actual BAT diagnoses a Node installation without npm");
    assert.ok(
      path.resolve(nodeOnly).startsWith(`${path.resolve(output)}${path.sep}`),
    );
    await rm(nodeOnly, { recursive: true });
    for (const name of [
      "package.json",
      "package-lock.json",
      "tsconfig.json",
      "TEST_PLC_LAB_WEB.bat",
    ])
      await copyFile(path.join(root, name), path.join(fixture, name));
    for (const name of ["scripts", "packages", "knowledge", "apps/desktop"])
      await cp(path.join(root, name), path.join(fixture, name), {
        recursive: true,
        filter: (file) =>
          !["node_modules", "dist", "src-tauri"].includes(path.basename(file)),
      });
    const spaced = launch(fixture);
    await ready(spaced);
    assert.match(spaced.logs(), /checkout with spaces/);
    const response = await fetch(`${url}/__plc_lab_preview__/content`);
    assert.equal((await response.json()).lessons.length, 25);
    await stop(spaced);
    steps.push(
      "actual BAT and Vite content run from a repository path containing spaces",
    );
  }
  const after = hostEvidence();
  if (before && after) {
    assert.deepEqual(after.nativeProcesses, before.nativeProcesses);
    assert.equal(after.codeIntegrityBlocks, 0);
    assert.equal(after.rootHash, before.rootHash);
    assert.equal(after.smartAppControl, before.smartAppControl);
    steps.push(
      "native process/Code Integrity checks, root EXE hash and host Smart App Control remain unchanged",
    );
  }
  await writeFile(
    path.join(output, "result.json"),
    JSON.stringify(
      { result: "PASS", steps, hostBefore: before, hostAfter: after },
      null,
      2,
    ),
  );
  console.log(`Web launcher lifecycle PASS (${steps.length} checks).`);
} catch (error) {
  await writeFile(
    path.join(output, "result.json"),
    JSON.stringify(
      {
        result: "FAIL",
        steps,
        error: error.message,
        logs: children.map((item) => item.logs()),
      },
      null,
      2,
    ),
  );
  throw error;
} finally {
  if (occupied) await new Promise((resolve) => occupied.close(resolve));
  for (const item of children)
    if (item.child.exitCode === null) {
      item.child.stdin.end("q\r\n");
      await Promise.race([item.done, delay(10000)]);
    }
  if (fixture) {
    const absolute = await realpath(fixture);
    assert.ok(absolute.startsWith(`${path.resolve(output)}${path.sep}`));
    await rm(absolute, { recursive: true, force: true });
  }
}
