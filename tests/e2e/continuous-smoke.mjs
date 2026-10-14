import assert from "node:assert/strict";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { CONTRACT_VERSION } from "../../packages/ipc/src/generated.ts";

/** Real automatic host pacing through Tauri/WebView2. No test-issued ticks. */
export async function interactiveContinuousSmoke({
  page,
  editSource,
  compileButton,
  defaultSource,
  tonSource,
  directory,
}) {
  const button = (name) => page.getByRole("button", { name, exact: true });
  let sessionId;
  const query = (command) =>
    page.evaluate(
      ({ command, request }) =>
        window.__TAURI_INTERNALS__.invoke(command, { request }),
      {
        command,
        request: {
          contractVersion: CONTRACT_VERSION,
          sessionId,
          ...(command === "get_trace_page"
            ? { afterSequence: null, limit: 256 }
            : {}),
        },
      },
    );
  const mode = async (value) =>
    page.waitForFunction(
      (value) =>
        document.querySelector('[data-testid="execution-mode"]')
          ?.textContent === value,
      value,
    );
  const count = () => page.getByTestId("runtime-scan-count").innerText();
  const cycles = async (minimum) =>
    page.waitForFunction(
      (minimum) =>
        BigInt(
          document.querySelector('[data-testid="runtime-scan-count"]')
            ?.textContent ?? "0",
        ) >= BigInt(minimum),
      String(minimum),
    );
  const load = async () => {
    await compileButton.click();
    await page
      .getByTestId("compile-status")
      .getByText("COMPILE SUCCESS", { exact: true })
      .waitFor();
    await button("Load Program").click();
    await page
      .getByTestId("runtime-state")
      .getByText("READY", { exact: true })
      .waitFor();
    sessionId = await page
      .getByTestId("runtime-status")
      .getAttribute("data-session-id");
    assert.ok(sessionId);
    return query("get_runtime_snapshot");
  };
  const quiescent = async (expectedMode) => {
    await mode(expectedMode);
    const before = await query("get_runtime_snapshot"),
      history = await query("get_trace_page");
    const lastCycle = history.entries
      .filter((e) => e.kind === "ScanCompleted")
      .at(-1);
    assert.ok(lastCycle);
    for (const field of [
      "completedScans",
      "simulationTimeMicros",
      "variables",
      "functionBlocks",
      "process",
      "fault",
    ])
      assert.deepEqual(
        before[field],
        lastCycle[field],
        `${expectedMode} changed ${field} instead of preserving the last completed cycle.`,
      );
    await delay(550); // Test observation window, never domain time or a cycle driver.
    const after = await query("get_runtime_snapshot");
    assert.deepEqual(
      after,
      before,
      `${expectedMode} changed PLC/TON/process/count/time.`,
    );
    assert.deepEqual(
      await query("get_trace_page"),
      history,
      `${expectedMode} appended automatic cycles.`,
    );
    return { before, after, history, observationWindowMs: 550 };
  };
  await button("Use Example").click();
  await page
    .getByRole("alertdialog", { name: "Replace Structured Text source" })
    .getByRole("button", { name: "Replace Source", exact: true })
    .click();
  await load();
  await button("Attach Tank Scene").click();
  await page.getByTestId("tank-scene").waitFor();
  await button("Run").click();
  await mode("RUNNING");
  for (const name of [
    "Run",
    "Single Scan",
    "Compile",
    "Load Program",
    "Reset Runtime",
    "Unload",
    "Reset Process",
    "Detach Process",
  ])
    assert.equal(
      await button(name).isDisabled(),
      true,
      `${name} must be unavailable during Run.`,
    );
  await cycles(3);
  await button("Pause").click();
  const pause = await quiescent("PAUSED");
  assert.equal(await button("Resume").isEnabled(), true);
  assert.equal(await button("Single Scan").isDisabled(), true);
  await button("Resume").click();
  await mode("RUNNING");
  await cycles(BigInt(pause.before.completedScans) + 3n);
  await page
    .locator('[data-trace-kind="ScanCompleted"][data-trace-scan="1"]')
    .click();
  const inspector = page.getByTestId("trace-inspector");
  const selectedSequence = await inspector.getAttribute(
      "data-selected-sequence",
    ),
    historical = await inspector.innerText(),
    beforeHistory = await count();
  await cycles(BigInt(beforeHistory) + 3n);
  assert.equal(await inspector.innerText(), historical);
  assert.equal(
    await inspector.getAttribute("data-selected-sequence"),
    selectedSequence,
  );
  assert.equal(await page.getByTestId("execution-mode").innerText(), "RUNNING");
  await cycles(9);
  await button("Stop").click();
  const stop = await quiescent("STOPPED");
  assert.ok(
    BigInt(stop.before.completedScans) > BigInt(pause.before.completedScans),
  );
  const scans = stop.history.entries.filter((e) => e.kind === "ScanCompleted");
  assert.equal(stop.before.process.levelPermille, 800, "Stop reset the tank.");
  const automaticTankProof = [];
  for (const [scan, time, inlet] of [
    ["8", "800000", true],
    ["9", "900000", false],
  ]) {
    const e = scans.find((e) => e.completedScans === scan);
    assert.ok(e);
    assert.equal(e.simulationTimeMicros, time);
    assert.equal(e.process.levelPermille, 800);
    assert.equal(e.process.highLevel, true);
    assert.equal(e.process.inletValveOpen, inlet);
    assert.equal(e.executionMode, "RUNNING");
    automaticTankProof.push(e);
  }
  assert.equal(
    await page.getByTestId("tank-level").innerText(),
    `${(stop.before.process.levelPermille / 10).toFixed(1)} %`,
  );
  assert.equal(
    await page.getByTestId("tank-fill").getAttribute("height"),
    String((105 * stop.before.process.levelPermille) / 1000),
  );
  assert.equal(
    await page.getByTestId("runtime-time").innerText(),
    `${BigInt(stop.before.simulationTimeMicros) / 1000n} ms`,
  );
  await page.screenshot({ path: path.join(directory, "continuous-tank.png") });
  assert.equal(await button("Single Scan").isEnabled(), true);
  const previous = await count();
  await button("Single Scan").click();
  await cycles(BigInt(previous) + 1n);
  const manual = await query("get_runtime_snapshot");
  assert.equal(
    BigInt(manual.completedScans),
    BigInt(stop.before.completedScans) + 1n,
  );
  assert.equal(
    BigInt(manual.simulationTimeMicros),
    BigInt(stop.before.simulationTimeMicros) + 100000n,
  );
  await button("Unload").click();
  await page
    .getByTestId("runtime-status")
    .getByText("NOT LOADED", { exact: true })
    .waitFor();
  await editSource(tonSource);
  await load();
  await page.getByLabel("Set Start", { exact: true }).click();
  await button("Run").click();
  await cycles(2);
  await button("Pause").click();
  const tonPause = await quiescent("PAUSED");
  assert.ok(tonPause.before.functionBlocks.length > 0);
  await button("Resume").click();
  await cycles(BigInt(tonPause.before.completedScans) + 2n);
  await button("Stop").click();
  const tonStop = await quiescent("STOPPED");
  assert.ok(
    BigInt(tonStop.before.completedScans) >
      BigInt(tonPause.before.completedScans),
  );
  await button("Unload").click();
  await page
    .getByTestId("runtime-status")
    .getByText("NOT LOADED", { exact: true })
    .waitFor();
  await editSource(defaultSource);
  await compileButton.click();
  await page
    .getByTestId("compile-status")
    .getByText("COMPILE SUCCESS", { exact: true })
    .waitFor();
  return {
    result: "PASS",
    actualHostAutomaticScans: true,
    pause,
    stop,
    manualAfterStop: manual,
    automaticTankProof,
    historicalSelection: {
      selectedSequence,
      historical,
      liveAdvanced: true,
      modeRemainedRunning: true,
    },
    tonPause,
    tonStop,
    realBackendResponsesCorrelated: true,
  };
}
