import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  addCoil,
  addRung,
  addVariable,
  elementIds,
  insertParallel,
  insertSeries,
  newLadDocument,
  parseLad,
  removeCoil,
  removeElement,
  serializeLad,
  setCoilVariable,
  setContactVariable,
  toggleContact,
  duplicateRung,
  findNode,
  insertParallel as parallelTo,
  isSynthetic,
  moveElement,
  moveRung,
  nextInstance,
  pasteRung,
  readingOrder,
  setBlockPin,
  setCoil,
  type LadDocument,
  type LadNode,
} from "./ladModel";

const motorScene = [
  {
    tag: "StartPB",
    dataType: "BOOL",
    direction: "input",
    description: "Start",
  },
  { tag: "StopPB", dataType: "BOOL", direction: "input", description: "Stop" },
  { tag: "Level", dataType: "INT", direction: "input", description: "Raw" },
  {
    tag: "MotorContactor",
    dataType: "BOOL",
    direction: "output",
    description: "K1",
  },
];

/** The network without ids: what the ladder means. */
function shape(node: LadNode): unknown {
  switch (node.type) {
    case "contact":
      return [node.contact, node.variable];
    case "compare":
      return ["compare", node.op];
    case "block":
      return [node.block, node.instance];
    case "series":
      return ["series", ...node.items.map(shape)];
    case "parallel":
      return ["parallel", ...node.branches.map(shape)];
  }
}
const firstContact = (d: LadDocument, rung = 0): number => {
  const n = d.rungs[rung]?.network;
  if (n?.type !== "series" || n.items[0]?.type !== "contact")
    throw new Error("no first contact");
  return n.items[0].id;
};
const lastId = (d: LadDocument) => Math.max(...elementIds(d));

