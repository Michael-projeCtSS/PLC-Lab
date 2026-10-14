import { act, renderHook, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { compileSt } from "@plc-lab/ipc";
import type { CompileResponse } from "@plc-lab/ipc";
import { useStCompilation } from "./useStCompilation";
vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  compileSt: vi.fn(),
}));
const success: CompileResponse = {
  success: true,
  programName: "Main",
  symbols: [],
  diagnostics: [],
};
function deferred() {
  let resolve!: (value: CompileResponse) => void;
  const promise = new Promise<CompileResponse>((r) => {
    resolve = r;
  });
  return { resolve, promise };
}
it("older requests never overwrite newer source/results", async () => {
  const old = deferred(),
    recent = deferred();
  vi.mocked(compileSt)
    .mockReturnValueOnce(old.promise)
    .mockReturnValueOnce(recent.promise);
  const { result } = renderHook(useStCompilation);
  act(() => {
    void result.current.compile();
  });
  expect(result.current.state.kind).toBe("compiling");
  act(() => result.current.edit("PROGRAM New END_PROGRAM"));
  expect(result.current.state.kind).toBe("not-compiled");
  act(() => {
    void result.current.compile();
  });
  await act(async () => recent.resolve({ ...success, programName: "New" }));
  await act(async () =>
    old.resolve({
      success: false,
      diagnostics: [
        {
          code: "PLC-ST-002",
          severity: "error",
          message: "Old error",
          span: null,
        },
      ],
    }),
  );
  expect(result.current.state).toEqual({
    kind: "result",
    result: { ...success, programName: "New" },
  });
});
it("editing clears diagnostics and declaration data immediately", async () => {
  vi.mocked(compileSt).mockResolvedValue(success);
  const { result } = renderHook(useStCompilation);
  await act(async () => result.current.compile());
  expect(result.current.state.kind).toBe("result");
  act(() => result.current.edit(""));
  expect(result.current.state.kind).toBe("not-compiled");
  expect(result.current.source).toBe("");
});
it("unmount invalidates pending requests", async () => {
  const pending = deferred();
  vi.mocked(compileSt).mockReturnValue(pending.promise);
  const view = renderHook(useStCompilation);
  act(() => {
    void view.result.current.compile();
  });
  view.unmount();
  await act(async () => pending.resolve(success));
  expect(view.result.current.state.kind).toBe("compiling");
});
it("transport errors remain separate from semantic diagnostics", async () => {
  vi.mocked(compileSt).mockRejectedValue(new Error("Disconnected"));
  const { result } = renderHook(useStCompilation);
  act(() => {
    void result.current.compile();
  });
  await waitFor(() =>
    expect(result.current.state.kind).toBe("transport-error"),
  );
});
