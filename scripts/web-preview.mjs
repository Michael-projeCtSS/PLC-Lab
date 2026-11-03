// Browser-only development entry point. This file never builds or starts Tauri.
import { existsSync } from "node:fs";
import { lstat, readFile, realpath } from "node:fs/promises";
import { createHash, randomUUID } from "node:crypto";
import { execFile, spawnSync } from "node:child_process";
import { promisify } from "node:util";
import { createInterface } from "node:readline";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  lockfileMatchesInstalled,
  onlyOverriddenProblems,
} from "./human-test.mjs";
import { root } from "./toolchain.mjs";

export const PREVIEW_URL = "http://127.0.0.1:1420";
export function npmCliFor(node = process.execPath) {
  const cli = path.join(path.dirname(node), "node_modules/npm/bin/npm-cli.js");
  if (!existsSync(cli))
    throw new Error(
      "npm is missing beside Node.js. Install the standard Node.js 24+ distribution with npm.",
    );
  return cli;
}
function runNpm(cli, args, capture = false) {
  const result = spawnSync(
    process.execPath,
    ["--use-system-ca", cli, ...args],
    {
      cwd: root,
      env: process.env,
      encoding: "utf8",
      windowsHide: true,
      ...(capture ? {} : { stdio: "inherit" }),
    },
  );
  if (result.error) throw result.error;
  return result;
}
export async function validateDependencies() {
  if (Number(process.versions.node.split(".")[0]) < 24)
    throw new Error("Node.js 24 or later is required.");
  const cli = npmCliFor();
  const expected = JSON.parse(
    await readFile(path.join(root, "package-lock.json"), "utf8"),
  );
  const installed = path.join(root, "node_modules/.package-lock.json");
  let valid = false;
  if (existsSync(installed)) {
    try {
      valid = lockfileMatchesInstalled(
        expected,
        JSON.parse(await readFile(installed, "utf8")),
      );
    } catch (error) {
      if (!(error instanceof SyntaxError)) throw error;
    }
  }
  if (valid) {
    const result = runNpm(cli, ["ls", "--all", "--json"], true);
    if (result.status !== 0) {
      const diagnostics = JSON.parse(result.stdout || "{}");
      const manifest = JSON.parse(
        await readFile(path.join(root, "package.json"), "utf8"),
      );
      valid =
        diagnostics.error?.code === "ELSPROBLEMS" &&
        onlyOverriddenProblems(diagnostics.problems, manifest.overrides);
    }
  }
  if (!valid) {
    if (
      existsSync(path.join(root, "node_modules")) &&
      (await lstat(path.join(root, "node_modules"))).isSymbolicLink()
    )
      throw new Error(
        "Linked node_modules does not match the lockfile. Repair dependencies in its owning checkout; automatic npm ci through a directory link is refused.",
      );
    console.log("Installing locked browser dependencies with npm ci...");
    const result = runNpm(cli, ["ci"]);
    if (result.status !== 0)
      throw new Error(
        `npm ci failed (exit ${result.status}). Review the installation output.`,
      );
  } else console.log("Node/npm and installed dependency integrity: PASS.");
}

export async function existingPreview() {
  try {
    const response = await fetch(`${PREVIEW_URL}/__plc_lab_preview__/status`, {
      signal: AbortSignal.timeout(1500),
    });
    if (!response.ok) return false;
    const status = await response.json();
    const rootId = createHash("sha256")
      .update((await realpath(root)).toLowerCase())
      .digest("hex");
    return (
      status.application === "PLC Lab UI preview" &&
      status.managed === true &&
      status.rootId === rootId
    );
  } catch (error) {
    if (
      error instanceof SyntaxError ||
      error instanceof TypeError ||
      error.name === "TimeoutError"
    )
      return false;
    throw error;
  }
}

async function openDefaultBrowser() {
  const execute = promisify(execFile);
  if (process.platform === "win32")
    await execute(
      "powershell.exe",
      [
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        `Start-Process -FilePath '${PREVIEW_URL}' -WindowStyle Normal`,
      ],
      { windowsHide: true, timeout: 20000 },
    );
  else
    await execute(
      process.platform === "darwin" ? "open" : "xdg-open",
      [PREVIEW_URL],
      { timeout: 20000 },
    );
}

export async function main({ openBrowser = true } = {}) {
  console.log(
    `PLC LAB — BROWSER UI PREVIEW\nRepository: ${root}\nBrowser URL: ${PREVIEW_URL}\nPLC simulation and native files require the desktop backend.`,
  );
  if (await existingPreview()) {
    console.log(
      `The preview is already running for this checkout. Reuse its browser tab at ${PREVIEW_URL}; no duplicate browser window was opened. Stop it in its original terminal with Q then Enter.`,
    );
    return;
  }
  await validateDependencies();
  const { createServer } = await import("vite");
  process.env.PLC_LAB_WEB_PREVIEW_SESSION = randomUUID();
  const server = await createServer({
    configFile: path.join(root, "apps/desktop/vite.config.ts"),
    root: path.join(root, "apps/desktop"),
    server: { host: "127.0.0.1", port: 1420, strictPort: true, open: false },
  });
  let input;
  let stopped = false;
  let listening = false;
  const stop = async () => {
    if (stopped) return;
    stopped = true;
    input?.close();
    process.stdin.pause();
    process.stdin.unref?.();
    await server.close();
    if (listening)
      console.log(
        "Browser preview stopped cleanly. Port 1420 has been released.",
      );
  };
  try {
    await server.listen();
    listening = true;
    server.printUrls();
    // Open once, after the fixed port is actually ready; retries reuse this session.
    if (openBrowser) await openDefaultBrowser();
    console.log(
      openBrowser
        ? "Default browser launch requested."
        : "Browser opening disabled explicitly for automated testing (--no-open).",
    );
    console.log(
      "Edit UI/content to see Vite updates.\nTo stop: Q then Enter, or Ctrl+C. Closing this terminal also ends this in-process server.",
    );
    input = createInterface({ input: process.stdin });
    input.on("line", (line) => {
      if (line.trim().toLowerCase() === "q")
        void stop().catch((error) => {
          console.error(error.message);
          process.exitCode = 1;
        });
    });
    input.on(
      "close",
      () =>
        void stop().catch((error) => {
          console.error(error.message);
          process.exitCode = 1;
        }),
    );
    process.once("SIGINT", () => void stop());
    process.once("SIGTERM", () => void stop());
  } catch (error) {
    await stop();
    throw new Error(
      `Browser preview could not start. Port 1420 may be occupied by another server; stop that server or use its existing tab. ${error.message}`,
      { cause: error },
    );
  }
}
if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const args = process.argv.slice(2);
  if (args.some((arg) => arg !== "--no-open")) {
    console.error("Usage: node scripts/web-preview.mjs [--no-open]");
    process.exitCode = 1;
  } else
    main({ openBrowser: !args.includes("--no-open") }).catch((error) => {
      console.error(`WEB PREVIEW FAILED: ${error.message}`);
      process.exitCode = 1;
    });
}
