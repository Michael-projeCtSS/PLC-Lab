// Verify an extracted CI production artifact before replacing the local root product.
import { constants } from "node:fs";
import {
  cp,
  copyFile,
  mkdir,
  mkdtemp,
  readFile,
  rename,
  rm,
} from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  APPLICATION,
  distributionEntries,
  LEGACY_DIRS,
  LEGACY_FILES,
  RESOURCES,
  verifyDistribution,
} from "./distribution.mjs";
import {
  assertDevelopmentMayReplace,
  DEVELOPMENT,
  fileHash,
} from "./release-trust.mjs";
import { loadSigningConfig } from "./windows-signing.mjs";
import { root } from "./toolchain.mjs";

export async function installPreparedDistribution(
  sourceRoot,
  productRoot = root,
  { production = true, config } = {},
) {
  sourceRoot = path.resolve(sourceRoot);
  productRoot = path.resolve(productRoot);
  if (
    sourceRoot === productRoot ||
    sourceRoot === path.join(productRoot, RESOURCES) ||
    sourceRoot.startsWith(`${path.join(productRoot, RESOURCES)}${path.sep}`)
  )
    throw new Error(
      "Extract the signed release to a separate directory before installation.",
    );
  if (production) config ??= await loadSigningConfig();
  else await assertDevelopmentMayReplace(productRoot);
  const verify = async (directory) => {
    const problems = await verifyDistribution(directory, {
      production,
      config,
    });
    if (problems.length)
      throw new Error(`SIGNATURE VERIFICATION FAILED: ${problems.join("; ")}`);
    if (!production) {
      const manifest = JSON.parse(
        await readFile(path.join(directory, RESOURCES, "product.json"), "utf8"),
      );
      if (manifest.releaseKind !== DEVELOPMENT)
        throw new Error(
          "Unsigned staging requires an explicit UNSIGNED_DEVELOPMENT_BUILD manifest.",
        );
    }
  };
  await verify(sourceRoot);
  await mkdir(productRoot, { recursive: true });
  const transaction = await mkdtemp(path.join(productRoot, ".staging-signed-"));
  if (!transaction.startsWith(`${productRoot}${path.sep}`))
    throw new Error("Unsafe installation staging path.");
  const prepared = path.join(transaction, "prepared");
  const backup = path.join(transaction, "backup");
  await mkdir(prepared);
  await mkdir(backup);
  const moved = [];
  const installed = [];
  let rollbackFailed = false;
  try {
    const entries = await distributionEntries(sourceRoot);
    for (const name of entries) {
      const source = path.join(sourceRoot, name);
      const target = path.join(prepared, name);
      if (name === RESOURCES) {
        await cp(source, target, {
          recursive: true,
          errorOnExist: true,
          force: false,
          filter: async (entry) => {
            const { lstat } = await import("node:fs/promises");
            if ((await lstat(entry)).isSymbolicLink())
              throw new Error(
                "Signed distribution must not contain symbolic links.",
              );
            return true;
          },
        });
      } else await copyFile(source, target, constants.COPYFILE_EXCL);
    }
    await verify(prepared);
    const expectedHash = await fileHash(path.join(prepared, APPLICATION));
    const retired = [...LEGACY_FILES, ...LEGACY_DIRS];
    try {
      const previous = JSON.parse(
        await readFile(
          path.join(productRoot, RESOURCES, "product.json"),
          "utf8",
        ),
      );
      for (const dependency of previous.dependencies ?? []) {
        if (
          typeof dependency.name !== "string" ||
          !/^[^\\/:]+\.dll$/i.test(dependency.name) ||
          entries.includes(dependency.name)
        )
          continue;
        const oldFile = path.join(productRoot, dependency.name);
        try {
          if ((await fileHash(oldFile)) === dependency.sha256)
            retired.push(dependency.name);
        } catch (error) {
          if (error.code !== "ENOENT") throw error;
        }
      }
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
    for (const name of retired) {
      const target = path.resolve(productRoot, name);
      if (!target.startsWith(`${productRoot}${path.sep}`))
        throw new Error("Unsafe retired distribution path.");
      await mkdir(path.dirname(path.join(backup, name)), { recursive: true });
      try {
        await rename(target, path.join(backup, name));
        moved.push(name);
      } catch (error) {
        if (error.code !== "ENOENT") throw error;
      }
    }
    for (const name of entries) {
      try {
        await rename(path.join(productRoot, name), path.join(backup, name));
        moved.push(name);
      } catch (error) {
        if (error.code !== "ENOENT") throw error;
      }
      await rename(path.join(prepared, name), path.join(productRoot, name));
      installed.push(name);
    }
    await verify(productRoot);
    if ((await fileHash(path.join(productRoot, APPLICATION))) !== expectedHash)
      throw new Error(
        "Installed root executable changed after signature verification.",
      );
    const manifest = JSON.parse(
      await readFile(path.join(productRoot, RESOURCES, "product.json"), "utf8"),
    );
    console.log(
      `${manifest.releaseKind} installed: ${path.join(productRoot, APPLICATION)} (${manifest.application.sha256})`,
    );
  } catch (error) {
    try {
      for (const name of installed.reverse())
        await rename(path.join(productRoot, name), path.join(prepared, name));
      for (const name of moved.reverse())
        await rename(path.join(backup, name), path.join(productRoot, name));
    } catch (rollbackError) {
      rollbackFailed = true;
      throw new Error(
        `Installation failed: ${error.message}. Recovery files retained at ${transaction}: ${rollbackError.message}`,
        { cause: rollbackError },
      );
    }
    throw error;
  } finally {
    if (!rollbackFailed)
      await rm(transaction, { recursive: true, force: true });
  }
}

export async function installSignedRelease(sourceRoot, productRoot = root) {
  return installPreparedDistribution(sourceRoot, productRoot);
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  if (!process.argv[2]) {
    console.error(
      "Usage: node scripts/install-signed-release.mjs <extracted-signed-artifact-directory>",
    );
    process.exitCode = 1;
  } else {
    try {
      await installSignedRelease(process.argv[2]);
    } catch (error) {
      console.error(error.message);
      process.exitCode = 1;
    }
  }
}
