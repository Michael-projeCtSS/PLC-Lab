import { describe, expect, it } from "vitest";
import type { StandardBlockDto } from "@plc-lab/ipc";
import type { LadDocument } from "../ladModel";
import {
  instructions,
  literalType,
  matchInstructions,
  score,
  suggest,
} from "./complete";

const d: LadDocument = {
  format: "plclab-lad",
  version: 2,
  name: "Main",
  variables: [
    { name: "MotorStart", type: "BOOL", direction: "input", comment: "S1" },
    { name: "MotorRunning", type: "BOOL", direction: "input" },
    { name: "MotorFault", type: "BOOL", direction: "local" },
    { name: "Motor", type: "BOOL", direction: "output" },
    { name: "Level", type: "INT", direction: "input" },
    { name: "Ratio", type: "REAL", direction: "local" },
    { name: "Waited", type: "TIME", direction: "local" },
    { name: "Delay", type: "TIME", direction: "input" },
  ],
  rungs: [
    {
      id: 1,
      network: {
        type: "series",
        id: 2,
        items: [
          { type: "contact", id: 3, contact: "normallyOpen", variable: "" },
          {
            type: "compare",
            id: 4,
            op: "gt",
            left: { variable: "" },
            right: { variable: "Level" },
          },
          { type: "block", id: 5, block: "TON", instance: "T1" },
          { type: "block", id: 8, block: "CTU", instance: "C1" },
        ],
      },
      coils: [{ id: 6, variable: "" }],
    },
  ],
};
const names = (s: ReturnType<typeof suggest>) =>
  s.map((x) => ("name" in x ? `${x.kind}:${x.name}` : `${x.kind}:${x.text}`));

describe("variable completion", () => {
  it("ranks exact, prefix, word and substring matches", () => {
    expect(score("Motor", "motor")).toBe(100);
    expect(score("MotorStart", "mot")).toBeGreaterThan(
      score("RunMotor", "mot")!,
    );
    expect(score("MotorRunning", "mr")).toBe(60);
    expect(score("MotorRunning", "run")).toBe(55);
    expect(score("Level", "xyz")).toBeNull();
  });

  it("a contact gets BOOL variables only, best match first", () => {
    expect(names(suggest(d, 3, "variable", "Mot"))).toEqual([
      "variable:Motor",
      "variable:MotorFault",
      "variable:MotorStart",
      "variable:MotorRunning",
      // An undeclared name is always offered last, explicitly, for creation.
      "create:Mot",
    ]);
    expect(names(suggest(d, 3, "variable", "lev"))).toEqual(["create:lev"]);
  });

  it("a coil never offers inputs", () => {
    expect(names(suggest(d, 6, "variable", "mot"))).toEqual([
      "variable:Motor",
      "variable:MotorFault",
      "create:mot",
    ]);
  });

  it("a comparator operand follows the other operand's type", () => {
    // Right is Level (INT): INT variables and an INT literal.
    expect(names(suggest(d, 4, "left", "le"))).toEqual([
      "variable:Level",
      "create:le",
    ]);
    expect(names(suggest(d, 4, "left", "10"))).toEqual(["literal:10"]);
  });

  it("a timer preset offers TIME variables and IEC time literals", () => {
    const s = names(suggest(d, 5, "preset", ""));
    expect(s.slice(0, 2)).toEqual(["variable:Delay", "variable:Waited"]);
    expect(s).toContain("literal:T#5s");
    expect(names(suggest(d, 5, "preset", "5"))).toEqual(
      ["literal:T#5s", "literal:T#5ms", "create:5"].filter(
        (x) => x !== "create:5",
      ),
    );
    expect(names(suggest(d, 5, "preset", "t#2"))).toContain("literal:T#2s");
    // ET may only write a TIME variable that is not an input.
    expect(names(suggest(d, 5, "output", ""))).toEqual(["variable:Waited"]);
    // A counter preset is an INT.
    expect(names(suggest(d, 8, "preset", "4"))).toEqual(["literal:4"]);
  });

  it("an unknown name is offered for creation, never bound silently", () => {
    const s = suggest(d, 3, "variable", "Pump1");
    expect(s).toEqual([
      { kind: "create", name: "Pump1", type: "BOOL", direction: "local" },
    ]);
    expect(suggest(d, 6, "variable", "Lamp")).toEqual([
      { kind: "create", name: "Lamp", type: "BOOL", direction: "output" },
    ]);
    // Taken names (variables, instances) and literals are never offered for creation.
    expect(names(suggest(d, 3, "variable", "T1"))).toEqual([]);
    expect(names(suggest(d, 3, "variable", "motor"))).toEqual([
      "variable:Motor",
      "variable:MotorFault",
      "variable:MotorStart",
      "variable:MotorRunning",
    ]);
  });

  it("instance names are proposed unique; taken names are not proposed", () => {
    expect(names(suggest(d, 5, "instance", ""))).toEqual([
      "instance:T2",
      "instance:T1",
    ]);
    expect(names(suggest(d, 5, "instance", "RunDelay"))).toEqual([
      "instance:RunDelay",
      "instance:T2",
      "instance:T1",
    ]);
    // C1 belongs to another box, Level is a variable.
    expect(names(suggest(d, 5, "instance", "C1"))).toEqual([
      "instance:T2",
      "instance:T1",
    ]);
    expect(names(suggest(d, 5, "instance", "Level"))).toEqual([
      "instance:T2",
      "instance:T1",
    ]);
  });

  it("knows IEC literal types like the compiler", () => {
    expect(literalType("T#1s500ms")).toBe("TIME");
    expect(literalType("true")).toBe("BOOL");
    expect(literalType("-12")).toBe("INT");
    expect(literalType("2.5")).toBe("REAL");
    expect(literalType("5s")).toBeNull();
  });
});

