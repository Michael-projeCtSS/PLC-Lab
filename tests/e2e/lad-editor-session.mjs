// Native acceptance of the visual ladder editor (CP9-E, ADR-0052) in the real desktop
// application: Tauri window, WebView2 and the Rust backend, with real mouse and keyboard
// input. Every topology check reads the ladder document the application saved to disk;
// behavior is the real PLC runtime driving the real process model; nothing is mocked.
//
// A: a contact dragged onto an empty rung, a second one dropped to its right: A AND B.
// B: a contact dropped below A of A AND B: (A OR C) AND B, branch wired automatically.
// C: operands typed on the contacts with completion chosen by keyboard.
// D: TON typed at an insertion point, instance and PT edited on the box, timer online.
// E: an existing contact dragged onto a new branch, undo, redo, save and reopen.
// F: an invalid drop is explained and changes nothing; Esc cancels a drag.
// G: a rung built with the keyboard only, including a confirmed new variable.
// The LAD program's truth table over all 8 input combinations is compared with the same
// logic written in Structured Text and run in the same native session. Screenshots at
// 1366x768, 1920x1080 and 2560x1440, English and Polish, dark and light, with a layout
// check for overlapping or clipped ladder elements.
//
// Usage: node tests/e2e/lad-editor-session.mjs [--exe target/debug/plc-lab-desktop.exe]
// Without --exe it starts TEST_PLC_LAB.bat, as a user does.
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { mkdir, readFile, readdir, rm } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { chromium } from "playwright-core";
import { root } from "../../scripts/toolchain.mjs";
import { editorText } from "./monaco-text.mjs";

if (process.platform !== "win32")
  throw new Error("The LAD editor session check requires Windows.");
const directory = path.join(root, "artifacts", "lad-editor-session");
await mkdir(directory, { recursive: true });
const port = await new Promise((resolve) => {
  const server = net.createServer();
  server.listen(0, "127.0.0.1", () => {
    const { port } = server.address();
    server.close(() => resolve(port));
  });
});
const exeIndex = process.argv.indexOf("--exe");
const exe =
  exeIndex > 0 && process.argv[exeIndex + 1]
    ? path.resolve(root, process.argv[exeIndex + 1])
    : null;
const stamp = Date.now();
const dataDir = path.join(directory, `data-${stamp}`);
const environment = {
  ...process.env,
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
  WEBVIEW2_USER_DATA_FOLDER: path.join(directory, `webview-${stamp}`),
  PLC_LAB_DATA_DIR: dataDir,
  PLC_LAB_NO_PAUSE: "1",
  PLC_LAB_NO_DIALOG: "1",
};
const launcher = exe ? path.basename(exe) : "TEST_PLC_LAB.bat";
const bat = path.join(root, "TEST_PLC_LAB.bat");
const session = exe
  ? spawn(exe, [], {
      cwd: path.join(root, "apps"),
      env: environment,
      stdio: ["ignore", "pipe", "pipe"],
    })
  : spawn(process.env.ComSpec ?? "cmd.exe", ["/d", "/s", "/c", `""${bat}""`], {
      cwd: path.join(root, "apps"),
      env: environment,
      stdio: ["ignore", "pipe", "pipe"],
      windowsVerbatimArguments: true,
    });
let output = "";
for (const stream of [session.stdout, session.stderr])
  stream.on("data", (chunk) => (output += chunk));
let exited = null;
session.on("exit", (code) => (exited = code ?? 1));

const passed = [];
const evidence = {};
let browser;
let page;
/** One DevTools session for the viewport emulation of the whole run. */
let cdp;
const pageErrors = [];
async function check(name, run) {
  await run();
  assert.deepEqual(pageErrors, [], `${name}: uncaught JavaScript errors`);
  const shell = await page.evaluate(() => ({
    children: document.getElementById("root")?.childElementCount ?? 0,
    nav: !!document.querySelector("nav"),
  }));
  assert.ok(shell.children > 0 && shell.nav, `${name}: the window went blank`);
  passed.push(name);
  console.log(`CHECK PASS: ${name}`);
}
async function until(read, accept, what, timeout = 15_000) {
  const deadline = Date.now() + timeout;
  let value;
  while (Date.now() < deadline) {
    value = await read();
    if (accept(value)) return value;
    await delay(150);
  }
  throw new Error(`${what}: saw ${JSON.stringify(value)}`);
}

