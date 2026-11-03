// SSL.com CodeSignTool integration. Provider output and credential arguments never
// enter logs; Windows Authenticode verification remains in windows-signing.mjs.
import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import {
  copyFile,
  lstat,
  mkdir,
  mkdtemp,
  readFile,
  rename,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { root } from "./toolchain.mjs";

const execute = promisify(execFile);
export const ESIGNER_PACKAGE = Object.freeze({
  version: "1.3.2",
  url: "https://github.com/SSLcom/CodeSignTool/releases/download/v1.3.2/CodeSignTool-v1.3.2.zip",
  sha256: "f14b1e1ef14bfa1fd00279c363aab0debbf5dcfba0e4bcdce5d22bb771de0e3a",
  jar: "jar/code_sign_tool-1.3.2.jar",
  jarSha256: "312a4a00d21117fadbe1b876cdef8075df94a9d197328b46b7e168b5a5179bb8",
  properties: "conf/code_sign_tool.properties",
  propertiesSha256:
    "3068bc196b30e05c79d117ece98ea66bca043c839c6dfdea884efd5fe5301bb8",
});
const directory = path.join(
  root,
  ".tools",
  `esigner-v${ESIGNER_PACKAGE.version}`,
);
const secretNames = [
  "ESIGNER_USERNAME",
  "ESIGNER_PASSWORD",
  "ESIGNER_CREDENTIAL_ID",
  "ESIGNER_TOTP_SECRET",
];

function fail(message) {
  throw new Error(`SIGNING NOT CONFIGURED: ${message}`);
}

export function esignerCredentials(environment = process.env) {
  for (const name of secretNames)
    if (
      typeof environment[name] !== "string" ||
      environment[name].length === 0 ||
      /[\0\r\n]/.test(environment[name])
    )
      fail(
        `set protected ${name}; credential values must be single-line secrets.`,
      );
  return Object.fromEntries(
    secretNames.map((name) => [name, environment[name]]),
  );
}

export function assertEsignerPackage(bytes) {
  if (
    createHash("sha256").update(bytes).digest("hex") !== ESIGNER_PACKAGE.sha256
  )
    fail(
      "SSL.com CodeSignTool package SHA-256 mismatch; no downloaded code was executed.",
    );
}

async function verifyClient() {
  for (const [name, expected] of [
    [ESIGNER_PACKAGE.jar, ESIGNER_PACKAGE.jarSha256],
    [ESIGNER_PACKAGE.properties, ESIGNER_PACKAGE.propertiesSha256],
  ]) {
    const file = path.join(directory, name);
    try {
      if (!(await lstat(file)).isFile())
        fail("CodeSignTool files must be regular files.");
      const actual = createHash("sha256")
        .update(await readFile(file))
        .digest("hex");
      if (actual !== expected)
        fail(
          "installed CodeSignTool files differ from the pinned provider package.",
        );
    } catch (error) {
      if (error.code === "ENOENT")
        fail(
          "run node scripts/esigner.mjs install to install the pinned signing client.",
        );
      throw error;
    }
  }
}

export async function installEsigner() {
  if (process.platform !== "win32")
    fail("CodeSignTool installation is supported on Windows only.");
  try {
    await stat(directory);
    await verifyClient();
    return {
      status: "Installed",
      version: ESIGNER_PACKAGE.version,
      packageSha256: ESIGNER_PACKAGE.sha256,
    };
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  const parent = path.join(root, ".tools");
  await mkdir(parent, { recursive: true });
  const temporary = await mkdtemp(path.join(parent, ".esigner-install-"));
  if (!temporary.startsWith(`${parent}${path.sep}`))
    throw new Error("Unsafe signing client cleanup path.");
  try {
    const archive = path.join(temporary, "client.zip");
    try {
      // Windows' HTTP client uses the normal OS trust store, including a managed
      // network's legitimate TLS inspection CA. Certificate checks stay enabled.
      await execute(
        "powershell.exe",
        [
          "-NoProfile",
          "-NonInteractive",
          "-Command",
          "$ErrorActionPreference='Stop'; $ProgressPreference='SilentlyContinue'; Invoke-WebRequest -UseBasicParsing -Uri $env:PLC_ESIGNER_URL -OutFile $env:PLC_ESIGNER_ARCHIVE -TimeoutSec 60",
        ],
        {
          env: {
            ...process.env,
            PLC_ESIGNER_URL: ESIGNER_PACKAGE.url,
            PLC_ESIGNER_ARCHIVE: archive,
          },
          windowsHide: true,
          timeout: 65_000,
        },
      );
    } catch {
      fail(
        "the official CodeSignTool package could not be downloaded; check network access and retry.",
      );
    }
    const bytes = await readFile(archive);
    assertEsignerPackage(bytes);
    const prepared = path.join(temporary, "prepared");
    await mkdir(prepared);
    // Extract only two known members after validating the complete archive hash.
    const extraction = [
      "$ErrorActionPreference='Stop'",
      "Add-Type -AssemblyName System.IO.Compression.FileSystem",
      "$zip=[IO.Compression.ZipFile]::OpenRead($env:PLC_ESIGNER_ARCHIVE)",
      "try { foreach($name in @('jar/code_sign_tool-1.3.2.jar','conf/code_sign_tool.properties')) {",
      "$entry=$zip.GetEntry($name); if(!$entry) { throw 'Missing pinned package member' }",
      "$target=Join-Path $env:PLC_ESIGNER_DIRECTORY $name",
      "[void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($target))",
      "[IO.Compression.ZipFileExtensions]::ExtractToFile($entry,$target,$false)",
      "} } finally { $zip.Dispose() }",
    ].join("\n");
    try {
      await execute(
        "powershell.exe",
        ["-NoProfile", "-NonInteractive", "-Command", extraction],
        {
          env: {
            ...process.env,
            PLC_ESIGNER_ARCHIVE: archive,
            PLC_ESIGNER_DIRECTORY: prepared,
          },
          windowsHide: true,
          timeout: 60_000,
        },
      );
    } catch {
      fail("the pinned CodeSignTool package could not be extracted.");
    }
    await rename(prepared, directory);
    await verifyClient();
    return {
      status: "Installed",
      version: ESIGNER_PACKAGE.version,
      packageSha256: ESIGNER_PACKAGE.sha256,
    };
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}

function javaEnvironment(environment = process.env) {
  const clean = { ...environment };
  for (const name of [
    "JAVA_TOOL_OPTIONS",
    "_JAVA_OPTIONS",
    "JDK_JAVA_OPTIONS",
    "CLASSPATH",
    ...secretNames,
  ])
    delete clean[name];
  return clean;
}

async function javaExecutable() {
  const home = process.env.JAVA_HOME_17_X64 || process.env.JAVA_HOME;
  const java = home ? path.join(home, "bin", "java.exe") : "java.exe";
  try {
    const result = await execute(java, ["-version"], {
      env: javaEnvironment(),
      windowsHide: true,
      timeout: 15_000,
    });
    const version = /version "(\d+)/.exec(
      `${result.stderr}\n${result.stdout}`,
    )?.[1];
    if (!version || Number(version) < 17)
      fail(
        "install Java 17 or later and set JAVA_HOME; hosted Windows runners provide JAVA_HOME_17_X64.",
      );
  } catch {
    fail(
      "Java 17 or later is unavailable or blocked; install a trusted Java runtime and set JAVA_HOME.",
    );
  }
  return java;
}

async function withClientContext(work) {
  const temporary = await mkdtemp(path.join(tmpdir(), "plc-esigner-"));
  if (!temporary.startsWith(`${tmpdir()}${path.sep}`))
    throw new Error("Unsafe signing cleanup path.");
  try {
    const configuration = path.join(temporary, "conf");
    await mkdir(configuration);
    await copyFile(
      path.join(directory, ESIGNER_PACKAGE.properties),
      path.join(configuration, "code_sign_tool.properties"),
    );
    await writeFile(
      path.join(configuration, "log4j2.xml"),
      '<Configuration status="OFF"><Loggers><Root level="OFF"/></Loggers></Configuration>',
    );
    return await work({
      temporary,
      java: await javaExecutable(),
      arguments: [
        "-Xmx1024M",
        `-Dlog4j2.configurationFile=${path.join(configuration, "log4j2.xml")}`,
        "-Dlog4j2.disableJmx=true",
        "-jar",
        path.join(directory, ESIGNER_PACKAGE.jar),
      ],
      options: {
        cwd: temporary,
        env: { ...javaEnvironment(), CODE_SIGN_TOOL_PATH: temporary },
        windowsHide: true,
        timeout: 180_000,
        maxBuffer: 1024 * 1024,
      },
    });
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}

/** Start the genuine pinned provider CLI without credentials or signing requests. */
export async function esignerClientCheck() {
  if (process.platform !== "win32")
    fail("the CodeSignTool client check requires Windows.");
  await verifyClient();
  await withClientContext(async (context) => {
    try {
      const result = await execute(
        context.java,
        [...context.arguments, "--help"],
        context.options,
      );
      const output = `${result.stdout}\n${result.stderr}`;
      if (
        !/Usage:\s*CodeSignTool/i.test(output) ||
        !/credential_info/.test(output) ||
        !/batch_sign/.test(output) ||
        /Exception|NoSuchFile|could not|not found|missing.*(?:config|properties)/i.test(
          output,
        )
      )
        throw new Error("Unexpected provider client help output");
    } catch {
      fail(
        "the pinned CodeSignTool JAR could not start with its production configuration; check the installed Java runtime and provider client.",
      );
    }
  });
  return {
    status: "ClientCheckPassed",
    clientVersion: ESIGNER_PACKAGE.version,
    credentialsUsed: false,
    signingRequested: false,
  };
}

export async function esignerPreflight() {
  esignerCredentials();
  if (process.platform !== "win32")
    fail("production signing and Authenticode verification require Windows.");
  await verifyClient();
  await javaExecutable();
  return {
    status: "Configured",
    provider: "ssl-esigner",
    clientVersion: ESIGNER_PACKAGE.version,
  };
}

/** Sign one intermediate artifact. Caller must validate it with Windows before staging. */
export async function esignerSign(file) {
  await esignerPreflight();
  const credentials = esignerCredentials();
  file = path.resolve(file);
  if (!/\.(exe|dll|cat)$/i.test(file))
    fail(
      "only the application, shipped DLLs and manifest catalog may be signed.",
    );
  await withClientContext(async (context) => {
    const { temporary, java } = context;
    const input = path.join(temporary, "input");
    const output = path.join(temporary, "signed");
    await Promise.all([input, output].map((item) => mkdir(item)));
    await copyFile(file, path.join(input, path.basename(file)));
    const args = [
      ...context.arguments,
      "sign",
      `-username=${credentials.ESIGNER_USERNAME}`,
      `-password=${credentials.ESIGNER_PASSWORD}`,
      `-credential_id=${credentials.ESIGNER_CREDENTIAL_ID}`,
      `-totp_secret=${credentials.ESIGNER_TOTP_SECRET}`,
      `-input_file_path=${path.join(input, path.basename(file))}`,
      `-output_dir_path=${output}`,
    ];
    try {
      // execFile passes an argument array directly: no shell/BAT expansion or
      // command printing. Provider diagnostics may contain credentials, so discard
      // stdout, stderr, original Error and cause on every failure.
      await execute(java, args, context.options);
      const signed = path.join(output, path.basename(file));
      if (!(await stat(signed)).isFile())
        throw new Error("Missing signed output");
      await copyFile(signed, file);
    } catch {
      throw new Error(
        "SIGNING FAILED: eSigner did not produce the signed artifact. Check the issued credential, TOTP configuration, cloud signing quota and provider status; no raw credential diagnostics are logged.",
      );
    }
  });
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  try {
    const operations = {
      install: installEsigner,
      doctor: esignerClientCheck,
      preflight: esignerPreflight,
    };
    if (!operations[process.argv[2]])
      throw new Error(
        "Usage: node scripts/esigner.mjs install|doctor|preflight",
      );
    console.log(JSON.stringify(await operations[process.argv[2]](), null, 2));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
