import assert from "node:assert/strict";
import path from "node:path";
import { CONTRACT_VERSION } from "../../packages/ipc/src/generated.ts";

/** Exercise real UI mutation IPC and correlate it with coherent Rust observations. */
export async function interactiveRuntimeSmoke({
  page,
  editSource,
  compileButton,
  tonSource,
  defaultSource,
  directory,
}) {
  // Correlate each UI mutation with one real, targeted backend observation.
  // The observation token is presentation metadata, never PLC scan/time state.
  const mutation = async (command, action) => {
    const status = page.getByTestId("runtime-status");
    const before = await status.getAttribute("data-observation-key");
    const previousSession = await status.getAttribute("data-session-id");
    await action();
    await page.waitForFunction((before) => {
      const node = document.querySelector('[data-testid="runtime-status"]');
      return (
        node.dataset.observationKey !== before &&
        node.dataset.runtimePending === ""
      );
    }, before);
    if (command === "unload_program") {
      const error = await page.evaluate(
        async ({ id, version }) => {
          try {
            await window.__TAURI_INTERNALS__.invoke("get_runtime_snapshot", {
              request: { contractVersion: version, sessionId: id },
            });
            return null;
          } catch (error) {
            return error;
          }
        },
        { id: previousSession, version: CONTRACT_VERSION },
      );
      assert.equal(error.code, "APP-RUNTIME-STALE-SESSION");
      return { sessionId: previousSession, observationError: error };
    }
    const id = await status.getAttribute("data-session-id");
    assert.ok(
      id,
      "A loaded session must identify the authoritative observation.",
    );
    return observe(id);
  };
  const disabledContinuous = async () => {
    for (const name of ["Run", "Pause", "Stop"])
      assert.equal(
        await page.getByRole("button", { name, exact: true }).isDisabled(),
        true,
      );
  };
  const shownValue = (value) =>
    value.type === "BOOL"
      ? value.value
        ? "TRUE"
        : "FALSE"
      : value.type === "TIME"
        ? BigInt(value.microseconds) % 1000n === 0n
          ? `T#${BigInt(value.microseconds) / 1000n}ms`
          : `T#${value.microseconds}us`
        : String(value.value);
  const visible = async (snapshot) => {
    assert.equal(snapshot.contractVersion, CONTRACT_VERSION);
    await page.waitForFunction((snapshot) => {
      const count = document.querySelector(
        '[data-testid="runtime-scan-count"]',
      );
      return (
        count?.textContent === snapshot.completedScans &&
        document.querySelector('[data-testid="runtime-state"]')?.textContent ===
          snapshot.state
      );
    }, snapshot);
    const micros = BigInt(snapshot.simulationTimeMicros);
    const time =
      micros % 1000n === 0n ? `${micros / 1000n} ms` : `${micros} µs`;
    assert.equal(await page.getByTestId("runtime-time").innerText(), time);
    for (const variable of snapshot.variables)
      assert.equal(
        await page
          .getByLabel(`Value ${variable.name}`, { exact: true })
          .innerText(),
        shownValue(variable.value),
      );
    for (const block of snapshot.functionBlocks) {
      assert.equal(
        await page.getByLabel(`Q ${block.name}`, { exact: true }).innerText(),
        `Q = ${block.q ? "TRUE" : "FALSE"}`,
      );
      assert.equal(
        await page.getByLabel(`ET ${block.name}`, { exact: true }).innerText(),
        `ET = ${shownValue({ type: "TIME", microseconds: block.etMicros })}`,
      );
    }
    await disabledContinuous();
  };
  const loaded = async () => {
    const snapshot = await mutation("load_st_program", () =>
      page.getByRole("button", { name: "Load Program", exact: true }).click(),
    );
    await visible(snapshot);
    assert.equal(
      await page
        .getByRole("button", { name: "Single Scan", exact: true })
        .isEnabled(),
      true,
    );
    return snapshot;
  };
  const scan = async () => {
    const response = await mutation("single_scan", () =>
      page.getByRole("button", { name: "Single Scan", exact: true }).click(),
    );
    await visible(response);
    return response;
  };
  const compileLoad = async (source) => {
    await editSource(source);
    assert.equal(
      await page
        .getByRole("button", { name: "Single Scan", exact: true })
        .isDisabled(),
      true,
    );
    await compileButton.click();
    await page
      .getByTestId("compile-status")
      .getByText("COMPILE SUCCESS", { exact: true })
      .waitFor();
    return loaded();
  };
  const stageStart = async () => {
    const response = await mutation("set_runtime_input", () =>
      page.getByLabel("Set Start", { exact: true }).check(),
    );
    await visible(response);
    assert.equal(
      response.variables.find((v) => v.name === "Start").value.value,
      true,
    );
    return response;
  };
  const observe = async (id) =>
    page.evaluate(
      ({ id, version }) =>
        window.__TAURI_INTERNALS__.invoke("get_runtime_snapshot", {
          request: { contractVersion: version, sessionId: id },
        }),
      { id, version: CONTRACT_VERSION },
    );

  const initial = await loaded();
  assert.equal(initial.completedScans, "0");
  assert.equal(initial.simulationTimeMicros, "0");
  assert.equal(initial.cyclePeriodMicros, "100000");
  assert.equal(initial.operationBudget, "10000");
  const staged = await stageStart();
  assert.equal(
    staged.variables.find((v) => v.name === "Motor").value.value,
    false,
  );
  const defaultScan = await scan();
  assert.equal(
    defaultScan.variables.find((v) => v.name === "Motor").value.value,
    true,
  );
  assert.deepEqual(await observe(defaultScan.sessionId), defaultScan);

  const readAfterWriteSource =
    "PROGRAM Main\nVAR_INPUT Start : BOOL; END_VAR\nVAR_OUTPUT Motor : BOOL; END_VAR\nVAR Internal : BOOL; END_VAR\nInternal := Start;\nMotor := Internal;\nEND_PROGRAM";
  await editSource(readAfterWriteSource);
  assert.match(await page.getByTestId("runtime-status").innerText(), /STALE/);
  assert.equal(
    await page
      .getByRole("button", { name: "Single Scan", exact: true })
      .isDisabled(),
    true,
  );
  await compileButton.click();
  await page
    .getByTestId("compile-status")
    .getByText("COMPILE SUCCESS", { exact: true })
    .waitFor();
  const rwLoad = await loaded();
  assert.notEqual(rwLoad.sessionId, initial.sessionId);
  await stageStart();
  const rw = await scan();
  for (const name of ["Internal", "Motor"])
    assert.equal(rw.variables.find((v) => v.name === name).value.value, true);
  assert.equal(rw.completedScans, "1");
  assert.equal(rw.simulationTimeMicros, "100000");

  const tonInitial = await compileLoad(tonSource);
  assert.equal(
    tonInitial.variables.find((v) => v.name === "Motor").value.value,
    false,
  );
  assert.equal(tonInitial.functionBlocks[0].q, false);
  assert.equal(tonInitial.functionBlocks[0].etMicros, "0");
  await stageStart();
  const tonSequence = [];
  const tonSnapshots = [];
  for (const expected of [
    { scan: "1", time: "100000", q: false, et: "0" },
    { scan: "2", time: "200000", q: false, et: "100000" },
    { scan: "3", time: "300000", q: false, et: "200000" },
    { scan: "4", time: "400000", q: true, et: "300000" },
  ]) {
    const result = await scan();
    const block = result.functionBlocks[0];
    assert.equal(result.completedScans, expected.scan);
    assert.equal(result.simulationTimeMicros, expected.time);
    assert.equal(block.q, expected.q);
    assert.equal(block.etMicros, expected.et);
    assert.equal(
      result.variables.find((v) => v.name === "Motor").value.value,
      expected.q,
    );
    tonSnapshots.push(result);
    tonSequence.push({
      completedScans: result.completedScans,
      simulationTimeMicros: result.simulationTimeMicros,
      motor: result.variables.find((v) => v.name === "Motor").value.value,
      q: block.q,
      etMicros: block.etMicros,
    });
  }
  assert.deepEqual(await observe(tonInitial.sessionId), tonSnapshots.at(-1));
  await page.getByTestId("runtime-fb-Delay").scrollIntoViewIfNeeded();
  await page.screenshot({ path: path.join(directory, "runtime-ton.png") });
  const faultSource =
    "PROGRAM Main\nVAR_INPUT Divisor : INT; END_VAR\nVAR_OUTPUT Result : INT; END_VAR\nResult := 100 / Divisor;\nEND_PROGRAM";
  const faultInitial = await compileLoad(faultSource);
  const fault = await scan();
  assert.equal(fault.state, "FAULTED");
  assert.equal(fault.fault.code, "PLC-RUNTIME-003");
  assert.equal(fault.fault.category, "runtimeFault");
  assert.equal(fault.completedScans, "0");
  assert.equal(fault.simulationTimeMicros, "0");
  assert.deepEqual(fault.variables, faultInitial.variables);
  assert.deepEqual(fault.fault.span, {
    start: { line: 4, column: 11 },
    end: { line: 4, column: 24 },
  });
  assert.equal(
    await page
      .getByRole("button", { name: "Single Scan", exact: true })
      .isDisabled(),
    true,
  );
  const diagnostic = page
    .getByTestId("runtime-diagnostics")
    .getByRole("button");
  await diagnostic.getByText(/PLC-RUNTIME-003/).waitFor();
  await diagnostic.click();
  await page
    .locator('[data-testid="st-editor"] .squiggly-error')
    .first()
    .waitFor();
  const marker = await page
    .locator('[data-testid="st-editor"] .squiggly-error')
    .first()
    .boundingBox();
  const expression = await page
    .locator('[data-testid="st-editor"] .view-lines')
    .evaluate((lines) => {
      for (const line of lines.querySelectorAll(".view-line")) {
        const raw = line.textContent.replace(/\u00a0/g, " ");
        const start = raw.indexOf("100 / Divisor");
        if (start < 0) continue;
        const walker = document.createTreeWalker(line, NodeFilter.SHOW_TEXT);
        let offset = 0;
        let a, b;
        for (let node = walker.nextNode(); node; node = walker.nextNode()) {
          const end = offset + node.textContent.length;
          if (!a && start >= offset && start < end)
            a = { node, index: start - offset };
          if (start + 13 > offset && start + 13 <= end)
            b = { node, index: start + 13 - offset };
          offset = end;
        }
        if (a && b) {
          const range = document.createRange();
          range.setStart(a.node, a.index);
          range.setEnd(b.node, b.index);
          const r = range.getBoundingClientRect();
          return { x: r.x, y: r.y, width: r.width, height: r.height };
        }
      }
      return null;
    });
  assert.ok(
    marker && expression,
    "Runtime fault expression and marker must be rendered.",
  );
  assert.ok(
    Math.abs(marker.x - expression.x) <= 2,
    "Runtime marker starts at the wrong source position.",
  );
  assert.ok(
    Math.abs(marker.width - expression.width) <= 2,
    "Runtime marker covers the wrong expression.",
  );
  assert.ok(
    marker.y < expression.y + expression.height &&
      marker.y + marker.height > expression.y,
    "Runtime marker is on the wrong line.",
  );
  await page.getByTestId("runtime-variable-Result").scrollIntoViewIfNeeded();
  await page.screenshot({ path: path.join(directory, "runtime-fault.png") });
  const reset = await mutation("reset_runtime", () =>
    page.getByRole("button", { name: "Reset Runtime", exact: true }).click(),
  );
  await visible(reset);
  assert.deepEqual(reset, faultInitial);
  assert.equal(
    await page.locator('[data-testid="st-editor"] .squiggly-error').count(),
    0,
  );
  assert.equal(
    await page
      .getByRole("button", { name: "Single Scan", exact: true })
      .isEnabled(),
    true,
  );

  const typedSource =
    "PROGRAM Main\nVAR_INPUT Amount : INT; SetPoint : REAL; Duration : TIME; END_VAR\nVAR_OUTPUT AmountOut : INT; RealOut : REAL; TimeOut : TIME; END_VAR\nAmountOut := Amount; RealOut := SetPoint; TimeOut := Duration;\nEND_PROGRAM";
  await compileLoad(typedSource);
  for (const [name, label, value] of [
    ["Amount", "Set Amount", "-32768"],
    ["SetPoint", "Set SetPoint", "1.5"],
    ["Duration", "Set Duration (microseconds)", "18446744073709551615"],
  ]) {
    await page.getByLabel(label, { exact: true }).fill(value);
    const staged = await mutation("set_runtime_input", () =>
      page.getByRole("button", { name: `Apply ${name}`, exact: true }).click(),
    );
    await visible(staged);
  }
  const typed = await scan();
  assert.deepEqual(typed.variables.find((v) => v.name === "TimeOut").value, {
    type: "TIME",
    microseconds: "18446744073709551615",
  });
  assert.equal(
    typed.variables.find((v) => v.name === "AmountOut").value.value,
    -32768,
  );
  assert.equal(
    typed.variables.find((v) => v.name === "RealOut").value.value,
    1.5,
  );
  const unload = await mutation("unload_program", () =>
    page.getByRole("button", { name: "Unload", exact: true }).click(),
  );
  assert.equal(unload.sessionId, typed.sessionId);
  assert.equal(await page.getByTestId("runtime-variables").count(), 0);
  assert.equal(
    await page
      .getByRole("button", { name: "Single Scan", exact: true })
      .isDisabled(),
    true,
  );
  await editSource(defaultSource);
  await compileButton.click();
  await page
    .getByTestId("compile-status")
    .getByText("COMPILE SUCCESS", { exact: true })
    .waitFor();
  await disabledContinuous();
  return {
    initialLoad: initial,
    defaultScan,
    readAfterWrite: rw,
    tonInitial,
    tonSequence,
    fault,
    reset,
    typedValues: typed,
    sourceEditDisablesScan: true,
    runtimeFaultMarkerCorrect: true,
    unloadRemovesValues: true,
    continuousControlsDisabled: true,
    realBackendResponsesCorrelated: true,
  };
}