/** The boolean expression of a ladder network, as ADR-0052 previews it. */
function expression(node, nested = false) {
  switch (node.type) {
    case "contact": {
      const v = node.variable || "?";
      if (node.contact === "normallyClosed") return `NOT ${v}`;
      if (node.contact === "positiveEdge") return `P(${v})`;
      if (node.contact === "negativeEdge") return `N(${v})`;
      return v;
    }
    case "compare":
      return `CMP(${node.op})`;
    case "block":
      return `${node.block}(${node.instance})`;
    case "series": {
      if (!node.items.length) return "TRUE";
      const text = node.items.map((c) => expression(c, true)).join(" AND ");
      return nested && node.items.length > 1 ? `(${text})` : text;
    }
    case "parallel": {
      const text = node.branches.map((c) => expression(c, true)).join(" OR ");
      return nested ? `(${text})` : text;
    }
  }
}
/** The ladder document of the project as the application saved it on disk. */
async function savedLadder() {
  const files = [];
  const walk = async (dir) => {
    for (const entry of await readdir(dir, { withFileTypes: true }).catch(
      () => [],
    )) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) await walk(full);
      else if (entry.name.endsWith(".plclab")) files.push(full);
    }
  };
  await walk(dataDir);
  for (const file of files) {
    const project = JSON.parse(await readFile(file, "utf8"));
    if (project.name !== "Ladder editor") continue;
    const main = project.programs?.[0];
    return main?.language === "LAD" ? JSON.parse(main.source) : null;
  }
  return null;
}
const rungLogic = async (index) => {
  const doc = await savedLadder();
  const rung = doc?.rungs?.[index];
  return rung ? expression(rung.network) : null;
};
const waitLogic = (index, expected) =>
  until(
    () => rungLogic(index),
    (v) => v === expected,
    `rung ${index + 1} should be ${expected}`,
  );