describe("instruction completion", () => {
  const standard: StandardBlockDto[] = [
    {
      name: "TON",
      parameters: [
        { name: "IN", dataType: "BOOL", direction: "Input", required: true },
        { name: "PT", dataType: "TIME", direction: "Input", required: true },
        { name: "Q", dataType: "BOOL", direction: "Output", required: false },
        { name: "ET", dataType: "TIME", direction: "Output", required: false },
      ],
    },
    { name: "PID", parameters: [] },
  ];
  const list = instructions(standard);
  const text = (i: { name: string }) => i.name;

  it("offers the LAD instructions with the runtime's pins, and no PID", () => {
    expect(list.map((i) => i.name)).toEqual([
      "NO",
      "NC",
      "P",
      "N",
      "TON",
      "TOF",
      "TP",
      "CTU",
      "CTD",
      "EQ",
      "NE",
      "GT",
      "GE",
      "LT",
      "LE",
      "COIL",
      "S",
      "R",
    ]);
    expect(list.find((i) => i.name === "TON")?.pins).toEqual([
      { name: "IN", type: "BOOL", direction: "in" },
      { name: "PT", type: "TIME", direction: "in" },
      { name: "Q", type: "BOOL", direction: "out" },
      { name: "ET", type: "TIME", direction: "out" },
    ]);
  });

  it("matches names, aliases and descriptions", () => {
    expect(matchInstructions(list, "TON", text)[0]?.name).toBe("TON");
    expect(
      matchInstructions(list, "t", text)
        .slice(0, 3)
        .map((i) => i.name)
        .sort(),
    ).toEqual(["TOF", "TON", "TP"]);
    expect(matchInstructions(list, "ge", text)[0]?.name).toBe("GE");
    expect(matchInstructions(list, ">=", text)[0]?.name).toBe("GE");
    expect(matchInstructions(list, "set", text)[0]?.name).toBe("S");
    expect(matchInstructions(list, "counter", text).map((i) => i.name)).toEqual(
      ["CTU", "CTD"],
    );
    expect(matchInstructions(list, "pid", text)).toEqual([]);
  });
});
