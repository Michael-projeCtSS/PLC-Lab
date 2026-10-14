import assert from "node:assert/strict";
import path from "node:path";

/** Actual native history inspection; no transport or simulation mocks. */
export async function exerciseTraceHistory(
  page,
  live,
  directory,
  contractVersion,
) {
  const query = () =>
    page.evaluate(
      async (request) =>
        window.__TAURI_INTERNALS__.invoke("get_trace_page", { request }),
      {
        contractVersion,
        sessionId: live.sessionId,
        afterSequence: null,
        limit: 256,
      },
    );
  const history = await query();
  assert.equal(history.contractVersion, contractVersion);
  assert.equal(history.sessionId, live.sessionId);
  assert.equal(history.entries.length, 11);
  assert.deepEqual(
    history.entries.map((e) => e.kind),
    ["SessionLoaded", "ProcessAttached", ...Array(9).fill("ScanCompleted")],
  );
  assert.deepEqual(
    history.entries.map((e) => e.sequence),
    Array.from({ length: 11 }, (_, i) => String(i + 1)),
  );
  await page.waitForFunction((id) => {
    const panel = document.querySelector('[data-testid="session-timeline"]');
    return (
      panel?.dataset.traceSession === id &&
      panel.dataset.tracePending === "false" &&
      panel.querySelectorAll("[data-trace-sequence]").length === 11
    );
  }, live.sessionId);
  const selected = [];
  for (const [sequence, scan, time, inlet] of [
    ["10", "8", "800000", true],
    ["11", "9", "900000", false],
  ]) {
    const event = history.entries.find((e) => e.sequence === sequence);
    assert.equal(event.completedScans, scan);
    assert.equal(event.simulationTimeMicros, time);
    assert.equal(event.process.levelPermille, 800);
    assert.equal(event.process.highLevel, true);
    assert.equal(event.process.inletValveOpen, inlet);
    await page.locator(`[data-trace-sequence="${sequence}"]`).click();
    const inspector = page.getByTestId("trace-inspector");
    await inspector.scrollIntoViewIfNeeded();
    assert.equal(
      await inspector.getAttribute("data-selected-sequence"),
      sequence,
    );
    assert.match(
      await inspector.innerText(),
      new RegExp(`Historical Trace · #${sequence}`),
    );
    assert.match(
      await inspector.innerText(),
      new RegExp(`Scan ${scan} · ${BigInt(time) / 1000n} ms`),
    );
    const tank = page.getByTestId("historical-tank");
    assert.deepEqual(await tank.locator("dd").allTextContents(), [
      "80.0 %",
      "INACTIVE",
      "ACTIVE",
      inlet ? "OPEN" : "CLOSED",
      "STOPPED",
    ]);
    const observation = await page.evaluate(
      async (request) =>
        window.__TAURI_INTERNALS__.invoke("get_runtime_snapshot", { request }),
      { contractVersion, sessionId: live.sessionId },
    );
    assert.deepEqual(
      observation,
      live,
      "Selecting history mutated live runtime/process state.",
    );
    assert.deepEqual(
      await query(),
      history,
      "Read/selection polluted history.",
    );
    await tank.scrollIntoViewIfNeeded();
    const bounds = await tank.boundingBox();
    const viewport = await page.evaluate(() => ({
      top: document.querySelector(".toolbar").getBoundingClientRect().bottom,
      bottom: document.querySelector(".statusbar").getBoundingClientRect().top,
    }));
    assert.ok(
      bounds &&
        bounds.y >= viewport.top &&
        bounds.y + bounds.height <= viewport.bottom,
      "Historical Tank observation is not readable in the native viewport.",
    );
    await page.screenshot({
      path: path.join(directory, `timeline-scan-${scan}.png`),
    });
    selected.push(event);
  }
  for (const label of ["Run", "Pause", "Stop"])
    assert.equal(
      await page.getByRole("button", { name: label, exact: true }).isDisabled(),
      true,
    );
  return {
    result: "PASS",
    realRustTracePage: history,
    selectedHistoricalEntries: selected,
    realBackendResponsesCorrelated: true,
    selectionPreservesLiveState: true,
    selectionAndQueryAppendNothing: true,
  };
}
