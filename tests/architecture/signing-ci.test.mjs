import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { parsers } from "prettier/plugins/yaml";
import { root } from "../../scripts/toolchain.mjs";

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

test("production credentials are isolated behind explicit dispatch, trusted ref and protected environment", async () => {
  const { source, value } = await workflow("windows-signed-release.yml");
  assert.deepEqual(Object.keys(value.on), ["workflow_dispatch"]);
  assert.equal(value.on.workflow_dispatch, null, "no arbitrary source inputs");
  assert.deepEqual(value.permissions, { contents: "read" });
  assert.equal(value.jobs.validate.uses, "./.github/workflows/validation.yml");
  assert.equal(value.jobs.sign.needs, "validate");
  for (const job of Object.values(value.jobs)) {
    assert.match(job.if, /github\.repository == 'Michael-projeCtSS\/PLC-Lab'/);
    assert.match(
      job.if,
      /github\.ref == 'refs\/heads\/claude\/product-development-v2'/,
    );
  }
  const sign = value.jobs.sign;
  assert.match(sign.if, /needs\.validate\.result == 'success'/);
  assert.equal(sign.environment, "windows-production-signing");
  assert.deepEqual(sign.permissions, {
    contents: "read",
    actions: "read",
  });
  const checkout = sign.steps.find((step) =>
    step.uses?.startsWith("actions/checkout@"),
  );
  assert.equal(checkout.with.ref, "${{ github.sha }}");
  assert.equal(checkout.with["persist-credentials"], "false");
  for (const step of sign.steps.filter((step) => step.uses))
    assert.match(step.uses, /^(?:actions|azure)\/[a-z-]+@[a-f0-9]{40}$/);
  assert.doesNotMatch(
    source,
    /continue-on-error:|secrets: inherit|id-token:|AZURE_|creds:|contents: write|pull_request_target:/,
  );
});

test("only the immutable archive from this validation run reaches the signing service", async () => {
  const canonical = (await workflow("validation.yml")).value;
  const outputs = canonical.on.workflow_call.outputs;
  assert.match(outputs["windows-artifact-id"].value, /outputs\.artifact-id/);
  assert.match(
    outputs["windows-artifact-digest"].value,
    /outputs\.artifact-digest/,
  );
  const uploaded = canonical.jobs["windows-validation"].steps.find(
    (step) => step.id === "unsigned-windows",
  );
  assert.match(uploaded.with.name, /UNSIGNED-development/);
  const clientCheckIndex = canonical.jobs["windows-validation"].steps.findIndex(
    (step) => step.run?.includes("node scripts/esigner.mjs doctor"),
  );
  const validationIndex = canonical.jobs["windows-validation"].steps.findIndex(
    (step) => step.run === "npm run ci:windows",
  );
  assert.ok(clientCheckIndex >= 0 && clientCheckIndex < validationIndex);
  assert.match(
    canonical.jobs["windows-validation"].steps[clientCheckIndex].run,
    /node scripts\/esigner\.mjs install/,
  );
  assert.doesNotMatch(
    JSON.stringify(canonical.jobs),
    /ESIGNER_(?:USERNAME|PASSWORD|CREDENTIAL_ID|TOTP_SECRET)|id-token/,
  );
  const { value } = await workflow("windows-signed-release.yml");
  const steps = value.jobs.sign.steps;
  const transferIndex = steps.findIndex((step) => step.env?.ARTIFACT_ID);
  const signIndex = steps.findIndex((step) =>
    step.run?.includes("windows-signing.mjs sign "),
  );
  const installClientIndex = steps.findIndex(
    (step) => step.run === "node scripts/esigner.mjs install",
  );
  assert.ok(
    installClientIndex >= 0 &&
      installClientIndex < transferIndex &&
      transferIndex < signIndex,
  );
  const transfer = steps[transferIndex];
  assert.equal(
    transfer.env.ARTIFACT_ID,
    "${{ needs.validate.outputs.windows-artifact-id }}",
  );
  assert.equal(
    transfer.env.ARTIFACT_DIGEST,
    "${{ needs.validate.outputs.windows-artifact-digest }}",
  );
  assert.match(transfer.run, /workflow_run\.id -ne \$env:GITHUB_RUN_ID/);
  assert.match(transfer.run, /workflow_run\.head_sha -cne \$env:GITHUB_SHA/);
  assert.match(
    transfer.run,
    /Get-FileHash -LiteralPath \$archive -Algorithm SHA256/,
  );
  assert.match(transfer.run, /actualDigest -ine \$env:ARTIFACT_DIGEST.*throw/);
  assert.ok(
    transfer.run.indexOf("actualDigest -ine") <
      transfer.run.indexOf("Expand-Archive"),
    "the archive must pass SHA-256 verification before extraction",
  );
  assert.match(transfer.run, /Unsafe or duplicate path/);
  assert.match(transfer.run, /second application executable/);
  assert.match(transfer.run, /node scripts\/distribution\.mjs \$destination/);
  const signingSteps = steps.filter((step) =>
    step.run?.includes("windows-signing.mjs sign"),
  );
  assert.equal(
    signingSteps.length,
    2,
    "application and manifest catalog must both be signed",
  );
  assert.equal(
    signingSteps[0].run,
    'node scripts/windows-signing.mjs sign "artifacts/unsigned-windows/PLC Lab.exe"',
  );
  assert.equal(
    signingSteps[1].run,
    'node scripts/windows-signing.mjs sign-catalog "artifacts/prepared-production/resources/product.cat"',
  );
  for (const signing of signingSteps) {
    assert.deepEqual(Object.keys(signing.env).sort(), [
      "ESIGNER_CREDENTIAL_ID",
      "ESIGNER_PASSWORD",
      "ESIGNER_TOTP_SECRET",
      "ESIGNER_USERNAME",
    ]);
    for (const [name, value] of Object.entries(signing.env))
      assert.equal(value, `\${{ secrets.${name} }}`);
    assert.doesNotMatch(signing.run, /\*|password|totp|credential|username/i);
  }
  for (const step of steps.filter((step) => !signingSteps.includes(step)))
    assert.doesNotMatch(JSON.stringify(step), /secrets\.ESIGNER_/);
});

