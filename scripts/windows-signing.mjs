// Authenticode is verified by Windows and SignTool, never by trusting JSON evidence.
import { execFile } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execute = promisify(execFile);
const implementation = fileURLToPath(
  new URL("./windows-signing.ps1", import.meta.url),
);
const endpoints = new Set(
  [
    "brs",
    "cus",
    "eus",
    "jpe",
    "krc",
    "ncus",
    "neu",
    "plc",
    "scus",
    "swn",
    "wcus",
    "weu",
    "wus",
    "wus2",
    "wus3",
  ].map((region) => `https://${region}.codesigning.azure.net`),
);

function configured(condition, message) {
  if (!condition) throw new Error(`SIGNING NOT CONFIGURED: ${message}`);
}

function fingerprint(value) {
  return (
    typeof value === "string" &&
    /^[A-Fa-f0-9]{40}$/.test(value) &&
    !/^0+$/.test(value)
  );
}

/** Nonsecret configuration only. Private keys, PFX passwords and tokens are not accepted. */
export function validateSigningConfig(input, directory = process.cwd()) {
  configured(
    input && typeof input === "object" && !Array.isArray(input),
    "provide a signing configuration object.",
  );
  const allowed = [
    "provider",
    "publisherSubject",
    "trustedRootThumbprints",
    "signtoolPath",
    "azure",
    "certificateStore",
  ];
  configured(
    Object.keys(input).every((key) => allowed.includes(key)),
    "unknown configuration field; credentials do not belong in this file.",
  );
  configured(
    ["azure-artifact-signing", "certificate-store", "ssl-esigner"].includes(
      input.provider,
    ),
    "select ssl-esigner, azure-artifact-signing or certificate-store.",
  );
  configured(
    typeof input.publisherSubject === "string" &&
      input.publisherSubject.includes("CN=") &&
      !/REPLACE|YOUR|[<>\r\n]/i.test(input.publisherSubject),
    "set the exact verified certificate publisherSubject.",
  );
  configured(
    Array.isArray(input.trustedRootThumbprints) &&
      input.trustedRootThumbprints.length > 0 &&
      input.trustedRootThumbprints.every(fingerprint),
    "pin trusted public CA root SHA-1 certificate thumbprints from the provider's official PKI documentation.",
  );
  const output = {
    ...input,
    trustedRootThumbprints: input.trustedRootThumbprints.map((value) =>
      value.toUpperCase(),
    ),
  };
  if (input.signtoolPath) {
    configured(
      typeof input.signtoolPath === "string",
      "signtoolPath must be a filename.",
    );
    output.signtoolPath = path.resolve(directory, input.signtoolPath);
  }
  if (input.provider === "azure-artifact-signing") {
    configured(
      !input.certificateStore,
      "certificateStore is only supported for the certificate-store provider.",
    );
    const azure = input.azure;
    configured(
      azure &&
        Object.keys(azure).every((key) =>
          [
            "endpoint",
            "accountName",
            "certificateProfileName",
            "dlibPath",
          ].includes(key),
        ),
      "configure the nonsecret Azure account/profile fields.",
    );
    const endpoint =
      typeof azure.endpoint === "string"
        ? azure.endpoint.replace(/\/$/, "")
        : undefined;
    configured(
      endpoints.has(endpoint),
      "use an official regional Artifact Signing endpoint matching the account region.",
    );
    for (const key of ["accountName", "certificateProfileName"])
      configured(
        typeof azure[key] === "string" &&
          /^[a-zA-Z0-9-]{3,64}$/.test(azure[key]) &&
          !/REPLACE|YOUR/i.test(azure[key]),
        `set Azure ${key}.`,
      );
    output.azure = { ...azure, endpoint };
    if (azure.dlibPath) {
      configured(
        typeof azure.dlibPath === "string",
        "dlibPath must be a filename.",
      );
      output.azure.dlibPath = path.resolve(directory, azure.dlibPath);
    }
  } else if (input.provider === "certificate-store") {
    configured(
      !input.azure,
      "azure is only supported for the azure-artifact-signing provider.",
    );
    const store = input.certificateStore;
    configured(
      store &&
        Object.keys(store).every((key) =>
          ["thumbprint", "storeLocation", "timestampUrl"].includes(key),
        ),
      "configure the nonsecret certificateStore fields.",
    );
    configured(
      fingerprint(store.thumbprint),
      "select the exact CA-issued signing certificate thumbprint.",
    );
    configured(
      ["CurrentUser", "LocalMachine"].includes(store.storeLocation),
      "certificate storeLocation must be CurrentUser or LocalMachine.",
    );
    let timestamp;
    try {
      timestamp = new URL(store.timestampUrl);
    } catch {
      /* Report the configuration error below. */
    }
    configured(
      timestamp &&
        ["http:", "https:"].includes(timestamp.protocol) &&
        !timestamp.username &&
        !timestamp.password &&
        !timestamp.search &&
        !timestamp.hash &&
        !/localhost|REPLACE|YOUR/i.test(timestamp.hostname),
      "set the CA's documented RFC 3161 timestampUrl without credentials.",
    );
    output.certificateStore = {
      ...store,
      thumbprint: store.thumbprint.toUpperCase(),
    };
  } else {
    configured(
      !input.azure && !input.certificateStore,
      "ssl-esigner verification uses only public publisher/root settings; credentials belong in protected environment variables.",
    );
  }
  return output;
}

