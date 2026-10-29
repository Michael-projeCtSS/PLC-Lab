import { expect, it } from "vitest";
import type { ModuleTypeDto, StationDto } from "@plc-lab/ipc";
import {
  addModule,
  freeSlots,
  moveModule,
  newModuleId,
  removeModule,
} from "./stationEdit";

const station: StationDto = {
  name: "PLC_1",
  cpu: "cpu.modular",
  modules: [
    {
      id: "di-r0s1",
      position: { rack: 0, slot: 1 },
      module: "di16.24vdc.sink",
      startAddress: 0,
      channels: [],
    },
    {
      id: "ai-r0s3",
      position: { rack: 0, slot: 3 },
      module: "ai4.ui",
      startAddress: 64,
      channels: [],
    },
  ],
  ioMap: [
    { channel: { rack: 0, slot: 1, channel: 2 }, tag: "PartAtEnd" },
    { channel: { rack: 0, slot: 3, channel: 0 }, tag: "LevelRaw" },
  ],
  network: { interface: null, modbusTcp: [] },
};
const di16: ModuleTypeDto = {
  id: "di16.24vdc.sink",
  name: "DI 16",
  product: null,
  channels: 16,
  class: "DI",
};
const ai4: ModuleTypeDto = {
  id: "ai4.ui",
  name: "AI 4",
  product: null,
  channels: 4,
  class: "AI",
};

it("adds a module with a unique stable id and the next free address", () => {
  const catalog = [di16, ai4];
  const next = addModule(station, di16, 4, catalog);
  const added = next.modules.find((m) => m.position.slot === 4)!;
  expect(added.id).toBe("di-r0s4");
  expect(added.startAddress).toBe(2);
  expect(
    addModule(station, ai4, 5, catalog).modules.find(
      (m) => m.position.slot === 5,
    )!.startAddress,
  ).toBe(72);
  // An id that is taken (after a move) gets a suffix instead of a duplicate.
  const moved = moveModule(station, "di-r0s1", 4);
  expect(newModuleId(moved, di16, 0, 1)).toBe("di-r0s1-2");
  expect(station.modules).toHaveLength(2);
});

it("moves a module with its mappings and keeps its id", () => {
  const next = moveModule(station, "ai-r0s3", 6);
  expect(next.modules.find((m) => m.id === "ai-r0s3")!.position.slot).toBe(6);
  expect(next.ioMap.find((m) => m.tag === "LevelRaw")!.channel.slot).toBe(6);
  expect(next.ioMap.find((m) => m.tag === "PartAtEnd")!.channel.slot).toBe(1);
  expect(moveModule(station, "ai-r0s3", 1)).toEqual(station);
  expect(freeSlots(next)).not.toContain(6);
});

it("removes a module and the mappings of its channels", () => {
  const next = removeModule(station, "di-r0s1");
  expect(next.modules.map((m) => m.id)).toEqual(["ai-r0s3"]);
  expect(next.ioMap.map((m) => m.tag)).toEqual(["LevelRaw"]);
});

const di8: ModuleTypeDto = {
  ...di16,
  id: "di8.24vdc.sink",
  name: "DI 8",
  channels: 8,
};
const do16: ModuleTypeDto = {
  ...di16,
  id: "do16.24vdc",
  name: "DO 16",
  class: "DO",
};
const ao2: ModuleTypeDto = {
  ...ai4,
  id: "ao2.ui",
  name: "AO 2",
  channels: 2,
  class: "AO",
};
const catalog = [di16, di8, do16, ai4, ao2];
const empty: StationDto = { ...station, modules: [], ioMap: [] };

/** Add modules in order to consecutive slots; returns `[id, startAddress]` per module. */
function build(types: ModuleTypeDto[], from: StationDto = empty) {
  let s = from;
  for (const t of types) s = addModule(s, t, freeSlots(s)[0]!, catalog);
  return s;
}
const addresses = (s: StationDto) =>
  s.modules.map((m) => [m.module.split(".")[0], m.startAddress]);

it("allocates digital and analog modules of each direction without overlap", () => {
  // Inputs (DI, AI) share the input image, outputs (DO, AO) the output image; digital
  // modules start at byte 0 and analog modules at byte 64.
  expect(addresses(build([di16, di16, di8]))).toEqual([
    ["di16", 0],
    ["di16", 2],
    ["di8", 4],
  ]);
  expect(addresses(build([do16, do16]))).toEqual([
    ["do16", 0],
    ["do16", 2],
  ]);
  expect(addresses(build([ai4, ai4, ai4]))).toEqual([
    ["ai4", 64],
    ["ai4", 72],
    ["ai4", 80],
  ]);
  expect(addresses(build([ao2, ao2]))).toEqual([
    ["ao2", 64],
    ["ao2", 68],
  ]);
});

it("interleaved additions keep the digital and analog ranges apart", () => {
  // The analog input at 64 never pushes a digital input to 72 (regression).
  expect(addresses(build([di16, ai4, di16, ao2, do16, ai4, di8]))).toEqual([
    ["di16", 0],
    ["ai4", 64],
    ["di16", 2],
    ["ao2", 64],
    ["do16", 0],
    ["ai4", 72],
    ["di8", 4],
  ]);
});

it("moving keeps addresses, removing frees them for the next module", () => {
  const s = build([di16, di16, ai4]);
  const moved = moveModule(s, "di-r0s1", 9);
  expect(moved.modules.find((m) => m.id === "di-r0s1")!.startAddress).toBe(0);
  // The moved module still occupies bytes 0..1: the new module in slot 1 goes after.
  const slot = (x: StationDto, n: number) =>
    x.modules.find((m) => m.position.slot === n)!.startAddress;
  expect(slot(build([di16], moved), 1)).toBe(4);
  // Removing the first module frees bytes 0..1 for the next one of that size.
  const removed = removeModule(s, "di-r0s1");
  const again = build([di16], removed);
  expect(again.modules.find((m) => m.position.slot === 1)!.startAddress).toBe(
    0,
  );
  // A gap too small for the module is skipped (first fit).
  const gap = removeModule(build([di8, di8, di8]), "di-r0s2");
  expect(slot(build([di16], gap), 2)).toBe(3);
});

it("allocation is deterministic", () => {
  const sequence = [di16, ai4, do16, ao2, di8, ai4];
  expect(build(sequence)).toEqual(build(sequence));
});