test("unsigned results never become production artifacts without signing, verification and final root smoke", async () => {
  const { value } = await workflow("windows-signed-release.yml");
  const steps = value.jobs.sign.steps;
  const configuration = steps.find((step) => step.env?.SIGNING_READY);
  assert.match(configuration.run, /SIGNING NOT CONFIGURED/);
  assert.match(configuration.run, /SIGNING_READY -cne 'true'/);
  assert.match(configuration.run, /loadSigningConfig/);
  const signIndex = steps.findIndex((step) =>
    step.run?.includes("windows-signing.mjs sign "),
  );
  const stageIndex = steps.findIndex((step) =>
    step.run?.includes("stage-release.mjs --production --already-signed"),
  );
  const catalogSignIndex = steps.findIndex((step) =>
    step.run?.includes(
      'windows-signing.mjs sign-catalog "artifacts/prepared-production/resources/product.cat"',
    ),
  );
  const installIndex = steps.findIndex(
    (step) =>
      step.run ===
      "node scripts/install-signed-release.mjs artifacts/prepared-production",
  );
  const verifies = steps
    .map((step, index) => ({ step, index }))
    .filter(
      ({ step }) => step.run === "node scripts/distribution.mjs --production",
    );
  const smokeIndex = steps.findIndex((step) =>
    step.run?.includes("npm run test:desktop:release"),
  );
  const uploadIndex = steps.findIndex((step) =>
    step.uses?.startsWith("actions/upload-artifact@"),
  );
  assert.equal(verifies.length, 2);
  assert.ok(signIndex < stageIndex && stageIndex < catalogSignIndex);
  assert.ok(
    catalogSignIndex < installIndex && installIndex < verifies[0].index,
  );
  assert.ok(verifies[0].index < smokeIndex && smokeIndex < verifies[1].index);
  assert.ok(verifies[1].index < uploadIndex);
  assert.match(
    steps[stageIndex].run,
    /--source-exe "artifacts\/unsigned-windows\/PLC Lab\.exe"/,
  );
  assert.match(
    steps[stageIndex].run,
    /--prepare-only artifacts\/prepared-production$/,
  );
  assert.match(steps[smokeIndex].run, /test:release:relocated/);
  assert.match(steps[smokeIndex].run, /test:release:bat/);
  const upload = steps[uploadIndex];
  assert.equal(
    upload.if,
    undefined,
    "production upload requires all prior checks",
  );
  assert.match(upload.with.name, /SIGNED-production/);
  assert.match(upload.with.path, /^PLC Lab\.exe$/m);
  assert.match(upload.with.path, /^resources\/\*\*$/m);
  assert.doesNotMatch(upload.with.path, /target|Launcher|PLC Lab Desktop/);
  assert.ok(
    steps.some((step) =>
      step.run?.includes("SMART APP CONTROL DEVICE ACCEPTANCE: NOT VERIFIED"),
    ),
    "a hosted runner must not claim Smart App Control device acceptance",
  );
});