export async function loadSigningConfig(
  file = process.env.PLC_LAB_SIGNING_CONFIG,
) {
  configured(
    typeof file === "string" && file.length > 0,
    "set PLC_LAB_SIGNING_CONFIG to your nonsecret provider JSON; see docs/WINDOWS_CODE_SIGNING.md.",
  );
  let source;
  try {
    source = await readFile(file, "utf8");
  } catch {
    throw new Error(
      "SIGNING NOT CONFIGURED: the configured provider JSON cannot be read.",
    );
  }
  let parsed;
  try {
    parsed = JSON.parse(source);
  } catch {
    throw new Error("SIGNING NOT CONFIGURED: the provider JSON is invalid.");
  }
  return validateSigningConfig(parsed, path.dirname(path.resolve(file)));
}

async function invoke(action, file, { config } = {}) {
  const settings = config
    ? validateSigningConfig(config)
    : await loadSigningConfig();
  configured(
    process.platform === "win32",
    "Windows Authenticode and Windows SDK SignTool are required.",
  );
  const temporary = await mkdtemp(path.join(tmpdir(), "plc-lab-signing-"));
  const configFile = path.join(temporary, "provider.json");
  try {
    await writeFile(configFile, JSON.stringify(settings), { flag: "wx" });
    const args = [
      "-NoLogo",
      "-NoProfile",
      "-NonInteractive",
      "-File",
      implementation,
      "-Action",
      action,
      "-ConfigPath",
      configFile,
    ];
    if (file) args.push("-FilePath", path.resolve(file));
    const { stdout } = await execute("powershell.exe", args, {
      encoding: "utf8",
      windowsHide: true,
      timeout: 180_000,
      maxBuffer: 1024 * 1024,
    });
    return JSON.parse(stdout.replace(/^\uFEFF/, "").trim());
  } catch (error) {
    // Provider diagnostics can contain identity/token details. Emit only our controlled error lines.
    const controlled = `${error.stderr ?? ""}\n${error.stdout ?? ""}`.match(
      /(?:SIGNING NOT CONFIGURED|SIGNATURE VERIFICATION FAILED|SIGNING FAILED):[^\r\n]*/,
    )?.[0];
    // eslint-disable-next-line preserve-caught-error -- Raw provider output can contain credentials and must not survive in an error cause.
    throw new Error(
      controlled ??
        `${action === "Preflight" ? "SIGNING NOT CONFIGURED" : "SIGNATURE VERIFICATION FAILED"}: Windows signing command failed; check Windows SDK/client installation and docs/WINDOWS_CODE_SIGNING.md.`,
    );
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}

export async function preflightSigning(options) {
  const config = options?.config
    ? validateSigningConfig(options.config)
    : await loadSigningConfig();
  if (config.provider === "ssl-esigner") {
    const { esignerPreflight } = await import("./esigner.mjs");
    await esignerPreflight();
  }
  return invoke("Preflight", undefined, { config });
}
export async function signExecutable(file, options) {
  const config = options?.config
    ? validateSigningConfig(options.config)
    : await loadSigningConfig();
  if (config.provider === "ssl-esigner") {
    const { esignerSign } = await import("./esigner.mjs");
    await esignerSign(file);
    return verifyExecutable(file, { config });
  }
  return invoke("Sign", file, { config });
}
export async function verifyExecutable(file, options) {
  return invoke("Verify", file, options);
}
export async function signCatalog(file, options) {
  const config = options?.config
    ? validateSigningConfig(options.config)
    : await loadSigningConfig();
  if (config.provider === "ssl-esigner") {
    const { esignerSign } = await import("./esigner.mjs");
    await esignerSign(file);
    return verifyCatalog(file, { config });
  }
  return invoke("SignCatalog", file, { config });
}
export async function verifyCatalog(file, options) {
  return invoke("VerifyCatalog", file, options);
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const [action, file] = process.argv.slice(2);
  try {
    const operations = {
      preflight: preflightSigning,
      sign: () => signExecutable(file),
      verify: () => verifyExecutable(file),
      "sign-catalog": () => signCatalog(file),
      "verify-catalog": () => verifyCatalog(file),
    };
    if (!operations[action] || (action !== "preflight" && !file))
      throw new Error(
        "Usage: node scripts/windows-signing.mjs preflight|sign <exe>|verify <exe>|sign-catalog <cat>|verify-catalog <cat>",
      );
    console.log(JSON.stringify(await operations[action](), null, 2));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
