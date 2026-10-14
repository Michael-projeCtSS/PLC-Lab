import { spawnSync } from "node:child_process";
import path from "node:path";
import { root, toolchainEnvironment } from "./toolchain.mjs";

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
if (result.error)
  console.error(`Desktop command failed: ${result.error.message}`);
process.exit(result.status ?? 1);