try {
  await check(`${launcher} opens the native window`, async () => {
    const deadline = Date.now() + 15 * 60_000;
    for (;;) {
      if (exited !== null)
        throw new Error(`${launcher} ended (exit ${exited}) first: ${output}`);
      try {
        const r = await fetch(`http://127.0.0.1:${port}/json/version`, {
          signal: AbortSignal.timeout(1000),
        });
        if (r.ok) break;
      } catch {
        /* the window is not there yet */
      }
      if (Date.now() > deadline) throw new Error("No native window");
      await delay(500);
    }
    browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
    for (let i = 0; i < 100 && !page; i++) {
      page = browser.contexts().flatMap((c) => c.pages())[0];
      if (!page) await delay(100);
    }
    assert.ok(page, "no WebView2 page");
    await page.getByRole("navigation").first().waitFor({ timeout: 120_000 });
    await page.waitForLoadState("load");
    page.on("pageerror", (e) => pageErrors.push(e.stack ?? String(e)));
    page.on("console", (m) => {
      if (m.type() === "error") pageErrors.push(`console.error: ${m.text()}`);
    });
    await page.evaluate(() =>
      localStorage.setItem(
        "plc-lab.interface.v1",
        JSON.stringify({
          version: 1,
          language: "en",
          theme: "dark",
          density: "comfortable",
          explanations: true,
        }),
      ),
    );
    await page.reload();
    await page.getByRole("navigation").first().waitFor();
    // Room for several rungs next to the docks, as on a desktop monitor.
    cdp = await page.context().newCDPSession(page);
    await cdp.send("Emulation.setDeviceMetricsOverride", {
      width: 1920,
      height: 1080,
      deviceScaleFactor: 1,
      mobile: false,
    });
  });

  const views = () => page.getByRole("tablist", { name: "Workspace views" });
  const tab = (name) => views().getByRole("tab", { name, exact: true }).click();
  const ladder = () => page.getByTestId("lad-editor");
  const rung = (n) =>
    ladder().getByRole("listitem", { name: `Rung ${n}`, exact: true });
  const element = (n, name) =>
    rung(n).getByRole("button", { name, exact: true }).first();
  const palette = (name) =>
    page
      .getByRole("toolbar", { name: "Ladder tools" })
      .getByRole("button", { name, exact: true });
  const combobox = () => page.locator(".lad-inline-input");
  const problems = () => page.locator(".compile-problems");
  const notice = () => page.locator(".lad-notice");
  const box = async (locator) => {
    const b = await locator.boundingBox();
    assert.ok(b, "element has no box");
    return b;
  };
  /** A real mouse drag: press, move in steps, release. */
  const mouseDrag = async (from, to, { hold = false } = {}) => {
    // Record what the page receives, to explain a drag that does not start.
    await page.evaluate(() => {
      window.__drag = [];
      if (window.__dragRecorder) return;
      window.__dragRecorder = true;
      for (const type of [
        "pointerdown",
        "pointermove",
        "pointerup",
        "pointercancel",
        "mousedown",
        "mouseup",
        "dragstart",
        "blur",
        "focus",
      ])
        window.addEventListener(
          type,
          (e) => {
            const t = e.target;
            const name =
              t === window
                ? "window"
                : (t?.getAttribute?.("aria-label") ??
                  t?.className ??
                  String(t));
            window.__drag.push(
              `${type}@${Math.round(e.clientX ?? -1)},${Math.round(e.clientY ?? -1)}:${String(name).slice(0, 40)}`,
            );
          },
          true,
        );
    });
    await page.mouse.move(from.x, from.y);
    await page.mouse.down();
    await page.mouse.move(from.x + 12, from.y + 12, { steps: 3 });
    await page.mouse.move(to.x, to.y, { steps: 12 });
    await delay(150);
    const during = await page.evaluate(
      ({ x, y }) => ({
        ghost: document.querySelectorAll(".lad-ghost").length,
        notice: document.querySelector(".lad-notice")?.textContent ?? null,
        under: document.elementFromPoint(x, y)?.className ?? null,
        events: [...new Set(window.__drag)].slice(0, 40),
        count: window.__drag.length,
      }),
      to,
    );
    evidence.lastDrag = { from, to, ...during };
    if (!during.ghost)
      throw new Error(
        `no drag in progress: ${JSON.stringify(evidence.lastDrag)}`,
      );
    if (!hold) await page.mouse.up();
  };
  const center = (b) => ({ x: b.x + b.width / 2, y: b.y + b.height / 2 });
  /** A drop point on `locator`, after scrolling it to the middle of the rung area. */
  const spot = async (locator, where = "center") => {
    await locator.evaluate((el) =>
      el.scrollIntoView({ block: "center", inline: "nearest" }),
    );
    await delay(150);
    const b = await box(locator);
    return where === "right"
      ? rightOf(b)
      : where === "below"
        ? below(b)
        : center(b);
  };
  const rightOf = (b) => ({ x: b.x + b.width * 0.8, y: b.y + b.height / 2 });
  const below = (b) => ({ x: b.x + b.width / 2, y: b.y + b.height + 7 });
  /** Type into the open operand editor and accept the first suggestion. */
  const choose = async (text, expectOptions = null) => {
    const input = combobox();
    await input.waitFor({ timeout: 10_000 });
    await page.keyboard.type(text);
    const list = page.getByRole("listbox", { name: "Suggestions" });
    if (expectOptions) {
      const options = await until(
        () => list.getByRole("option").allInnerTexts(),
        (o) => expectOptions.every((x) => o.some((t) => t.includes(x))),
        `suggestions for ${text}`,
      );
      evidence[`suggestions:${text}`] = options;
    }
    await page.keyboard.press("Enter");
    await input.waitFor({ state: "detached", timeout: 10_000 });
  };

  await check("a Free Lab project switches to Ladder Diagram", async () => {
    await page.getByRole("button", { name: "FREE LAB", exact: true }).click();
    await page.getByRole("button", { name: "New project" }).first().click();
    await page.getByLabel("Project name").fill("Ladder editor");
    await page.getByLabel("Machine (scene)").selectOption("scene.io-basics");
    await page.getByRole("button", { name: "Create and open" }).click();
    const languages = page.getByRole("group", { name: "Program language" });
    await languages.waitFor({ timeout: 60_000 });
    await languages
      .getByRole("button", { name: "Ladder Diagram (LAD)" })
      .click();
    await page
      .getByRole("button", { name: "Replace with a new LAD program" })
      .click();
    await ladder().waitFor({ timeout: 20_000 });
  });

  await check(
    "A: contacts dragged onto an empty rung and to the right of A give A AND B",
    async () => {
      await page.getByRole("button", { name: "+ Rung" }).click();
      // Start from an empty network: delete the rung's default contact.
      await element(1, "Normally open contact (variable?)").click();
      await page.keyboard.press("Delete");
      const empty = rung(1).getByRole("button", {
        name: "Empty network (always on)",
      });
      await empty.waitFor();
      const onEmpty = await spot(empty);
      await mouseDrag(
        center(await box(palette("Insert Normally open contact"))),
        onEmpty,
      );
      // C: the dropped contact's operand is typed with completion.
      await choose("Butt", ["Button1", "Button2"]);
      await waitLogic(0, "Button1");
      const right = await spot(
        element(1, "Normally open contact Button1"),
        "right",
      );
      await mouseDrag(
        center(await box(palette("Insert Normally open contact"))),
        right,
      );
      await choose("Auto", ["AutoMode"]);
      await waitLogic(0, "Button1 AND AutoMode");
      await element(1, "Coil (variable?)").dblclick();
      await choose("Lamp1");
      await page.screenshot({ path: path.join(directory, "a-series.png") });
    },
  );

  await check(
    "B: a contact dropped below A of A AND B gives (A OR C) AND B with the branch wired",
    async () => {
      await page.getByRole("button", { name: "+ Rung" }).click();
      const first = element(2, "Normally open contact (variable?)");
      await first.click();
      // Typing on a selected contact opens its operand editor.
      await page.keyboard.press("B");
      await choose("utton1");
      await page.keyboard.press("F9");
      await choose("AutoMode");
      await waitLogic(1, "Button1 AND AutoMode");
      // The OR preview while hovering below A, then the drop.
      const target = await spot(
        element(2, "Normally open contact Button1"),
        "below",
      );
      await mouseDrag(
        center(await box(palette("Insert Normally open contact"))),
        target,
        { hold: true },
      );
      const preview = await notice().innerText();
      assert.match(
        preview,
        /New branch in parallel with Normally open contact Button1 \(OR\)/,
      );
      assert.match(preview, /\(Button1 OR \?\) AND AutoMode/);
      await page.screenshot({ path: path.join(directory, "b-or-preview.png") });
      await page.mouse.up();
      await choose("Part", ["PartPresent"]);
      await waitLogic(1, "(Button1 OR PartPresent) AND AutoMode");
      await rung(2)
        .getByRole("button", { name: "Parallel block of 2 branches" })
        .waitFor();
      await element(2, "Coil (variable?)").dblclick();
      await choose("Lamp2");
      await page.screenshot({ path: path.join(directory, "b-branch.png") });
    },
  );

  await check(
    "D: TON typed at an insertion point, instance and PT edited on the box",
    async () => {
      await page.getByRole("button", { name: "+ Rung" }).click();
      await element(3, "Normally open contact (variable?)").click();
      await page.keyboard.press("P");
      await choose("artPresent");
      await rung(3)
        .getByRole("button", {
          name: "Insertion point after Normally open contact PartPresent",
        })
        .click();
      await page.keyboard.press("T");
      const search = page.getByRole("combobox", { name: "Instruction name" });
      await search.waitFor();
      await page.keyboard.type("ON");
      const first = page
        .getByRole("listbox", { name: "Instructions" })
        .getByRole("option")
        .first();
      const offered = await first.innerText();
      assert.match(offered, /^TON/);
      // Pins as the runtime declares them.
      assert.match(offered, /IN: BOOL, PT: TIME → Q: BOOL, ET: TIME/);
      await page.keyboard.press("Enter");
      await combobox().waitFor();
      await page.keyboard.press("Control+A");
      await page.keyboard.type("Hold2");
      await page.keyboard.press("Tab");
      await until(
        () => combobox().getAttribute("aria-label"),
        (v) => v === "Preset time PT of TON Hold2",
        "the PT editor after Tab",
      );
      await page.keyboard.type("T#2s");
      await page.keyboard.press("Enter");
      await waitLogic(2, "PartPresent AND TON(Hold2)");
      const doc = await savedLadder();
      assert.deepEqual(doc.rungs[2].network.items[1].preset, {
        constant: "T#2s",
      });
      await element(3, "Coil (variable?)").dblclick();
      await choose("Lamp3");
      await problems().waitFor({ state: "detached", timeout: 15_000 });
    },
  );

  // The process controls of the I/O basics scene.
  const lamp = async (label) =>
    (
      await page
        .locator(`[aria-label^="${label}: "]`)
        .getAttribute("aria-label")
    ).endsWith(": on");
  const setSelector = async (auto) => {
    const sel = page.locator('[aria-label^="Selector S3: "]');
    for (let i = 0; i < 3; i++) {
      const now = (await sel.getAttribute("aria-label")).endsWith("AUTO");
      if (now === auto) return;
      await sel.click();
      await delay(300);
    }
    throw new Error("selector did not switch");
  };
  const setPresence = async (present) => {
    const sensor = page.locator('[aria-label^="Proximity sensor B1: target "]');
    for (let i = 0; i < 3; i++) {
      const now = (await sensor.getAttribute("aria-label")).includes(
        "target present",
      );
      if (now === present) return;
      await sensor.click();
      await delay(300);
    }
    throw new Error("sensor did not switch");
  };
  const button = () =>
    page.getByRole("button", { name: "Pushbutton S1 (hold to press)" });
  /** Lamp H1 and H2 for all 8 combinations of Button1, AutoMode and PartPresent. */
  const truthTable = async () => {
    await tab("Process");
    const rows = [];
    for (const a of [false, true])
      for (const b of [false, true])
        for (const c of [false, true]) {
          await setSelector(b);
          await setPresence(c);
          if (a) {
            await button().dispatchEvent("pointerdown", { pointerId: 7 });
            await button()
              .and(page.locator('[aria-pressed="true"]'))
              .waitFor({ timeout: 10_000 });
          }
          const want = [a && b, (a || c) && b];
          const got = await until(
            async () => [await lamp("Lamp H1"), await lamp("Lamp H2")],
            (v) => v[0] === want[0] && v[1] === want[1],
            `A=${a} B=${b} C=${c}`,
            4000,
          ).catch(async () => [await lamp("Lamp H1"), await lamp("Lamp H2")]);
          rows.push({ a, b, c, lamp1: got[0], lamp2: got[1] });
          if (a) {
            await button().dispatchEvent("pointerup", { pointerId: 7 });
            await button()
              .and(page.locator('[aria-pressed="false"]'))
              .waitFor({ timeout: 10_000 });
          }
        }
    await setSelector(false);
    await setPresence(false);
    return rows;
  };
  const expected = [false, true].flatMap((a) =>
    [false, true].flatMap((b) =>
      [false, true].map((c) => ({
        a,
        b,
        c,
        lamp1: a && b,
        lamp2: (a || c) && b,
      })),
    ),
  );

  let ladResults;
  await check(
    "the ladder compiles in Rust, runs, and gives the truth table of A AND B and (A OR C) AND B",
    async () => {
      await page
        .getByRole("toolbar")
        .getByRole("button", { name: "Build & Load to PLC" })
        .click();
      await page.getByText("Sim running").first().waitFor({ timeout: 30_000 });
      await until(
        () => page.locator(".lad-status").innerText(),
        (t) => /^Online, scan \d+/.test(t),
        "online status",
      );
      ladResults = await truthTable();
      evidence.ladTruthTable = ladResults;
      assert.deepEqual(ladResults, expected);
    },
  );

  await check("D: the timer counts online and drives its coil", async () => {
    await tab("Process");
    await setPresence(true);
    await tab("Program");
    const ton = element(3, "TON Hold2").or(
      rung(3).getByRole("button", { name: /^TON Hold2: preset T#2s/ }),
    );
    const seen = new Set();
    await until(
      async () => {
        const label = (await ton.first().getAttribute("aria-label")) ?? "";
        const value = /value (T#\w+)/.exec(label)?.[1];
        if (value && value !== "T#0ms" && value !== "T#2s") seen.add(value);
        return value;
      },
      (v) => v === "T#2s",
      "ET should reach PT",
    );
    assert.ok(seen.size >= 2, `ET counting: ${JSON.stringify([...seen])}`);
    await element(3, "Coil Lamp3: TRUE, power flows").waitFor({
      timeout: 10_000,
    });
    await page.screenshot({ path: path.join(directory, "d-timer-online.png") });
    await tab("Process");
    await setPresence(false);
    await tab("Program");
  });

  await check(
    "F: an invalid drop explains itself and changes nothing; Esc cancels a drag",
    async () => {
      const before = JSON.stringify(await savedLadder());
      const onContact = await spot(
        element(1, /^Normally open contact Button1/),
      );
      await mouseDrag(center(await box(palette("Add coil"))), onContact, {
        hold: true,
      });
      assert.equal(
        await notice().innerText(),
        "Coils go in the coil column at the right end of a rung.",
      );
      await page.locator(".lad-ghost.invalid").waitFor();
      await page.screenshot({
        path: path.join(directory, "f-invalid-drop.png"),
      });
      await page.mouse.up();
      // A valid target, then Esc before releasing.
      const afterAuto = await spot(
        element(1, /^Normally open contact AutoMode/),
        "right",
      );
      await mouseDrag(
        center(await box(palette("Insert Normally open contact"))),
        afterAuto,
        { hold: true },
      );
      await page.keyboard.press("Escape");
      await page.mouse.up();
      await delay(1500);
      assert.equal(JSON.stringify(await savedLadder()), before);
      // The undo history still holds the last real edit (Lamp3 on rung 3's coil).
      await element(1, /^Normally open contact Button1/).click();
      await page.keyboard.press("Control+z");
      await until(
        async () => (await savedLadder()).rungs[2].coils[0].variable,
        (v) => v === "",
        "undo should revert the last real edit",
      );
      await page.keyboard.press("Control+y");
      await until(
        async () => (await savedLadder()).rungs[2].coils[0].variable,
        (v) => v === "Lamp3",
        "redo",
      );
    },
  );

  await check(
    "E: an existing contact dragged onto a new branch; undo, redo, save and reopen",
    async () => {
      const branch = await spot(
        element(1, /^Normally open contact Button1/),
        "below",
      );
      await mouseDrag(
        center(await box(element(1, /^Normally open contact AutoMode/))),
        branch,
      );
      await waitLogic(0, "(Button1 OR AutoMode)");
      await page.keyboard.press("Control+z");
      await waitLogic(0, "Button1 AND AutoMode");
      await page.keyboard.press("Control+y");
      await waitLogic(0, "(Button1 OR AutoMode)");
      await delay(1500);
      await page
        .getByRole("button", { name: "Back", exact: true })
        .first()
        .click();
      await page
        .locator(".list-row")
        .filter({ hasText: "Ladder editor" })
        .getByRole("button", { name: "Open", exact: true })
        .click();
      await ladder().waitFor({ timeout: 60_000 });
      await rung(1)
        .getByRole("button", { name: "Parallel block of 2 branches" })
        .waitFor();
      assert.equal(await rungLogic(0), "(Button1 OR AutoMode)");
      assert.equal(await rungLogic(1), "(Button1 OR PartPresent) AND AutoMode");
      assert.equal(await rungLogic(2), "PartPresent AND TON(Hold2)");
    },
  );

  await check(
    "G: a rung built with the keyboard only, with a confirmed new variable, compiles",
    async () => {
      const add = page.getByRole("button", { name: "+ Rung" });
      await add.focus();
      await page.keyboard.press("Enter");
      await until(
        () =>
          page.evaluate(() =>
            document.activeElement?.getAttribute("aria-label"),
          ),
        (v) => v === "Normally open contact (variable?)",
        "the new rung's contact has the focus",
      );
      await page.keyboard.press("P");
      await choose("artPresent");
      await page.keyboard.press("F9");
      await choose("Button2");
      await page.keyboard.press("Shift+F9");
      await choose("Auto");
      await waitLogic(3, "PartPresent AND (Button2 OR AutoMode)");
      await page.keyboard.press("ArrowRight");
      await page.keyboard.press("ArrowRight");
      await until(
        () =>
          page.evaluate(() =>
            document.activeElement?.getAttribute("aria-label"),
          ),
        (v) => v === "Coil (variable?)",
        "arrows reach the coil",
      );
      await page.keyboard.press("X");
      await page.keyboard.type("tra");
      // No variable is called Xtra: Enter offers to declare it, nothing more.
      await page.keyboard.press("Enter");
      const form = page.getByRole("form", { name: "Declare “Xtra”" });
      await form.waitFor();
      assert.equal(
        (await savedLadder()).variables.some((v) => v.name === "Xtra"),
        false,
      );
      await page.keyboard.press("Shift+Tab");
      await page.keyboard.press("ArrowDown");
      await page.keyboard.press("Tab");
      await page.keyboard.press("Enter");
      await until(
        async () => {
          const doc = await savedLadder();
          return [
            doc.variables.find((v) => v.name === "Xtra"),
            doc.rungs[3].coils[0].variable,
          ];
        },
        ([v, coil]) =>
          v?.type === "BOOL" && v.direction === "local" && coil === "Xtra",
        "Xtra declared as an internal BOOL and bound to the coil",
      );
      await problems().waitFor({ state: "detached", timeout: 15_000 });
      const build = page
        .getByRole("toolbar")
        .getByRole("button", { name: "Build & Load to PLC" });
      await build.focus();
      await page.keyboard.press("Enter");
      await until(
        () => page.locator(".lad-status").innerText(),
        (t) => /^Online, scan \d+/.test(t),
        "online after a keyboard Build & Load",
      );
      await page.screenshot({ path: path.join(directory, "g-keyboard.png") });
    },
  );

  // ---- screenshots and layout checks: 3 sizes × English/Polish × dark/light ----
  const layoutProblems = () =>
    page.evaluate(() => {
      const problems = [];
      for (const canvas of document.querySelectorAll(".lad-canvas")) {
        const c = canvas.getBoundingClientRect();
        const els = [...canvas.querySelectorAll(".lad-el")].map((e) => ({
          name: e.getAttribute("aria-label"),
          r: e.getBoundingClientRect(),
        }));
        for (const e of els)
          if (
            e.r.left < c.left - 1 ||
            e.r.right > c.right + 1 ||
            e.r.top < c.top - 1 ||
            e.r.bottom > c.bottom + 1
          )
            problems.push(`outside its rung: ${e.name}`);
        for (let i = 0; i < els.length; i++)
          for (let j = i + 1; j < els.length; j++) {
            const a = els[i].r;
            const b = els[j].r;
            if (
              a.left < b.right - 1 &&
              b.left < a.right - 1 &&
              a.top < b.bottom - 1 &&
              b.top < a.bottom - 1
            )
              problems.push(`overlap: ${els[i].name} / ${els[j].name}`);
          }
        for (const n of canvas.querySelectorAll(".lad-name"))
          if (n.scrollWidth > n.clientWidth + 1)
            problems.push(`clipped name: ${n.textContent}`);
      }
      return problems;
    });
  const settings = async (language, theme, labels) => {
    await page
      .getByRole("button", { name: labels.settings, exact: true })
      .click();
    await page
      .getByRole("combobox", { name: labels.language })
      .selectOption(language);
    const after =
      language === "pl"
        ? { theme: "Motyw", close: "Zamknij" }
        : { theme: "Theme", close: "Close" };
    await page.getByRole("combobox", { name: after.theme }).selectOption(theme);
    await page.getByRole("button", { name: after.close, exact: true }).click();
    assert.equal(
      await page.evaluate(() => document.documentElement.dataset.theme),
      theme,
    );
  };
  await check(
    "screenshots at 1366x768, 1920x1080 and 2560x1440 in English and Polish, dark and light",
    async () => {
      const shots = [];
      let labels = { settings: "Settings", language: "Language" };
      for (const [language, theme] of [
        ["en", "dark"],
        ["en", "light"],
        ["pl", "light"],
        ["pl", "dark"],
      ]) {
        await settings(language, theme, labels);
        labels =
          language === "pl"
            ? { settings: "Ustawienia", language: "Język" }
            : { settings: "Settings", language: "Language" };
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
          await delay(400);
          const file = `ladder-${language}-${theme}-${width}x${height}.png`;
          await page.screenshot({ path: path.join(directory, file) });
          shots.push({ file, problems: await layoutProblems() });
        }
      }
      await cdp.send("Emulation.clearDeviceMetricsOverride");
      evidence.screenshots = shots;
      const bad = shots.filter((s) => s.problems.length);
      assert.deepEqual(bad, [], JSON.stringify(bad));
      await settings("en", "dark", {
        settings: "Ustawienia",
        language: "Język",
      });
    },
  );

  await check(
    "the same logic in Structured Text gives the same truth table",
    async () => {
      await page
        .getByRole("group", { name: "Program language" })
        .getByRole("button", { name: "Structured Text (ST)" })
        .click();
      await page
        .getByRole("button", { name: "Replace with a new ST program" })
        .click();
      const editor = page.locator('[data-testid="st-editor"] .monaco-editor');
      await editor.waitFor({ timeout: 20_000 });
      const before = await editorText(page);
      assert.match(before, /VAR_INPUT[\s\S]*Button1 : BOOL;/);
      await editor.click({ position: { x: 300, y: 12 } });
      await page.keyboard.press("Control+End");
      await page.keyboard.press("ArrowUp");
      await page.keyboard.press("ArrowUp");
      await page.keyboard.type("Lamp1 := Button1 AND AutoMode;");
      await page.keyboard.press("Enter");
      await page.keyboard.type("Lamp2 := (Button1 OR PartPresent");
      await page.keyboard.press("End");
      const line = await editorText(page);
      if (!/PartPresent\)/.test(line)) await page.keyboard.type(")");
      await page.keyboard.type(" AND AutoMode;");
      await delay(500);
      const st = await editorText(page);
      assert.match(st, /Lamp2 := \(Button1 OR PartPresent\) AND AutoMode;/);
      await problems().waitFor({ state: "detached", timeout: 15_000 });
      await page
        .getByRole("toolbar")
        .getByRole("button", { name: "Build & Load to PLC" })
        .click();
      await page.getByText("Sim running").first().waitFor({ timeout: 30_000 });
      await delay(800);
      const stResults = await truthTable();
      evidence.stTruthTable = stResults;
      assert.deepEqual(stResults, ladResults);
    },
  );

  await check(
    "no black screen and no uncaught error in the session",
    async () => {
      assert.deepEqual(pageErrors, []);
    },
  );

  await browser.close().catch(() => undefined);
  browser = undefined;
  spawnSync("taskkill", ["/IM", "plc-lab-desktop.exe"], { stdio: "ignore" });
  const deadline = Date.now() + 60_000;
  while (exited === null && Date.now() < deadline) await delay(250);
  assert.equal(exited, 0, `${launcher} did not end cleanly`);
  if (!exe) assert.match(output, /\[DEV\] Application closed successfully/);
  passed.push("the window closes cleanly");
  console.log("CHECK PASS: the window closes cleanly");
  console.log(`EVIDENCE ${JSON.stringify(evidence)}`);
  console.log(`LAD EDITOR SESSION PASS (${passed.length} checks)`);
} catch (error) {
  console.error(`LAD EDITOR SESSION FAILED after: ${passed.join(", ")}`);
  console.error(error);
  console.error(`evidence: ${JSON.stringify(evidence)}`);
  console.error(`uncaught: ${JSON.stringify(pageErrors)}`);
  if (page && !page.isClosed())
    await page
      .screenshot({ path: path.join(directory, "failure.png") })
      .catch(() => undefined);
  process.exitCode = 1;
} finally {
  await browser?.close().catch(() => undefined);
  if (exited === null) {
    spawnSync("taskkill", ["/IM", "plc-lab-desktop.exe"], { stdio: "ignore" });
    await delay(1500);
  }
  if (!process.exitCode)
    for (const folder of [`webview-${stamp}`, `data-${stamp}`])
      for (let attempt = 0; attempt < 20; attempt++) {
        try {
          await rm(path.join(directory, folder), {
            recursive: true,
            force: true,
          });
          break;
        } catch {
          await delay(250);
        }
      }
}
