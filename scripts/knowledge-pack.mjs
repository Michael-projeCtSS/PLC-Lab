// Knowledge pack tooling without third-party dependencies.
//
//   node scripts/knowledge-pack.mjs validate [packDir]
//   node scripts/knowledge-pack.mjs zip [packDir] [zipFile]
//   node scripts/knowledge-pack.mjs verify [zipFile] [packDir]
//
// `validate` checks every content file against the JSON Schemas in the pack's schemas/
// folder (a strict subset of JSON Schema 2020-12) and identifier uniqueness. The Rust
// loader (knowledge-core) remains the authoritative validator at runtime.
// `zip` writes a deterministic archive (sorted entries, fixed timestamps) whose root
// folder is the pack folder. `verify` extracts the archive in memory, checks that it
// contains exactly the pack's files byte for byte and validates the extracted content.
import { createHash } from "node:crypto";
import {
  mkdirSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { crc32, deflateRawSync, inflateRawSync } from "node:zlib";
import { root } from "./toolchain.mjs";

export const PACK_NAME = "PLC_Lab_Knowledge_Pack_v1";
export const DEFAULT_PACK = path.join(root, "knowledge", PACK_NAME);
export const DEFAULT_ZIP = path.join(
  root,
  "releases",
  "knowledge",
  `${PACK_NAME}.zip`,
);

export function listFiles(dir, base = dir) {
  const out = [];
  for (const entry of readdirSync(dir).sort()) {
    const full = path.join(dir, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) out.push(...listFiles(full, base));
    else out.push(path.relative(base, full).split(path.sep).join("/"));
  }
  return out;
}

// ---- minimal JSON Schema validator --------------------------------------------------
function typeOf(v) {
  if (v === null) return "null";
  if (Array.isArray(v)) return "array";
  if (typeof v === "number") return Number.isInteger(v) ? "integer" : "number";
  return typeof v;
}
function matchesType(v, t) {
  const actual = typeOf(v);
  return actual === t || (t === "number" && actual === "integer");
}
function resolve(schema, rootSchema) {
  if (!schema.$ref) return schema;
  if (!schema.$ref.startsWith("#/$defs/"))
    throw new Error(`Unsupported $ref ${schema.$ref}`);
  const target = rootSchema.$defs?.[schema.$ref.slice(8)];
  if (!target) throw new Error(`Unknown $ref ${schema.$ref}`);
  return target;
}
export function validate(value, schema, rootSchema = schema, at = "$") {
  const s = resolve(schema, rootSchema);
  const errors = [];
  if (s.oneOf) {
    const matching = s.oneOf.filter(
      (option) => validate(value, option, rootSchema, at).length === 0,
    );
    if (matching.length !== 1)
      errors.push(`${at}: matches ${matching.length} of the allowed shapes`);
    return errors;
  }
  if ("const" in s && value !== s.const)
    errors.push(`${at}: expected ${JSON.stringify(s.const)}`);
  if (s.enum && !s.enum.includes(value))
    errors.push(`${at}: expected one of ${s.enum.join(", ")}`);
  if (s.type) {
    const types = Array.isArray(s.type) ? s.type : [s.type];
    if (!types.some((t) => matchesType(value, t)))
      return [`${at}: expected ${types.join(" or ")}`];
  }
  if (typeof value === "string") {
    if (s.minLength && value.length < s.minLength)
      errors.push(`${at}: must not be empty`);
    if (s.pattern && !new RegExp(s.pattern).test(value))
      errors.push(`${at}: does not match ${s.pattern}`);
  }
  if (typeof value === "number") {
    if (s.minimum !== undefined && value < s.minimum)
      errors.push(`${at}: below ${s.minimum}`);
    if (s.maximum !== undefined && value > s.maximum)
      errors.push(`${at}: above ${s.maximum}`);
  }
  if (Array.isArray(value)) {
    if (s.minItems && value.length < s.minItems)
      errors.push(`${at}: needs at least ${s.minItems} items`);
    if (s.items)
      value.forEach((item, i) =>
        errors.push(...validate(item, s.items, rootSchema, `${at}[${i}]`)),
      );
  }
  if (typeOf(value) === "object" && (s.properties || s.required)) {
    for (const key of s.required ?? [])
      if (!(key in value)) errors.push(`${at}: missing ${key}`);
    for (const [key, v] of Object.entries(value)) {
      if (s.properties?.[key])
        errors.push(
          ...validate(v, s.properties[key], rootSchema, `${at}.${key}`),
        );
      else if (s.additionalProperties === false)
        errors.push(`${at}: unknown field ${key}`);
    }
  }
  return errors;
}

const SCHEMA_FOR = {
  fundamentals: "article",
  instructions: "article",
  vendors: "article",
  protocols: "article",
  troubleshooting: "article",
  lessons: "lesson",
  exercises: "exercise",
  scenes: "scene",
  sources: "sources",
};

/** Validate a pack given a function that returns file contents by relative path. */
export function validatePack(files, read) {
  const problems = [];
  const schema = (name) => JSON.parse(read(`schemas/${name}.schema.json`));
  const schemas = Object.fromEntries(
    ["pack", "article", "lesson", "exercise", "scene", "sources"].map((n) => [
      n,
      schema(n),
    ]),
  );
  problems.push(
    ...validate(JSON.parse(read("pack.json")), schemas.pack).map(
      (e) => `pack.json ${e}`,
    ),
  );
  const ids = new Map();
  const claim = (id, file) => {
    if (ids.has(id))
      problems.push(
        `${file}: identifier ${id} already defined in ${ids.get(id)}`,
      );
    ids.set(id, file);
  };
  for (const file of files) {
    const [top] = file.split("/");
    if (!file.endsWith(".json") || file === "pack.json" || top === "schemas")
      continue;
    const kind = SCHEMA_FOR[top];
    if (!kind) {
      problems.push(
        `${file}: content folder ${top} is not part of pack format 1`,
      );
      continue;
    }
    let data;
    try {
      data = JSON.parse(read(file));
    } catch (error) {
      problems.push(`${file}: ${error.message}`);
      continue;
    }
    problems.push(...validate(data, schemas[kind]).map((e) => `${file} ${e}`));
    if (kind === "sources")
      for (const s of data.sources ?? []) claim(s.id, file);
    else claim(data.id, file);
  }
  return { problems, items: ids.size };
}

// ---- deterministic zip --------------------------------------------------------------
const DOS_TIME = 0; // 00:00:00
const DOS_DATE = ((2026 - 1980) << 9) | (1 << 5) | 1; // 2026-01-01

export function buildZip(packDir, rootName = PACK_NAME) {
  const files = listFiles(packDir);
  const local = [];
  const central = [];
  let offset = 0;
  for (const rel of files) {
    const name = Buffer.from(`${rootName}/${rel}`, "utf8");
    const data = readFileSync(path.join(packDir, rel));
    const compressed = deflateRawSync(data, { level: 9 });
    const crc = crc32(data);
    const header = Buffer.alloc(30);
    header.writeUInt32LE(0x04034b50, 0);
    header.writeUInt16LE(20, 4);
    header.writeUInt16LE(0x0800, 6); // UTF-8 names
    header.writeUInt16LE(8, 8); // deflate
    header.writeUInt16LE(DOS_TIME, 10);
    header.writeUInt16LE(DOS_DATE, 12);
    header.writeUInt32LE(crc, 14);
    header.writeUInt32LE(compressed.length, 18);
    header.writeUInt32LE(data.length, 22);
    header.writeUInt16LE(name.length, 26);
    header.writeUInt16LE(0, 28);
    local.push(header, name, compressed);
    const entry = Buffer.alloc(46);
    entry.writeUInt32LE(0x02014b50, 0);
    entry.writeUInt16LE(20, 4);
    entry.writeUInt16LE(20, 6);
    entry.writeUInt16LE(0x0800, 8);
    entry.writeUInt16LE(8, 10);
    entry.writeUInt16LE(DOS_TIME, 12);
    entry.writeUInt16LE(DOS_DATE, 14);
    entry.writeUInt32LE(crc, 16);
    entry.writeUInt32LE(compressed.length, 20);
    entry.writeUInt32LE(data.length, 24);
    entry.writeUInt16LE(name.length, 28);
    entry.writeUInt32LE(offset, 42);
    central.push(entry, name);
    offset += header.length + name.length + compressed.length;
  }
  const centralBuffer = Buffer.concat(central);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(files.length, 8);
  end.writeUInt16LE(files.length, 10);
  end.writeUInt32LE(centralBuffer.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...local, centralBuffer, end]);
}

