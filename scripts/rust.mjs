import { spawnSync } from "node:child_process";
import path from "node:path";
import { root, selectToolchain } from "./toolchain.mjs";

try {
  const toolchain = selectToolchain();
  const result = spawnSync(
    path.join(
      toolchain.bin,
      process.platform === "win32" ? "cargo.exe" : "cargo",
    ),
    process.argv.slice(2),
    {
      cwd: root,
      env: toolchain.environment,
      stdio: "inherit",
    },
  );
  if (result.error) throw result.error;
  process.exitCode = result.status ?? 1;
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
