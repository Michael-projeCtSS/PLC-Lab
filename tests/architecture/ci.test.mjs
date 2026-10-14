import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm } from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { parsers } from "prettier/plugins/yaml";
import { validationPlan, readToolchain, runStages } from "../../scripts/ci.mjs";
import { root } from "../../scripts/toolchain.mjs";

// Decode only the YAML vocabulary used here; Prettier performs actual YAML parsing.
function decode(node) {
  if (!node) return null;
  if (node.type === "mapping")
    return Object.fromEntries(
      node.children.map((item) => item.children.map(decode)),
    );
  if (node.type === "sequence") return node.children.map(decode);
  if (node.value !== undefined) return node.value;
  return decode(node.children?.[0]);
}
async function workflow(name) {
  const source = await readFile(
    path.join(root, ".github/workflows", name),
    "utf8",
  );
  const tree = parsers.yaml.parse(source);
  return {
    source,
    value: decode(
      tree.children[0].children.find((node) => node.type === "documentBody"),
    ),
  };
}
test("workflow YAML preserves triggers, two lanes, read permissions and scoped artifacts", async () => {
  const { source, value } = await workflow("validation.yml");
  assert.deepEqual(Object.keys(value.on).sort(), [
    "pull_request",
    "push",
    "workflow_call",
    "workflow_dispatch",
  ]);
  assert.deepEqual(value.permissions, { contents: "read" });
  assert.equal(value.jobs["windows-validation"]["runs-on"], "windows-2022");
  assert.equal(value.jobs["linux-core"]["runs-on"], "ubuntu-24.04");
  for (const job of Object.values(value.jobs)) {
    for (const step of job.steps.filter((item) => item.uses))
      assert.match(step.uses, /^actions\/[a-z-]+@[a-f0-9]{40}$/);
    const checkout = job.steps.find((item) =>
      item.uses?.startsWith("actions/checkout@"),
    );
    assert.equal(checkout.with["persist-credentials"], "false");
    const node = job.steps.find((item) =>
      item.uses?.startsWith("actions/setup-node@"),
    );
    assert.equal(node.with["node-version"], "24");
    assert.equal(node.with["package-manager-cache"], "false");
    const upload = job.steps.find((item) =>
      item.uses?.startsWith("actions/upload-artifact@"),
    );
    assert.equal(upload.if, "always()");
    assert.equal(upload.with["retention-days"], "14");
    const commands = job.steps
      .filter((item) => item.run)
      .map((item) => item.run);
    assert.ok(commands.includes("node scripts/ci.mjs dependencies"));
    assert.ok(commands.includes("node scripts/ci.mjs setup-rust"));
  }
  assert.match(source, /artifacts\/desktop-smoke\/native\.log/);
  assert.match(source, /artifacts\/desktop-smoke\/result\.json/);
  assert.match(source, /artifacts\/desktop-smoke\/desktop\.png/);
  assert.doesNotMatch(
    source,
    /continue-on-error|pull_request_target|secrets:|contents: write|id-token: write/,
  );
  const legacy = (await workflow("quality.yml")).value;
  assert.deepEqual(Object.keys(legacy.on), ["workflow_dispatch"]);
  assert.equal(
    legacy.jobs.validation.uses,
    "./.github/workflows/validation.yml",
  );
});

test("Windows plan preserves canonical checks and real native smoke plus both M1C profiles", () => {
  const plan = validationPlan("windows", "win32");
  assert.deepEqual(
    plan.map((item) => item.id),
    ["canonical-check", "m1c-debug", "runtime-clock-release", "native-desktop"],
  );
  assert.equal(plan[0].args.at(-1), "npm.cmd run check");
  assert.equal(plan[3].args.at(-1), "npm.cmd run test:desktop");
  for (const name of [
    "plc-types",
    "plc-ir",
    "plc-runtime",
    "simulation-core",
    "st-frontend",
    "app-contract",
    "desktop-session",
    "process-simulation",
  ])
    assert.ok(plan[1].args.includes(name));
  assert.ok(plan[2].args.includes("--release"));
  assert.ok(plan[2].args.includes("simulation-core"));
  assert.ok(plan[2].args.includes("plc-runtime"));
  assert.ok(plan[2].args.includes("st-frontend"));
  assert.ok(plan[2].args.includes("app-contract"));
  assert.ok(plan[2].args.includes("desktop-session"));
  assert.ok(plan[2].args.includes("process-simulation"));
  const portable = validationPlan("core", "linux");
  for (const packageName of [
    "app-contract",
    "desktop-session",
    "process-simulation",
  ]) {
    assert.ok(portable[1].args.includes(packageName));
    assert.ok(portable[2].args.includes(packageName));
    assert.ok(portable[3].args.includes(packageName));
  }
  const core = validationPlan("core", "linux");
  assert.equal(core[0].id, "rust-format");
  assert.ok(core[1].args.includes("--all-targets"));
  assert.deepEqual(core[1].args.slice(-3), ["--", "-D", "warnings"]);
  assert.equal(core.at(-1).args.join(" "), "run test:architecture");
  assert.ok(
    core.every((item) => !item.args.join(" ").includes("test:desktop")),
  );
  assert.throws(() => validationPlan("unknown"), /Unknown validation lane/);
});

