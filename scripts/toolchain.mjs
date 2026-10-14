import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

export const root = fileURLToPath(new URL("../", import.meta.url));

export function toolchainEnvironment() {
  const environment = { ...process.env };
  const cargoHome = path.join(root, ".tools", "cargo");
  const cargoBin = path.join(cargoHome, "bin");
  const pathKey =
    Object.keys(environment).find((key) => key.toUpperCase() === "PATH") ??
    "PATH";
  const cargoName = process.platform === "win32" ? "cargo.exe" : "cargo";
  if (existsSync(path.join(cargoBin, cargoName))) {
    environment.CARGO_HOME = cargoHome;
    environment.RUSTUP_HOME = path.join(root, ".tools", "rustup");
    environment[pathKey] =
      `${cargoBin}${path.delimiter}${environment[pathKey] ?? ""}`;
  }
  return environment;
}
