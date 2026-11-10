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
  if (node.type === "contact") return [node.contact, node.variable];
  if (node.type === "series") return ["series", ...node.items.map(shape)];
  return ["parallel", ...node.branches.map(shape)];
}
const firstContact = (d: LadDocument, rung = 0): number => {
  const n = d.rungs[rung]?.network;
  if (n?.type !== "series" || n.items[0]?.type !== "contact")
    throw new Error("no first contact");
  return n.items[0].id;
};
const lastId = (d: LadDocument) => Math.max(...elementIds(d));

describe("ladder document model", () => {
  it("a new program declares the scene's BOOL signals and has no rungs", () => {
    const d = newLadDocument("Main", motorScene);
    expect(d.variables.map((v) => [v.name, v.direction])).toEqual([
      ["StartPB", "input"],
      ["StopPB", "input"],
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
});
