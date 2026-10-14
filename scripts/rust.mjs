import { spawnSync } from "node:child_process";
import { root, toolchainEnvironment } from "./toolchain.mjs";

const result = spawnSync(
  process.platform === "win32" ? "cargo.exe" : "cargo",
  process.argv.slice(2),
  {
    cwd: root,
    env: toolchainEnvironment(),
    stdio: "inherit",
  },
);
if (result.error)
  console.error(
    `Rust toolchain unavailable: ${result.error.message}. See docs/DEVELOPMENT.md.`,
  );
process.exit(result.status ?? 1);
