// The user distribution: the application executable `PLC Lab.exe` in the product root,
// the data it ships with in `resources/` (knowledge packs and `product.json`), and the
// optional RUN_PLC_LAB.bat fallback. Exactly one application executable (ADR-0030).
//
//   node scripts/distribution.mjs [productRoot]   verify a staged distribution
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { readFile, readdir, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { root } from "./toolchain.mjs";

export const APPLICATION = "PLC Lab.exe";
export const RESOURCES = "resources";
export const PRODUCT_MANIFEST = "product.json";
export const FALLBACK_BAT = "RUN_PLC_LAB.bat";
/** Entry points of earlier distributions that must not remain beside the application. */
export const LEGACY_FILES = [
  "Launcher.exe",
  "PLC Lab Desktop.exe",
  "PLC-Lab.exe",
  "plc-lab-desktop.exe",
  "plc-lab-launcher.exe",
];
export const LEGACY_DIRS = [path.join("releases", "current")];

async function hash(file) {
  return createHash("sha256")
    .update(await readFile(file))
    .digest("hex");
}

async function filesBelow(dir, prefix = "") {
  const out = [];
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const name = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isDirectory())
      out.push(...(await filesBelow(path.join(dir, entry.name), name)));
    else out.push(name);
  }
  return out;
}

/** The top-level entries a copy of the distribution needs, from its manifest. */
export async function distributionEntries(productRoot) {
  const manifest = JSON.parse(
    await readFile(path.join(productRoot, RESOURCES, PRODUCT_MANIFEST), "utf8"),
  );
  return [
    APPLICATION,
    ...(manifest.dependencies ?? []).map((d) => d.name),
    RESOURCES,
    ...(existsSync(path.join(productRoot, FALLBACK_BAT)) ? [FALLBACK_BAT] : []),
  ];
}

/** Problems of the staged distribution in `productRoot`; empty when it is complete. */
export async function verifyDistribution(productRoot) {
  const problems = [];
  const application = path.join(productRoot, APPLICATION);
  const manifestFile = path.join(productRoot, RESOURCES, PRODUCT_MANIFEST);
  if (!existsSync(application) || !(await stat(application)).isFile())
    return [`${APPLICATION} is missing in ${productRoot}`];
  if (!existsSync(manifestFile))
    return [`${RESOURCES}/${PRODUCT_MANIFEST} is missing`];
  const manifest = JSON.parse(await readFile(manifestFile, "utf8"));
  if (manifest.application?.name !== APPLICATION)
    problems.push(`${PRODUCT_MANIFEST} does not name ${APPLICATION}`);
  else if (manifest.application.sha256 !== (await hash(application)))
    problems.push(`${APPLICATION} differs from the staged build`);
  for (const dependency of manifest.dependencies ?? []) {
    const file = path.join(productRoot, dependency.name);
    if (!existsSync(file)) problems.push(`${dependency.name} is missing`);
    else if (dependency.sha256 !== (await hash(file)))
      problems.push(`${dependency.name} differs from the staged build`);
  }
  for (const record of manifest.files ?? []) {
    const file = path.join(productRoot, RESOURCES, ...record.name.split("/"));
    if (!existsSync(file))
      problems.push(`${RESOURCES}/${record.name} is missing`);
    else if (record.sha256 !== (await hash(file)))
      problems.push(`${RESOURCES}/${record.name} differs from the manifest`);
  }
  if (
    !(manifest.files ?? []).some((r) =>
      /^knowledge-packs\/[^/]+\/pack\.json$/.test(r.name),
    )
  )
    problems.push("no knowledge pack is staged in resources/knowledge-packs");
  // Exactly one application executable in everything the distribution consists of.
  const shipped = [
    APPLICATION,
    ...(manifest.dependencies ?? []).map((d) => d.name),
    ...(await filesBelow(path.join(productRoot, RESOURCES))).map(
      (name) => `${RESOURCES}/${name}`,
    ),
  ];
  const executables = shipped.filter((name) => /\.exe$/i.test(name));
  if (executables.length !== 1 || executables[0] !== APPLICATION)
    problems.push(
      `the distribution must contain exactly one application executable (${APPLICATION}), found: ${executables.join(", ")}`,
    );
  for (const legacy of LEGACY_FILES)
    if (existsSync(path.join(productRoot, legacy)))
      problems.push(`${legacy} from an earlier distribution is still present`);
  for (const legacy of LEGACY_DIRS)
    if (existsSync(path.join(productRoot, legacy)))
      problems.push(
        `${legacy.split(path.sep).join("/")} from an earlier distribution is still present`,
      );
  const bat = path.join(productRoot, FALLBACK_BAT);
  if (existsSync(bat)) {
    const text = await readFile(bat, "utf8");
    if (!text.includes(`%~dp0${APPLICATION}`))
      problems.push(`${FALLBACK_BAT} does not start ${APPLICATION}`);
    if (/releases|Launcher|PLC Lab Desktop|npm|cargo|node/i.test(text))
      problems.push(
        `${FALLBACK_BAT} refers to something other than ${APPLICATION}`,
      );
  }
  return problems;
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const productRoot = path.resolve(process.argv[2] ?? root);
  const problems = await verifyDistribution(productRoot);
  if (problems.length) {
    console.error(
      `Distribution check FAILED for ${productRoot}:\n- ${problems.join("\n- ")}`,
    );
    process.exitCode = 1;
  } else {
    const entries = await distributionEntries(productRoot);
    console.log(
      `Distribution check PASS: ${productRoot} (${entries.join(", ")}; one application executable: ${APPLICATION})`,
    );
  }
}
