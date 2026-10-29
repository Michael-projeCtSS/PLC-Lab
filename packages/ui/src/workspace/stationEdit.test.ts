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
