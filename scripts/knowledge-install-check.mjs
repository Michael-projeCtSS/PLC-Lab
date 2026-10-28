// Installed knowledge pack check.
//
//   node scripts/knowledge-install-check.mjs [zipFile]
//
// Extracts the release archive into a fresh temporary folder whose path contains spaces,
// validates the extracted files against the pack's schemas, then lets the application
// discover, index and search that folder (tests/conformance/tests/knowledge_install.rs
// through PLC_LAB_KNOWLEDGE_ROOT). The folder is removed afterwards.
import { spawnSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import os from "node:os";
import path from "node:path";
import {
  DEFAULT_PACK,
  DEFAULT_ZIP,
  PACK_NAME,
  listFiles,
  readZip,
  validatePack,
  verifyZip,
} from "./knowledge-pack.mjs";
import { root, toolchainEnvironment } from "./toolchain.mjs";

/** Write every archive entry below `target`, refusing names that would leave it. */
export function extractZip(buffer, target) {
  const entries = readZip(buffer);
  const base = path.resolve(target);
  for (const [name, data] of entries) {
    const destination = path.resolve(base, name);
    if (!destination.startsWith(base + path.sep))
      throw new Error(`Unsafe archive entry ${name}`);
    mkdirSync(path.dirname(destination), { recursive: true });
    writeFileSync(destination, data);
  }
  return entries.size;
}

const zipFile = path.resolve(process.argv[2] ?? DEFAULT_ZIP);
const archive = verifyZip(zipFile, DEFAULT_PACK);
if (archive.problems.length) {
  for (const p of archive.problems) console.error(p);
  console.error(
    "The archive does not match the source pack. Run: npm run knowledge:zip",
  );
  process.exit(1);
}
const install = mkdtempSync(path.join(os.tmpdir(), "PLC Lab knowledge check "));
let status = 1;
try {
  const extracted = extractZip(readFileSync(zipFile), install);
  const packDir = path.join(install, PACK_NAME);
  const files = listFiles(packDir);
  const { problems, items } = validatePack(files, (rel) =>
    readFileSync(path.join(packDir, rel), "utf8"),
  );
  for (const p of problems) console.error(p);
  console.log(
    `Extracted ${extracted} files to ${packDir}: ${items} items, ${problems.length} schema problem(s).`,
  );
  if (problems.length === 0) {
    const result = spawnSync(
      process.platform === "win32" ? "cargo.exe" : "cargo",
      [
        "test",
        "--locked",
        "-p",
        "plc-conformance",
        "--test",
        "knowledge_install",
        "--",
        "--nocapture",
      ],
      {
        cwd: root,
        env: { ...toolchainEnvironment(), PLC_LAB_KNOWLEDGE_ROOT: install },
        stdio: "inherit",
      },
    );
    if (result.error) console.error(result.error.message);
    status = result.status ?? 1;
  }
} finally {
  rmSync(install, { recursive: true, force: true });
}
console.log(
  status === 0
    ? "Installed knowledge pack check PASS."
    : "Installed knowledge pack check FAILED.",
);
process.exit(status);
