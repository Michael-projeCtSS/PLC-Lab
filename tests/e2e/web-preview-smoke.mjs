// Real browser over the actual development server; no mocked IPC/backend.
import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import path from "node:path";
import { chromium } from "playwright-core";
import { root } from "../../scripts/toolchain.mjs";

const output = path.join(root, "artifacts/web-preview-smoke");
await mkdir(output, { recursive: true });
const browser = await chromium.launch({
  channel: process.env.PLC_LAB_BROWSER_CHANNEL ?? "msedge",
  headless: true,
});
const context = await browser.newContext({
  locale: "pl-PL",
  viewport: { width: 1440, height: 950 },
});
const page = await context.newPage();
const errors = [];
page.on("pageerror", (error) => errors.push(error.message));
page.on("console", (message) => {
  if (message.type() === "error") errors.push(message.text());
});
const steps = [];
async function step(name, run) {
  await run();
  steps.push(name);
}
const nav = () =>
  page.getByRole("navigation", {
    name: /^(Main|Nawigacja główna)$/,
  });
try {
  await step("real preview endpoint and Polish Home", async () => {
    await page.goto("http://127.0.0.1:1420");
    await page
      .getByRole("heading", { name: "Ucz się automatyki w praktyce" })
      .waitFor();
    await page.getByRole("button", { name: "Rozpocznij lekcję 1" }).waitFor();
    assert.equal(await page.locator("html").getAttribute("lang"), "pl");
    await page
      .getByRole("complementary", { name: "TRYB PODGLĄDU INTERFEJSU" })
      .waitFor();
    assert.equal(
      await page.getByRole("alert").filter({ hasText: /\S/ }).count(),
      0,
    );
    assert.equal(
      await page.evaluate(() => "__TAURI_INTERNALS__" in window),
      false,
    );
  });
  await step("live Vite updates without rebuilding an executable", async () => {
    const stylesheet = path.join(root, "packages/ui/src/styles.css");
    const token = randomUUID();
    const probe = `\n/* browser smoke ${token} */\n:root { --plc-web-smoke-probe: ${token}; }\n`;
    await writeFile(stylesheet, (await readFile(stylesheet, "utf8")) + probe);
    try {
      await page.waitForFunction(
        (value) =>
          getComputedStyle(document.documentElement)
            .getPropertyValue("--plc-web-smoke-probe")
            .trim() === value,
        token,
      );
    } finally {
      await writeFile(
        stylesheet,
        (await readFile(stylesheet, "utf8")).replace(probe, ""),
      );
    }
  });
  await step("real Polish lesson and disabled native actions", async () => {
    await page.getByRole("button", { name: "Rozpocznij lekcję 1" }).click();
    await page
      .getByRole("heading", { name: "Czym jest PLC?", exact: true })
      .waitFor();
    await page
      .getByRole("button", { name: /Otwórz.*(?:symul|laborator|scen|panel)/i })
      .first()
      .waitFor();
    assert.ok(
      await page
        .getByRole("button", {
          name: /Otwórz.*(?:symul|laborator|scen|panel)/i,
        })
        .first()
        .isDisabled(),
    );
  });
  await step("Practice catalogue and useful native explanation", async () => {
    await nav().getByRole("button", { name: "ĆWICZENIA", exact: true }).click();
    const exercises = page.locator(".tile");
    await exercises.first().waitFor();
    assert.ok((await exercises.count()) >= 10);
    await exercises.first().click();
    const dialog = page.getByRole("dialog");
    await dialog
      .getByRole("heading", { name: "Wymagane natywne zaplecze" })
      .waitFor();
    await dialog.getByRole("button", { name: "Zamknij", exact: true }).click();
  });
  await step("Knowledge catalogue, TON reader and Polish text", async () => {
    await nav()
      .getByRole("button", { name: "BAZA WIEDZY", exact: true })
      .click();
    await page
      .getByRole("button", { name: /TON.*(?:opóź|czas|załącz)/i })
      .first()
      .click();
    await page
      .locator("article")
      .getByRole("heading", { name: /TON/ })
      .waitFor();
    assert.ok((await page.locator("article").innerText()).includes("TON"));
    assert.equal(
      await page.getByRole("alert").filter({ hasText: /\S/ }).count(),
      0,
    );
  });
  await step(
    "Settings language and density persist after refresh",
    async () => {
      await page
        .getByRole("button", { name: "Ustawienia", exact: true })
        .click();
      await page.getByLabel(/^Język/).selectOption("en");
      await page.getByLabel(/^UI density/).selectOption("compact");
      await page
        .getByRole("dialog", { name: "Settings" })
        .getByRole("button", { name: "Close", exact: true })
        .click();
      await page.reload();
      await page
        .getByRole("heading", { name: "Learn industrial automation by doing" })
        .waitFor();
      assert.equal(await page.locator("html").getAttribute("lang"), "en");
      assert.equal(
        await page.locator(".app").getAttribute("data-density"),
        "compact",
      );
      await page.getByRole("button", { name: "Settings", exact: true }).click();
      await page.getByLabel(/^Language/).selectOption("pl");
      await page
        .getByRole("dialog", { name: "Ustawienia" })
        .getByRole("button", { name: "Zamknij", exact: true })
        .click();
    },
  );
  await step(
    "Basic/Advanced editor and native-only engineering tabs",
    async () => {
      await nav()
        .getByRole("button", { name: "LABORATORIUM", exact: true })
        .click();
      await page.locator(".monaco-editor").waitFor();
      await page.getByRole("button", { name: /ZAAWANSOWANY|ADVANCED/ }).click();
      await page.getByRole("tab", { name: /Sprzęt|Hardware/ }).click();
      assert.ok((await page.locator("main").innerText()).includes("natywne"));
      await page.getByRole("button", { name: /PODSTAWOWY|BASIC/ }).click();
      await page.getByRole("tab", { name: "Program", exact: true }).waitFor();
      assert.ok(
        await page
          .getByRole("button", { name: /Kompiluj i wgraj/ })
          .last()
          .isDisabled(),
      );
    },
  );
  await step(
    "native progress and About do not invoke unavailable IPC",
    async () => {
      await nav().getByRole("button", { name: "POSTĘPY", exact: true }).click();
      await page
        .getByRole("heading", { name: "Wymagane natywne zaplecze" })
        .waitFor();
      await page
        .getByRole("button", { name: "O aplikacji", exact: true })
        .click();
      assert.equal(
        await page.getByRole("alert").filter({ hasText: /\S/ }).count(),
        0,
      );
    },
  );
  await step("responsive Polish navigation and browser refresh", async () => {
    await page.setViewportSize({ width: 390, height: 844 });
    await nav().getByRole("button", { name: "START", exact: true }).click();
    await page.reload();
    await page
      .getByRole("heading", { name: "Ucz się automatyki w praktyce" })
      .waitFor();
    assert.ok(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    );
    await page.screenshot({
      path: path.join(output, "polish-mobile.png"),
      fullPage: true,
    });
    await page.setViewportSize({ width: 1440, height: 950 });
    await page.screenshot({
      path: path.join(output, "polish-home.png"),
      fullPage: true,
    });
  });
  assert.deepEqual(errors, []);
  await writeFile(
    path.join(output, "result.json"),
    JSON.stringify(
      {
        result: "PASS",
        url: page.url(),
        steps,
        errors,
        nativeBackend: false,
        plcSimulation: "NOT SUPPORTED",
      },
      null,
      2,
    ),
  );
  console.log(`Browser UI preview smoke PASS (${steps.length} steps).`);
} catch (error) {
  await page.screenshot({
    path: path.join(output, "failure.png"),
    fullPage: true,
  });
  await writeFile(
    path.join(output, "result.json"),
    JSON.stringify(
      { result: "FAIL", steps, errors, error: error.message },
      null,
      2,
    ),
  );
  throw error;
} finally {
  await browser.close();
}
