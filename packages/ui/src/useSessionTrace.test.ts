import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { getTracePage } from "@plc-lab/ipc";
import type { TracePageDto } from "@plc-lab/ipc";
import { useSessionTrace } from "./useSessionTrace";
import { mergeTracePage } from "./trace-history";
import { traceEntry, tracePage } from "../../../tests/frontend/trace-fixtures";
vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  getTracePage: vi.fn(),
}));
beforeEach(() => vi.resetAllMocks());
const loaded = { id: "1" as string | null, key: "1", current: true };
const mount = (initial = loaded) =>
  renderHook((p) => useSessionTrace(p.id, p.key, p.current), {
    initialProps: initial,
  });
it("does not query before Load and requests initial SessionLoaded history", async () => {
  vi.mocked(getTracePage).mockResolvedValue(tracePage());
  const h = mount({ id: null, key: "0", current: false });
  expect(getTracePage).not.toHaveBeenCalled();
  h.rerender(loaded);
  await waitFor(() => expect(h.result.current.pending).toBe(false));
  expect(h.result.current.page?.entries[0]?.kind).toBe("SessionLoaded");
  expect(getTracePage).toHaveBeenCalledWith("1", null, 256);
});
it("refreshes from the last retained sequence after a manual observation", async () => {
  vi.mocked(getTracePage)
    .mockResolvedValueOnce(tracePage())
    .mockResolvedValueOnce(tracePage([traceEntry("2", "ScanCompleted")]));
  const h = mount();
  await waitFor(() => expect(h.result.current.pending).toBe(false));
  h.rerender({ ...loaded, key: "2" });
  await waitFor(() => expect(h.result.current.page?.entries).toHaveLength(2));
  expect(getTracePage).toHaveBeenLastCalledWith("1", "1", 256);
});
it("merges repeated pages without duplicate events and removes evicted entries", () => {
  const a = tracePage([traceEntry(), traceEntry("2", "ScanCompleted")]);
  expect(
    mergeTracePage(a, tracePage([traceEntry("2", "ScanCompleted")])).entries,
  ).toHaveLength(2);
  const p = {
    ...tracePage([
      traceEntry("5", "ScanCompleted"),
      traceEntry("6", "ScanCompleted"),
    ]),
    capacity: 2,
    oldestSequence: "5",
    latestSequence: "6",
    droppedCount: "4",
    gap: true,
  };
  expect(mergeTracePage(a, p).entries.map((e) => e.sequence)).toEqual([
    "5",
    "6",
  ]);
});
it("new Load replaces history and starts fetching without the old cursor", async () => {
  vi.mocked(getTracePage)
    .mockResolvedValueOnce(tracePage())
    .mockResolvedValueOnce(tracePage(undefined, "2"));
  const h = mount();
  await waitFor(() => expect(h.result.current.pending).toBe(false));
  h.rerender({ ...loaded, id: "2", key: "2" });
  await waitFor(() => expect(h.result.current.page?.sessionId).toBe("2"));
  expect(h.result.current.page?.entries).toHaveLength(1);
  expect(getTracePage).toHaveBeenLastCalledWith("2", null, 256);
});
it("source edit preserves accepted old history and discards a pending page", async () => {
  let complete!: (p: TracePageDto) => void;
  vi.mocked(getTracePage)
    .mockResolvedValueOnce(tracePage())
    .mockReturnValueOnce(
      new Promise((r) => {
        complete = r;
      }),
    );
  const h = mount();
  await waitFor(() => expect(h.result.current.pending).toBe(false));
  h.rerender({ ...loaded, key: "2" });
  await waitFor(() => expect(getTracePage).toHaveBeenCalledTimes(2));
  h.rerender({ ...loaded, key: "2", current: false });
  await act(async () =>
    complete(tracePage([traceEntry("2", "ScanCompleted")])),
  );
  expect(h.result.current.page?.entries).toHaveLength(1);
  expect(h.result.current.pending).toBe(false);
});
it("unload clears history and a late old-session response cannot repopulate it", async () => {
  let complete!: (p: TracePageDto) => void;
  vi.mocked(getTracePage).mockReturnValue(
    new Promise((r) => {
      complete = r;
    }),
  );
  const h = mount();
  h.rerender({ id: null, key: "2", current: false });
  await act(async () => complete(tracePage()));
  expect(h.result.current.page).toBeNull();
});
it("replacement discards the pending prior session page", async () => {
  let old!: (p: TracePageDto) => void;
  vi.mocked(getTracePage)
    .mockReturnValueOnce(
      new Promise((r) => {
        old = r;
      }),
    )
    .mockResolvedValueOnce(tracePage(undefined, "2"));
  const h = mount();
  h.rerender({ ...loaded, id: "2", key: "2" });
  await waitFor(() => expect(h.result.current.page?.sessionId).toBe("2"));
  await act(async () => old(tracePage()));
  expect(h.result.current.page?.sessionId).toBe("2");
});
it("reports query failures without mutating live state or fabricating history", async () => {
  vi.mocked(getTracePage).mockRejectedValue(new Error("backend unavailable"));
  const h = mount();
  await waitFor(() =>
    expect(h.result.current.error).toBe("backend unavailable"),
  );
  expect(h.result.current.page).toBeNull();
});
it("catches up bounded pages from each returned cursor without polling", async () => {
  vi.mocked(getTracePage)
    .mockResolvedValueOnce({ ...tracePage(), latestSequence: "3" })
    .mockResolvedValueOnce(
      tracePage([
        traceEntry("2", "ScanCompleted"),
        traceEntry("3", "ScanCompleted"),
      ]),
    );
  const h = mount();
  await waitFor(() => expect(h.result.current.page?.entries).toHaveLength(3));
  expect(getTracePage).toHaveBeenNthCalledWith(2, "1", "1", 256);
  expect(getTracePage).toHaveBeenCalledTimes(2);
  expect(h.result.current.pending).toBe(false);
});
