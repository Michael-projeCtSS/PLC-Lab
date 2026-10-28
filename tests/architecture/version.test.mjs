import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { root } from "../../scripts/toolchain.mjs";

const read = (file) => readFileSync(path.join(root, file), "utf8");
const tracked = (pattern) =>
  execFileSync("git", ["ls-files", pattern], { cwd: root, encoding: "utf8" })
    .split(/\r?\n/)
    .filter(Boolean);

test("the product has one canonical version: the Cargo workspace version", () => {
  const version = /\[workspace\.package\][^[]*?\bversion\s*=\s*"([^"]+)"/s.exec(
    read("Cargo.toml"),
  )?.[1];
  assert.match(version ?? "", /^\d+\.\d+\.\d+(-[\w.]+)?$/);
  for (const manifest of tracked("*Cargo.toml").filter(
    (file) => file !== "Cargo.toml",
  ))
    assert.match(
      read(manifest),
      /^version\.workspace = true$/m,
      `${manifest} inherits the workspace version`,
    );
  for (const manifest of tracked("*package.json"))
    assert.equal(JSON.parse(read(manifest)).version, version, manifest);
  const lock = JSON.parse(read("package-lock.json"));
  assert.equal(lock.version, version, "package-lock.json");
  // Tauri falls back to the Cargo package version, which is the workspace version.
  assert.equal(
    JSON.parse(read("apps/desktop/src-tauri/tauri.conf.json")).version,
    undefined,
  );
});