/** Extract every entry of a zip produced by any standard tool (stored or deflated). */
export function readZip(buffer) {
  const endAt = buffer.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]));
  if (endAt < 0) throw new Error("Not a zip archive.");
  const count = buffer.readUInt16LE(endAt + 10);
  let at = buffer.readUInt32LE(endAt + 16);
  const entries = new Map();
  for (let i = 0; i < count; i++) {
    if (buffer.readUInt32LE(at) !== 0x02014b50)
      throw new Error("Corrupt central directory.");
    const method = buffer.readUInt16LE(at + 10);
    const crc = buffer.readUInt32LE(at + 16);
    const size = buffer.readUInt32LE(at + 20);
    const nameLength = buffer.readUInt16LE(at + 28);
    const extra = buffer.readUInt16LE(at + 30);
    const comment = buffer.readUInt16LE(at + 32);
    const localAt = buffer.readUInt32LE(at + 42);
    const name = buffer.toString("utf8", at + 46, at + 46 + nameLength);
    if (name.includes("..") || path.isAbsolute(name))
      throw new Error(`Unsafe entry ${name}`);
    const dataAt =
      localAt +
      30 +
      buffer.readUInt16LE(localAt + 26) +
      buffer.readUInt16LE(localAt + 28);
    const raw = buffer.subarray(dataAt, dataAt + size);
    const data =
      method === 8
        ? inflateRawSync(raw)
        : method === 0
          ? Buffer.from(raw)
          : null;
    if (!data) throw new Error(`Unsupported compression in ${name}`);
    if (crc32(data) !== crc) throw new Error(`CRC mismatch in ${name}`);
    if (!name.endsWith("/")) entries.set(name, data);
    at += 46 + nameLength + extra + comment;
  }
  return entries;
}