test("Rust setup reads the repository pin and refuses missing quality components", async () => {
  const text = await readFile(path.join(root, "rust-toolchain.toml"), "utf8");
  const config = readToolchain(text);
  assert.equal(config.channel, "1.99.0");
  assert.deepEqual(config.components, ["rustfmt", "clippy"]);
  assert.throws(
    () => readToolchain(text.replace('"clippy"', '"other"')),
    /requires/,
  );
  assert.throws(
    () => readToolchain(text.replace('channel = "1.99.0"', "")),
    /cannot guess/,
  );
});

async function fixture() {
  const parent = path.join(root, "artifacts", "ci-fixtures");
  await mkdir(parent, { recursive: true });
  return mkdtemp(path.join(parent, "test-"));
}
async function cleanup(directory) {
  assert.ok(
    path
      .resolve(directory)
      .startsWith(`${path.resolve(root, "artifacts/ci-fixtures")}${path.sep}`),
  );
  await rm(directory, { recursive: true, force: true });
}

test("stage failure retains stdout/stderr/exit code and never marks later stages successful", async () => {
  const directory = await fixture();
  try {
    const report = await runStages(
      [
        {
          id: "intentional-failure",
          command: process.execPath,
          args: [
            "-e",
            "process.stdout.write('stdout evidence'); process.stderr.write('stderr evidence'); process.exit(7)",
          ],
        },
        {
          id: "must-not-run",
          command: process.execPath,
          args: ["-e", "throw Error('unexpected execution')"],
        },
      ],
      { directory, lane: "fixture", display: false },
    );
    assert.equal(report.result, "FAIL");
    assert.equal(report.stages[0].exitCode, 7);
    assert.equal(report.stages[1].result, "NOT_RUN");
    const log = await readFile(
      path.join(directory, "intentional-failure.log"),
      "utf8",
    );
    assert.match(log, /stdout evidence/);
    assert.match(log, /stderr evidence/);
    assert.deepEqual(
      JSON.parse(await readFile(path.join(directory, "result.json"), "utf8")),
      report,
    );
  } finally {
    await cleanup(directory);
  }
});

test("process-creation failures are failures with retained diagnostics", async () => {
  const directory = await fixture();
  try {
    const report = await runStages(
      [
        {
          id: "missing-command",
          command: path.join(directory, "missing-command.exe"),
          args: [],
        },
      ],
      { directory, lane: "fixture", display: false },
    );
    assert.equal(report.result, "FAIL");
    assert.notEqual(report.stages[0].exitCode, 0);
    assert.match(report.stages[0].launchError, /ENOENT/);
    assert.match(
      await readFile(path.join(directory, "missing-command.log"), "utf8"),
      /Process creation failed/,
    );
  } finally {
    await cleanup(directory);
  }
});

test("successful stages require real process success and CLI errors return nonzero", async () => {
  const directory = await fixture();
  try {
    const report = await runStages(
      [
        {
          id: "real-node-process",
          command: process.execPath,
          args: ["-e", "process.stdout.write('executed')"],
        },
      ],
      { directory, lane: "fixture", display: false },
    );
    assert.equal(report.result, "PASS");
    assert.equal(report.stages[0].exitCode, 0);
    const failed = spawnSync(process.execPath, ["scripts/ci.mjs", "unknown"], {
      cwd: root,
      encoding: "utf8",
    });
    assert.notEqual(failed.status, 0);
    assert.match(failed.stderr, /Unknown validation lane/);
  } finally {
    await cleanup(directory);
  }
});

test("repository npm command launches through the real platform command path", async () => {
  const directory = await fixture();
  try {
    const stage = validationPlan("core").at(-1);
    stage.id = "npm-status";
    stage.args =
      process.platform === "win32"
        ? ["/d", "/s", "/c", "npm.cmd run ci:status"]
        : ["run", "ci:status"];
    const report = await runStages([stage], {
      directory,
      lane: "fixture",
      display: false,
    });
    assert.equal(report.result, "PASS");
    assert.match(
      await readFile(path.join(directory, "npm-status.log"), "utf8"),
      /HOST POLICY MAY BLOCK GENERATED EXECUTABLES/,
    );
  } finally {
    await cleanup(directory);
  }
});
