import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";

export const PRODUCTION = "SIGNED_PRODUCTION_RELEASE";
export const DEVELOPMENT = "UNSIGNED_DEVELOPMENT_BUILD";

export async function fileHash(file) {
  return createHash("sha256")
    .update(await readFile(file))
    .digest("hex");
}

/** Certificate-table presence is an overwrite guard, never a trust decision. */
export function hasCertificateTable(bytes) {
  if (bytes.length < 64 || bytes.toString("ascii", 0, 2) !== "MZ") return false;
  const pe = bytes.readUInt32LE(60);
  if (pe + 26 > bytes.length || bytes.readUInt32LE(pe) !== 0x4550) return false;
  const optional = pe + 24;
  const magic = bytes.readUInt16LE(optional);
  const table =
    optional +
    (magic === 0x20b ? 112 : magic === 0x10b ? 96 : bytes.length) +
    32;
  return table + 8 <= bytes.length && bytes.readUInt32LE(table + 4) > 0;
}

export async function assertDevelopmentMayReplace(productRoot) {
  const { default: path } = await import("node:path");
  try {
    const manifest = JSON.parse(
      await readFile(path.join(productRoot, "resources/product.json"), "utf8"),
    );
    if (manifest.releaseKind === PRODUCTION || manifest.signing)
      throw new Error(
        "A signed production release cannot be overwritten by an unsigned development build. Use a separate development checkout.",
      );
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  try {
    if (
      hasCertificateTable(await readFile(path.join(productRoot, "PLC Lab.exe")))
    )
      throw new Error(
        "A signed executable cannot be overwritten by an unsigned development build. Use a separate development checkout.",
      );
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
}

export function assertSigningRecord(manifest, evidence) {
  if (manifest.releaseKind !== PRODUCTION)
    throw new Error(
      "SIGNATURE VERIFICATION FAILED: distribution is not labeled SIGNED_PRODUCTION_RELEASE.",
    );
  if (
    !manifest.signing ||
    manifest.application?.sha256 !== evidence.sha256 ||
    manifest.signing.sha256 !== evidence.sha256
  )
    throw new Error(
      "SIGNATURE VERIFICATION FAILED: signed executable and product manifest SHA-256 disagree.",
    );
  for (const key of [
    "publisherSubject",
    "signerThumbprint",
    "rootThumbprint",
    "timestampSubject",
  ])
    if (!evidence[key] || manifest.signing[key] !== evidence[key])
      throw new Error(
        `SIGNATURE VERIFICATION FAILED: product manifest ${key} differs from verified Authenticode evidence.`,
      );
}
