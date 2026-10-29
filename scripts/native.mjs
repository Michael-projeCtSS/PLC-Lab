import { spawnSync } from "node:child_process";
import path from "node:path";
import { root, toolchainEnvironment } from "./toolchain.mjs";

try {
  const result = spawnSync(
    process.execPath,
    [
      path.join(root, "node_modules", "@tauri-apps", "cli", "tauri.js"),
      ...process.argv.slice(2),
    ],
    {
      cwd: path.join(root, "apps", "desktop"),
      env: toolchainEnvironment(),
      stdio: "inherit",
    },
  );
  if (result.error) throw result.error;
  process.exitCode = result.status ?? 1;
} catch (error) {
  console.error(`Desktop command failed: ${error.message}`);
  process.exitCode = 1;
}