describe("ladder document model", () => {
  it("a new program declares the scene's signals and has no rungs", () => {
    const d = newLadDocument("Main", motorScene);
    expect(d.variables.map((v) => [v.name, v.direction])).toEqual([
      ["StartPB", "input"],
      ["StopPB", "input"],
      ["Level", "input"],
      ["MotorContactor", "output"],
    ]);
    expect(d.rungs).toEqual([]);
    expect(parseLad(serializeLad(d))).toEqual({ document: d });
  });

  it("builds the self-holding motor rung of the graded LAD solution", () => {
    let d = addRung(newLadDocument("Main", motorScene));
    const start = firstContact(d);
    d = setContactVariable(d, start, "StartPB");
    d = insertParallel(d, start, "normallyOpen");
    d = setContactVariable(d, lastId(d), "MotorContactor");
    // Select the parallel block and continue in series after it.
    const block = d.rungs[0]?.network;
    if (block?.type !== "series" || block.items[0]?.type !== "parallel")
      throw new Error("expected a parallel block first");
    d = insertSeries(d, block.items[0].id, "normallyOpen");
    d = setContactVariable(d, lastId(d), "StopPB");
    d = insertSeries(d, lastId(d), "normallyOpen");
    d = setContactVariable(d, lastId(d), "OverloadOK");
    d = setCoilVariable(d, d.rungs[0]?.coils[0]?.id ?? 0, "MotorContactor");
    const solution = JSON.parse(
      readFileSync(
        join(
          process.cwd(),
          "tests/conformance/solutions/ex.motor-start-stop.lad.json",
        ),
        "utf8",
      ),
    ) as LadDocument;
    const rung = solution.rungs[0];
    expect(rung && shape(d.rungs[0]!.network)).toEqual(
      rung && shape(rung.network),
    );
    expect(d.rungs[0]?.coils.map((c) => c.variable)).toEqual([
      "MotorContactor",
    ]);
    // Ids stay unique and positive.
    const ids = elementIds(d);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids.every((id) => id > 0)).toBe(true);
  });

  it("parallel insertion adds a branch to an existing parallel block", () => {
    let d = addRung(newLadDocument("Main", motorScene));
    const a = firstContact(d);
    d = insertParallel(d, a, "normallyOpen");
    const b = lastId(d);
    d = insertParallel(d, b, "normallyClosed");
    expect(shape(d.rungs[0]!.network)).toEqual([
      "series",
      [
        "parallel",
        ["normallyOpen", ""],
        ["normallyOpen", ""],
        ["normallyClosed", ""],
      ],
    ]);
  });

  it("deleting keeps the network valid: one-branch blocks collapse, the root may empty", () => {
    let d = addRung(newLadDocument("Main", motorScene));
    const a = firstContact(d);
    d = setContactVariable(d, a, "StartPB");
    d = insertParallel(d, a, "normallyOpen");
    const b = lastId(d);
    d = removeElement(d, b);
    expect(shape(d.rungs[0]!.network)).toEqual([
      "series",
      ["normallyOpen", "StartPB"],
    ]);
    d = removeElement(d, a);
    expect(shape(d.rungs[0]!.network)).toEqual(["series"]);
    // An empty rung takes a contact again.
    const root = d.rungs[0]!.network.id;
    d = insertParallel(d, root, "normallyClosed");
    expect(shape(d.rungs[0]!.network)).toEqual([
      "series",
      ["normallyClosed", ""],
    ]);
  });

  it("toggles NO/NC, adds and removes coils, and declares variables once", () => {
    let d = addRung(newLadDocument("Main", motorScene));
    const a = firstContact(d);
    d = toggleContact(d, a);
    expect(shape(d.rungs[0]!.network)).toEqual([
      "series",
      ["normallyClosed", ""],
    ]);
    d = addCoil(d, d.rungs[0]!.id);
    expect(d.rungs[0]!.coils).toHaveLength(2);
    d = removeCoil(d, d.rungs[0]!.coils[1]!.id);
    expect(d.rungs[0]!.coils).toHaveLength(1);
    const added = addVariable(d, "Memory", "local");
    expect(added?.variables.at(-1)).toEqual({
      name: "Memory",
      type: "BOOL",
      direction: "local",
    });
    expect(addVariable(d, "startpb", "local")).toBeNull();
    expect(addVariable(d, "2nd", "local")).toBeNull();
  });

  it("rejects text that is not a ladder document", () => {
    expect(parseLad("PROGRAM Main END_PROGRAM")).toHaveProperty("error");
    expect(parseLad('{"format":"other"}')).toHaveProperty("error");
    expect(
      parseLad(
        JSON.stringify({
          format: "plclab-lad",
          version: 1,
          name: "Main",
          variables: [],
          rungs: [{ id: 1, coils: [], network: { type: "x", id: 2 } }],
        }),
      ),
    ).toHaveProperty("error");
  });

  it("reads version 1 documents and writes version 2", () => {
    const v1 = JSON.stringify({
      format: "plclab-lad",
      version: 1,
      name: "Main",
      variables: [],
      rungs: [],
    });
    const parsed = parseLad(v1);
    expect(parsed).toHaveProperty("document");
    if (!("document" in parsed)) return;
    expect(JSON.parse(serializeLad(parsed.document)).version).toBe(2);
  });

  it("inserts boxes and comparators with fresh instance names and presets", () => {
    let d = addRung(newLadDocument("Main", motorScene));
    const start = firstContact(d);
    d = insertSeries(d, start, { type: "block", block: "TON" });
    const ton = findNode(d, lastId(d));
    expect(ton).toMatchObject({
      type: "block",
      block: "TON",
      instance: "T1",
      preset: { constant: "T#1s" },
    });
    d = insertSeries(d, lastId(d), { type: "block", block: "TOF" });
    expect(findNode(d, lastId(d))).toMatchObject({ instance: "T2" });
    d = insertSeries(d, lastId(d), { type: "block", block: "CTU" });
    expect(findNode(d, lastId(d))).toMatchObject({
      instance: "C1",
      preset: { constant: "3" },
    });
    d = insertSeries(d, start, { type: "compare", op: "ge" }, "before");
    expect(shape(d.rungs[0]!.network)).toEqual([
      "series",
      ["compare", "ge"],
      ["normallyOpen", ""],
      ["TON", "T1"],
      ["TOF", "T2"],
      ["CTU", "C1"],
    ]);
    // A variable takes no instance name, and no instance a variable name.
    expect(addVariable(d, "T1", "local")).toBeNull();
    expect(nextInstance(addVariable(d, "T3", "local")!, "TP")).toBe("T4");
  });

  it("sets and clears optional box pins and coil kinds", () => {
    let d = addRung(newLadDocument("Main", motorScene));
    d = insertSeries(d, firstContact(d), { type: "block", block: "CTU" });
    const box = lastId(d);
    d = setBlockPin(d, box, "reset", { variable: "StopPB" });
    d = setBlockPin(d, box, "output", "Level");
    expect(findNode(d, box)).toMatchObject({
      reset: { variable: "StopPB" },
      output: "Level",
    });
    d = setBlockPin(d, box, "reset", null);
    d = setBlockPin(d, box, "output", "");
    expect(findNode(d, box)).not.toHaveProperty("reset");
    expect(findNode(d, box)).not.toHaveProperty("output");
    const coil = d.rungs[0]!.coils[0]!.id;
    d = setCoil(d, coil, { kind: "set", variable: "MotorContactor" });
    expect(d.rungs[0]!.coils[0]).toEqual({
      id: coil,
      variable: "MotorContactor",
      kind: "set",
    });
    d = setCoil(d, coil, { kind: "output" });
    expect(d.rungs[0]!.coils[0]).toEqual({
      id: coil,
      variable: "MotorContactor",
    });
    d = addCoil(d, d.rungs[0]!.id, "reset");
    expect(d.rungs[0]!.coils[1]).toMatchObject({ kind: "reset" });
  });

  it("moves elements in their series and rungs in the program", () => {
    let d = addRung(newLadDocument("Main", motorScene));
    const a = firstContact(d);
    d = setContactVariable(d, a, "StartPB");
    d = insertSeries(d, a, "normallyClosed");
    d = setContactVariable(d, lastId(d), "StopPB");
    d = moveElement(d, a, 1);
    expect(shape(d.rungs[0]!.network)).toEqual([
      "series",
      ["normallyClosed", "StopPB"],
      ["normallyOpen", "StartPB"],
    ]);
    expect(moveElement(d, a, 1)).toEqual(d);
    d = addRung(d);
    const [first, second] = d.rungs.map((r) => r.id);
    d = moveRung(d, second!, -1);
    expect(d.rungs.map((r) => r.id)).toEqual([second, first]);
    expect(moveRung(d, second!, -1)).toEqual(d);
  });

  it("a copied rung gets fresh ids and instances and shares nothing", () => {
    let d = addRung(newLadDocument("Main", motorScene));
    d = parallelTo(d, firstContact(d), { type: "block", block: "TON" });
    d = duplicateRung(d, d.rungs[0]!.id);
    expect(d.rungs).toHaveLength(2);
    const ids = elementIds(d);
    expect(new Set(ids).size).toBe(ids.length);
    const instances = d.rungs.map(
      (r) => JSON.stringify(r.network).match(/"instance":"(\w+)"/)?.[1],
    );
    expect(instances).toEqual(["T1", "T2"]);
    d = pasteRung(d, d.rungs[0]!);
    expect(d.rungs.at(-1)?.network).toMatchObject({ type: "series" });
    expect(JSON.stringify(d.rungs.at(-1))).toContain('"instance":"T3"');
  });

  it("reads elements rung by rung, left to right, coils last", () => {
    let d = addRung(newLadDocument("Main", motorScene));
    const a = firstContact(d);
    d = parallelTo(d, a, "normallyOpen");
    const b = lastId(d);
    const network = d.rungs[0]!.network;
    const block = network.type === "series" ? network.items[0]!.id : 0;
    expect(readingOrder(d)).toEqual([[block, a, b, d.rungs[0]!.coils[0]!.id]]);
  });

  it("knows the compiler-owned symbols", () => {
    expect(isSynthetic("LAD#12")).toBe(true);
    expect(isSynthetic("edge#3")).toBe(true);
    expect(isSynthetic("LAD")).toBe(false);
    expect(isSynthetic("Motor")).toBe(false);
  });
});