export function verifyZip(zipFile, packDir) {
  const entries = readZip(readFileSync(zipFile));
  const expected = listFiles(packDir).map((f) => `${PACK_NAME}/${f}`);
  const problems = [];
  const names = [...entries.keys()].sort();
  if (JSON.stringify(names) !== JSON.stringify([...expected].sort()))
    problems.push("The archive does not contain exactly the pack's files.");
  for (const name of expected) {
    const data = entries.get(name);
    if (
      data &&
      !data.equals(
        readFileSync(path.join(packDir, name.slice(PACK_NAME.length + 1))),
      )
    )
      problems.push(`${name} differs from the source pack`);
  }
  const files = names
    .filter((n) => n.startsWith(`${PACK_NAME}/`))
    .map((n) => n.slice(PACK_NAME.length + 1));
  const result = validatePack(files, (rel) => {
    const data = entries.get(`${PACK_NAME}/${rel}`);
    if (!data) throw new Error(`missing ${rel}`);
    return data.toString("utf8");
  });
  return {
    problems: [...problems, ...result.problems],
    files: names.length,
    items: result.items,
  };
}

const sha256 = (buffer) => createHash("sha256").update(buffer).digest("hex");

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const [command, a, b] = process.argv.slice(2);
  if (command === "validate") {
    const dir = a ? path.resolve(a) : DEFAULT_PACK;
    const { problems, items } = validatePack(listFiles(dir), (rel) =>
      readFileSync(path.join(dir, rel), "utf8"),
    );
    for (const p of problems) console.error(p);
    console.log(`${items} items checked, ${problems.length} problem(s).`);
    process.exit(problems.length ? 1 : 0);
  } else if (command === "zip") {
    const dir = a ? path.resolve(a) : DEFAULT_PACK;
    const out = b ? path.resolve(b) : DEFAULT_ZIP;
    const zip = buildZip(dir);
    mkdirSync(path.dirname(out), { recursive: true });
    writeFileSync(out, zip);
    writeFileSync(`${out}.sha256`, `${sha256(zip)}  ${path.basename(out)}\n`);
    console.log(`Wrote ${out} (${zip.length} bytes, sha256 ${sha256(zip)})`);
  } else if (command === "verify") {
    const zipFile = a ? path.resolve(a) : DEFAULT_ZIP;
    const dir = b ? path.resolve(b) : DEFAULT_PACK;
    const { problems, files, items } = verifyZip(zipFile, dir);
    for (const p of problems) console.error(p);
    const fresh = buildZip(dir);
    if (!fresh.equals(readFileSync(zipFile))) {
      console.error(
        "The archive is not the deterministic build of the current pack. Run: npm run knowledge:zip",
      );
      process.exit(1);
    }
    console.log(
      `${files} files, ${items} items, ${problems.length} problem(s).`,
    );
    process.exit(problems.length ? 1 : 0);
  } else {
    console.error("Usage: node scripts/knowledge-pack.mjs validate|zip|verify");
    process.exit(2);
  }
}
