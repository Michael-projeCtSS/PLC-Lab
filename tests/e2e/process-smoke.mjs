import assert from "node:assert/strict";
import { exerciseTraceHistory } from "./trace-smoke.mjs";
import path from "node:path";
import { readFile } from "node:fs/promises";
import { CONTRACT_VERSION } from "../../packages/ipc/src/generated.ts";
import { root } from "../../scripts/toolchain.mjs";

/** Actual React -> Tauri -> Rust manual plant cycle. No transport/model mocks. */
export async function interactiveProcessSmoke({
  page,
  editSource,
  compileButton,
  defaultSource,
  directory,
}) {
  const observe = async (sessionId) =>
    page.evaluate(
      async ({ sessionId, contractVersion }) =>
        window.__TAURI_INTERNALS__.invoke("get_runtime_snapshot", {
          request: { sessionId, contractVersion },
        }),
      { sessionId, contractVersion: CONTRACT_VERSION },
    );
  const mutation = async (action) => {
    const status = page.getByTestId("runtime-status");
    const before = await status.getAttribute("data-observation-key");
    await action();
    await page.waitForFunction((before) => {
      const s = document.querySelector('[data-testid="runtime-status"]');
      return (
        s.dataset.observationKey !== before && s.dataset.runtimePending === ""
      );
    }, before);
    const id = await status.getAttribute("data-session-id");
    assert.ok(id);
    return observe(id);
  };
  const visible = async (snapshot) => {
    assert.equal(snapshot.contractVersion, CONTRACT_VERSION);
    await page.waitForFunction((snapshot) => {
      const counter = document.querySelector(
        '[data-testid="runtime-scan-count"]',
      );
      const level = document.querySelector('[data-testid="tank-level"]');
      return (
        counter?.textContent === snapshot.completedScans &&
        (snapshot.process === null
          ? level === null
          : level?.textContent ===
            `${(snapshot.process.levelPermille / 10).toFixed(1)} %`)
      );
    }, snapshot);
    const time = BigInt(snapshot.simulationTimeMicros);
    assert.equal(
      await page.getByTestId("runtime-time").innerText(),
      `${time / 1000n} ms`,
    );
    if (snapshot.process) {
      const p = snapshot.process;
      assert.equal(
        await page.getByTestId("tank-fill").getAttribute("height"),
        String((105 * p.levelPermille) / 1000),
      );
      assert.equal(
        await page.getByTestId("inlet-state").innerText(),
        `Inlet Valve: ${p.inletValveOpen ? "OPEN" : "CLOSED"}`,
      );
      assert.equal(
        await page.getByTestId("pump-state").innerText(),
        `Drain Pump: ${p.drainPumpRunning ? "RUNNING" : "STOPPED"}`,
      );
      assert.equal(
        await page.getByTestId("low-state").innerText(),
        `Low Level: ${p.lowLevel ? "ACTIVE" : "INACTIVE"}`,
      );
      assert.equal(
        await page.getByTestId("high-state").innerText(),
        `High Level: ${p.highLevel ? "ACTIVE" : "INACTIVE"}`,
      );
      for (const name of ["LevelLow", "LevelHigh", "LevelPercent"]) {
        assert.equal(
          await page
            .getByLabel(`Authority ${name}`, { exact: true })
            .innerText(),
          "PROCESS",
        );
        assert.equal(
          await page.getByLabel(`Set ${name}`, { exact: true }).count(),
          0,
        );
      }
    }
    assert.equal(
      await page.getByRole("button", { name: "Run", exact: true }).isEnabled(),
      snapshot.state === "READY",
    );
    for (const name of ["Pause", "Stop"])
      assert.equal(
        await page.getByRole("button", { name, exact: true }).isDisabled(),
        true,
      );
  };
  assert.equal(
    await page.getByText("No process scene attached", { exact: true }).count(),
    1,
  );
  await page.getByRole("button", { name: "Use Example", exact: true }).click();
  const confirmation = page.getByRole("alertdialog", {
    name: "Replace Structured Text source",
  });
  await confirmation.waitFor();
  assert.equal(
    await page.getByTestId("compile-status").innerText(),
    "COMPILE SUCCESS",
  );
  await confirmation
    .getByRole("button", { name: "Replace Source", exact: true })
    .click();
  await page
    .getByTestId("compile-status")
    .getByText("NOT COMPILED", { exact: true })
    .waitFor();
  assert.equal(
    await page
      .getByRole("button", { name: "Attach Tank Scene", exact: true })
      .isDisabled(),
    true,
  );
  assert.equal(
    await page
      .getByRole("button", { name: "Single Scan", exact: true })
      .isDisabled(),
    true,
  );
  assert.equal(
    await page.getByTestId("runtime-status").getAttribute("data-session-id"),
    null,
  );
  await compileButton.click();
  await page
    .getByTestId("compile-status")
    .getByText("COMPILE SUCCESS", { exact: true })
    .waitFor();
  const initialLoad = await mutation(() =>
    page.getByRole("button", { name: "Load Program", exact: true }).click(),
  );
  assert.equal(initialLoad.process, null);
  const attached = await mutation(() =>
    page
      .getByRole("button", { name: "Attach Tank Scene", exact: true })
      .click(),
  );
  await visible(attached);
  assert.equal(attached.process.levelPermille, 0);
  assert.equal(attached.process.lowLevel, true);
  assert.equal(attached.process.highLevel, false);
  assert.equal(attached.completedScans, "0");
  assert.equal(attached.simulationTimeMicros, "0");
  const low = attached.variables.find((v) => v.name === "LevelLow");
  const ownedInputRejected = await page.evaluate(
    async ({ id, variableId, version }) => {
      try {
        await window.__TAURI_INTERNALS__.invoke("set_runtime_input", {
          request: {
            contractVersion: version,
            sessionId: id,
            variableId,
            value: { type: "BOOL", value: false },
          },
        });
        return null;
      } catch (error) {
        return error;
      }
    },
    {
      id: attached.sessionId,
      variableId: low.variableId,
      version: CONTRACT_VERSION,
    },
  );
  assert.equal(ownedInputRejected.code, "PLC-PROCESS-004");
  assert.deepEqual(await observe(attached.sessionId), attached);
  const sequence = [];
  let s = attached;
  for (const [index, level] of [
    100, 200, 300, 400, 500, 600, 700, 800, 800,
  ].entries()) {
    s = await mutation(() =>
      page.getByRole("button", { name: "Single Scan", exact: true }).click(),
    );
    await visible(s);
    const n = index + 1;
    assert.equal(s.process.levelPermille, level);
    assert.equal(s.process.highLevel, n >= 8);
    assert.equal(s.process.lowLevel, n <= 2);
    assert.equal(s.process.inletValveOpen, n <= 8);
    assert.equal(
      s.variables.find((v) => v.name === "InletValve").value.value,
      n <= 8,
    );
    assert.equal(
      s.variables.find((v) => v.name === "LevelHigh").value.value,
      n >= 8,
    );
    assert.equal(
      s.variables.find((v) => v.name === "LevelPercent").value.value,
      level / 10,
    );
    assert.equal(s.completedScans, String(n));
    assert.equal(s.simulationTimeMicros, String(n * 100_000));
    sequence.push({
      scan: s.completedScans,
      timeMicros: s.simulationTimeMicros,
      ...s.process,
    });
    if (n === 8) {
      await page
        .getByRole("region", { name: "Process", exact: true })
        .scrollIntoViewIfNeeded();
      const bounds = await page
        .getByRole("img", { name: "Tank Filling: 80.0 percent" })
        .boundingBox();
      assert.ok(
        bounds && bounds.width >= 250 && bounds.height >= 100,
        "Tank diagram is unreadable at the native window size.",
      );
      const panel = await page
        .getByRole("region", { name: "Process", exact: true })
        .boundingBox();
      const timeline = await page
        .getByRole("region", { name: "Timeline", exact: true })
        .boundingBox();
      assert.ok(
        panel && timeline && panel.y + panel.height <= timeline.y,
        "Process panel overlaps the next workspace row.",
      );
      assert.ok(
        bounds.y >= panel.y &&
          bounds.y + bounds.height <= panel.y + panel.height,
        "Tank graphic is clipped inside the Process panel.",
      );
      const viewport = await page.evaluate(() => ({
        height: window.innerHeight,
        top: document.querySelector(".toolbar").getBoundingClientRect().bottom,
        bottom: document.querySelector(".statusbar").getBoundingClientRect()
          .top,
      }));
      assert.ok(
        bounds.y >= viewport.top && bounds.y + bounds.height <= viewport.bottom,
        "Tank graphic is outside the usable native viewport.",
      );
      for (const name of ["LevelLow", "LevelHigh", "LevelPercent"]) {
        const fullyShown = await page
          .getByTestId(`runtime-variable-${name}`)
          .locator("span")
          .first()
          .evaluate((node) => node.scrollWidth <= node.clientWidth);
        assert.equal(
          fullyShown,
          true,
          `Sensor name ${name} is truncated at the native window size.`,
        );
      }
      await page.screenshot({ path: path.join(directory, "tank-filling.png") });
    }
  }
  const traceHistory = await exerciseTraceHistory(
    page,
    s,
    directory,
    CONTRACT_VERSION,
  );
  const runtimeReset = await mutation(() =>
    page.getByRole("button", { name: "Reset Runtime", exact: true }).click(),
  );
  await visible(runtimeReset);
  assert.deepEqual(runtimeReset.process, s.process);
  assert.equal(runtimeReset.completedScans, "0");
  assert.equal(runtimeReset.simulationTimeMicros, "0");
  assert.equal(
    runtimeReset.variables.find((v) => v.name === "LevelHigh").value.value,
    true,
  );
  const afterRuntimeResetScan = await mutation(() =>
    page.getByRole("button", { name: "Single Scan", exact: true }).click(),
  );
  await visible(afterRuntimeResetScan);
  assert.equal(afterRuntimeResetScan.process.levelPermille, 800);
  assert.equal(afterRuntimeResetScan.process.inletValveOpen, false);
  const processReset = await mutation(() =>
    page.getByRole("button", { name: "Reset Process", exact: true }).click(),
  );
  await visible(processReset);
  assert.equal(processReset.process.levelPermille, 0);
  assert.equal(processReset.process.lowLevel, true);
  assert.equal(processReset.process.highLevel, false);
  assert.equal(
    processReset.completedScans,
    afterRuntimeResetScan.completedScans,
  );
  assert.equal(
    processReset.simulationTimeMicros,
    afterRuntimeResetScan.simulationTimeMicros,
  );
  assert.deepEqual(
    processReset.variables.filter((v) => v.role !== "Input"),
    afterRuntimeResetScan.variables.filter((v) => v.role !== "Input"),
  );
  const afterProcessResetScan = await mutation(() =>
    page.getByRole("button", { name: "Single Scan", exact: true }).click(),
  );
  await visible(afterProcessResetScan);
  assert.equal(afterProcessResetScan.process.levelPermille, 100);
  assert.equal(afterProcessResetScan.completedScans, "2");
  const detached = await mutation(() =>
    page.getByRole("button", { name: "Detach Process", exact: true }).click(),
  );
  await visible(detached);
  assert.equal(detached.process, null);
  assert.equal(detached.completedScans, afterProcessResetScan.completedScans);
  assert.equal(
    detached.simulationTimeMicros,
    afterProcessResetScan.simulationTimeMicros,
  );
  for (const name of ["LevelLow", "LevelHigh", "LevelPercent"]) {
    assert.equal(
      await page.getByLabel(`Authority ${name}`, { exact: true }).innerText(),
      "USER",
    );
    assert.equal(
      await page.getByLabel(`Set ${name}`, { exact: true }).isEnabled(),
      true,
    );
  }
  const pure = await mutation(() =>
    page.getByRole("button", { name: "Single Scan", exact: true }).click(),
  );
  assert.equal(pure.process, null);
  assert.equal(pure.completedScans, "3");
  const reattached = await mutation(() =>
    page
      .getByRole("button", { name: "Attach Tank Scene", exact: true })
      .click(),
  );
  await visible(reattached);
  const example = await readFile(
    path.join(root, "scenes/tank-filling/example.st"),
    "utf8",
  );
  await editSource(example.replace("PROGRAM Main", "PROGRAM Edited"));
  assert.equal(
    await page
      .getByRole("button", { name: "Single Scan", exact: true })
      .isDisabled(),
    true,
  );
  assert.equal(
    await page
      .getByRole("button", { name: "Reset Process", exact: true })
      .isDisabled(),
    true,
  );
  assert.equal(
    await page
      .getByRole("button", { name: "Detach Process", exact: true })
      .isDisabled(),
    true,
  );
  assert.match(await page.getByTestId("tank-scene").innerText(), /STALE/);
  assert.deepEqual(await observe(reattached.sessionId), reattached);
  await compileButton.click();
  await page
    .getByTestId("compile-status")
    .getByText("COMPILE SUCCESS", { exact: true })
    .waitFor();
  const reloaded = await mutation(() =>
    page.getByRole("button", { name: "Load Program", exact: true }).click(),
  );
  assert.equal(reloaded.process, null);
  assert.notEqual(reloaded.sessionId, reattached.sessionId);
  await page.getByRole("button", { name: "Unload", exact: true }).click();
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
    initialLoad,
    attached,
    sequence,
    traceHistory,
    runtimeReset,
    afterRuntimeResetScan,
    processReset,
    afterProcessResetScan,
    detached,
    ownedInputRejected: ownedInputRejected.code,
    sourceEditDisablesProcess: true,
    reloadDropsScene: true,
    realBackendResponsesCorrelated: true,
  };
}
