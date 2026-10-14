import { beforeEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { getTracePage, parseTracePage } from "./trace";
import { traceEntry, tracePage } from "../../../tests/frontend/trace-fixtures";
import {
  readySnapshot,
  tankSnapshot,
  runtimeFault,
} from "../../../tests/frontend/runtime-fixtures";
import { CONTRACT_VERSION } from "./generated";
vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
  invoke: vi.fn(),
}));
beforeEach(() => vi.resetAllMocks());
it("accepts exact typed observations and retains historical Tank, TON and provenance", () => {
  const s = tankSnapshot();
  s.state = "FAULTED";
  s.fault = { ...runtimeFault, relatedVariable: null };
  s.functionBlocks = [
    {
      kind: "TON",
      instanceId: "1",
      name: "Delay",
      q: true,
      etMicros: "300000",
    },
  ];
  const p = tracePage([traceEntry("2", "ScanFaulted", s)]);
  expect(parseTracePage(p)).toEqual(p);
});
it("requires closed event kinds and exact fields at every boundary", () => {
  for (const edit of [
    (p: ReturnType<typeof tracePage>) => ({ ...p, extra: 1 }),
    (p: ReturnType<typeof tracePage>) => ({
      ...p,
      entries: [{ ...p.entries[0], kind: "Replay" }],
    }),
    (p: ReturnType<typeof tracePage>) => ({
      ...p,
      entries: [{ ...p.entries[0], extra: 1 }],
    }),
    (p: ReturnType<typeof tracePage>) => ({
      ...p,
      contractVersion: CONTRACT_VERSION - 1,
    }),
  ])
    expect(() => parseTracePage(edit(tracePage()))).toThrow();
});
it("rejects unsafe numeric, zero, noncanonical and overflowing sequence transport", () => {
  for (const sequence of [
    0,
    "0",
    "01",
    "-1",
    "18446744073709551616",
    Number("9007199254740993"),
  ])
    expect(() =>
      parseTracePage({
        ...tracePage(),
        entries: [{ ...traceEntry(), sequence }],
      }),
    ).toThrow();
});
it("requires bounded metadata, monotonic contiguous entries and exact history loss", () => {
  const base = tracePage([
    traceEntry("2", "ScanCompleted"),
    traceEntry("3", "ScanCompleted"),
  ]);
  for (const mutation of [
    { capacity: 0 },
    { capacity: 4097 },
    { droppedCount: "1" },
    { oldestSequence: "4" },
    { latestSequence: null },
    { saturated: true },
    { gap: "true" },
    { entries: [base.entries[1], base.entries[0]] },
    { entries: [base.entries[0], base.entries[0]] },
  ])
    expect(() => parseTracePage({ ...base, ...mutation })).toThrow();
  const retained = {
    ...tracePage([traceEntry("5", "ScanCompleted")]),
    capacity: 3,
    oldestSequence: "5",
    latestSequence: "7",
    droppedCount: "4",
    gap: true,
  };
  expect(parseTracePage(retained)).toEqual(retained);
});
it("rejects malformed values, state, ownership and false student fault classification", () => {
  for (const entry of [
    { ...traceEntry("2", "ScanFaulted"), fault: null },
    { ...traceEntry(), runtimeState: "FAULTED" },
    {
      ...traceEntry(),
      variables: [
        {
          ...readySnapshot().variables[0],
          value: { type: "INT", value: 32768 },
        },
      ],
    },
    { ...traceEntry(), process: tankSnapshot().process },
    {
      ...traceEntry(),
      fault: { ...runtimeFault, category: "internalApplicationError" },
    },
  ])
    expect(() => parseTracePage(tracePage([entry as never]))).toThrow();
});
it("validates staged input semantic identity and rejects phantom staging on other kinds", () => {
  const e = {
    ...traceEntry("2", "InputStaged"),
    stagedInput: { variableId: "1", name: "Start" },
  };
  expect(parseTracePage(tracePage([e])).entries[0]).toEqual(e);
  for (const stagedInput of [
    null,
    { variableId: "2", name: "Motor" },
    { variableId: "1", name: "wrong" },
    { variableId: "1", name: "Start", extra: 1 },
  ])
    expect(() =>
      parseTracePage(tracePage([{ ...e, stagedInput } as never])),
    ).toThrow();
  expect(() =>
    parseTracePage(tracePage([{ ...e, kind: "ScanCompleted" }])),
  ).toThrow();
});
it("uses exact decimal strings above JS safe range without changing values", () => {
  const large = "9007199254740993";
  const s = readySnapshot();
  s.completedScans = large;
  s.simulationTimeMicros = "18446744073709551615";
  const e = traceEntry(large, "ScanCompleted", s);
  const p = {
    ...tracePage([e]),
    oldestSequence: large,
    droppedCount: "9007199254740992",
  };
  expect(parseTracePage(p)).toEqual(p);
});
it("queries targeted typed IPC incrementally with limit and exact cursor", async () => {
  const p = tracePage([traceEntry("2", "ScanCompleted")]);
  vi.mocked(invoke).mockResolvedValue(p);
  expect(await getTracePage("1", "1", 1)).toEqual(p);
  expect(invoke).toHaveBeenCalledWith("get_trace_page", {
    request: {
      contractVersion: CONTRACT_VERSION,
      sessionId: "1",
      afterSequence: "1",
      limit: 1,
    },
  });
});
it("rejects bad request IDs and limits before IPC", async () => {
  for (const limit of [0, 257, 1.5])
    await expect(getTracePage("1", null, limit)).rejects.toThrow();
  await expect(getTracePage("0")).rejects.toThrow();
  await expect(getTracePage("1", "01")).rejects.toThrow();
  expect(invoke).not.toHaveBeenCalled();
});
it("rejects another session or missing/incorrect query pages", async () => {
  for (const p of [
    tracePage(undefined, "2"),
    tracePage([]),
    { ...tracePage(), gap: true },
    tracePage([traceEntry("3", "ScanCompleted")]),
  ]) {
    vi.mocked(invoke).mockResolvedValue(p);
    await expect(getTracePage("1")).rejects.toThrow();
  }
});
