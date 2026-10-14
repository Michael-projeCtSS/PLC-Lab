import { tracePage } from "../../../tests/frontend/trace-fixtures";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import {
  compileSt,
  getCoreInfo,
  loadStProgram,
  setRuntimeInput,
  singleScan,
  resetRuntime,
  unloadProgram,
  getTracePage,
} from "@plc-lab/ipc";
import type { CompilerDiagnostic, RuntimeSnapshotDto } from "@plc-lab/ipc";
import { LabShell } from "./LabShell";
import { CONTRACT_VERSION } from "../../ipc/src/generated";
import {
  readySnapshot,
  runtimeFault,
} from "../../../tests/frontend/runtime-fixtures";
vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  compileSt: vi.fn(),
  getCoreInfo: vi.fn(),
  loadStProgram: vi.fn(),
  setRuntimeInput: vi.fn(),
  singleScan: vi.fn(),
  resetRuntime: vi.fn(),
  unloadProgram: vi.fn(),
  getTracePage: vi.fn(),
}));
vi.mock("./StructuredTextEditor", () => ({
  StructuredTextEditor: ({
    source,
    onChange,
    diagnostics,
    runtimeDiagnostics = [],
  }: {
    source: string;
    onChange: (v: string) => void;
    diagnostics: CompilerDiagnostic[];
    runtimeDiagnostics?: CompilerDiagnostic[];
  }) => (
    <textarea
      aria-label="ST test editor"
      data-testid="st-editor"
      data-compiler-markers={diagnostics.length}
      data-runtime-markers={runtimeDiagnostics.length}
      value={source}
      onChange={(e) => onChange(e.target.value)}
    />
  ),
}));
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(getTracePage).mockImplementation(async (sessionId) => ({
    ...tracePage(),
    sessionId,
  }));
  vi.mocked(getCoreInfo).mockResolvedValue({
    applicationName: "PLC Lab",
    appVersion: "test",
    coreVersion: "test",
    contractVersion: CONTRACT_VERSION,
    execution: "manual-single-scan",
    buildContext: "development",
  });
  vi.mocked(compileSt).mockResolvedValue({
    success: true,
    programName: "Main",
    symbols: [
      { name: "Start", typeName: "BOOL", role: "Input" },
      { name: "Motor", typeName: "BOOL", role: "Output" },
    ],
    diagnostics: [],
  });
  vi.mocked(loadStProgram).mockResolvedValue({
    kind: "loaded",
    snapshot: readySnapshot(),
  });
});
const click = async (name: string) => {
  fireEvent.click(screen.getByRole("button", { name }));
  await waitFor(() =>
    expect(screen.getByTestId("runtime-status")).not.toHaveTextContent(
      "request in progress",
    ),
  );
};
const mountLoaded = async (snapshot: RuntimeSnapshotDto = readySnapshot()) => {
  vi.mocked(loadStProgram).mockResolvedValue({ kind: "loaded", snapshot });
  render(<LabShell />);
  fireEvent.click(screen.getByRole("button", { name: "Compile" }));
  await screen.findByText("COMPILE SUCCESS");
  await click("Load Program");
};
it("compile alone remains declaration-only and does not invoke load", async () => {
  render(<LabShell />);
  expect(screen.getByRole("button", { name: "Load Program" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Compile" }));
  await screen.findByText("COMPILE SUCCESS");
  expect(loadStProgram).not.toHaveBeenCalled();
  expect(screen.queryByTestId("runtime-variables")).not.toBeInTheDocument();
  expect(screen.getAllByLabelText("Runtime value unavailable")).toHaveLength(2);
  expect(screen.getByRole("button", { name: "Single Scan" })).toBeDisabled();
});
it("explicit load displays backend defaults and only input editors", async () => {
  await mountLoaded();
  expect(screen.getByTestId("runtime-scan-count")).toHaveTextContent("0");
  expect(screen.getByTestId("runtime-time")).toHaveTextContent("0 ms");
  expect(screen.getByLabelText("Value Motor")).toHaveTextContent("FALSE");
  expect(screen.getByLabelText("Set Start")).toBeEnabled();
  expect(screen.queryByLabelText("Set Motor")).not.toBeInTheDocument();
  expect(screen.queryByLabelText("Set Internal")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Single Scan" })).toBeEnabled();
  expect(screen.getByRole("button", { name: "Run" })).toBeEnabled();
  for (const name of ["Pause", "Stop"])
    expect(screen.getByRole("button", { name })).toBeDisabled();
});
it("BOOL staging and committed values come from returned snapshots", async () => {
  await mountLoaded();
  const staged = readySnapshot();
  staged.variables[0]!.value = { type: "BOOL", value: true };
  vi.mocked(setRuntimeInput).mockResolvedValue(staged);
  fireEvent.click(screen.getByLabelText("Set Start"));
  await waitFor(() => expect(screen.getByLabelText("Set Start")).toBeChecked());
  expect(screen.getByLabelText("Value Motor")).toHaveTextContent("FALSE");
  const committed = structuredClone(staged);
  committed.variables[1]!.value = { type: "BOOL", value: true };
  committed.variables[2]!.value = { type: "BOOL", value: true };
  committed.completedScans = "17";
  committed.simulationTimeMicros = "123456";
  vi.mocked(singleScan).mockResolvedValue(committed);
  await click("Single Scan");
  expect(screen.getByLabelText("Value Motor")).toHaveTextContent("TRUE");
  expect(screen.getByLabelText("Value Internal")).toHaveTextContent("TRUE");
  expect(screen.getByTestId("runtime-scan-count")).toHaveTextContent("17");
  expect(screen.getByTestId("runtime-time")).toHaveTextContent("123456 µs");
});
it("source editing marks prior values stale and prevents execution/input commits", async () => {
  await mountLoaded();
  fireEvent.change(screen.getByLabelText("ST test editor"), {
    target: { value: "PROGRAM Changed END_PROGRAM" },
  });
  expect(screen.getByTestId("runtime-status")).toHaveTextContent("STALE");
  expect(screen.getByRole("button", { name: "Single Scan" })).toBeDisabled();
  expect(screen.getByLabelText("Set Start")).toBeDisabled();
  expect(screen.getByRole("button", { name: "Load Program" })).toBeDisabled();
  expect(screen.getByTestId("runtime-variables")).toBeInTheDocument();
});
it("INT REAL and TIME display exactly and numeric drafts require Apply", async () => {
  const s = readySnapshot();
  s.variables = [
    {
      variableId: "1",
      name: "Count",
      typeName: "INT",
      role: "Input",
      inputAuthority: "USER",
      value: { type: "INT", value: -12 },
    },
    {
      variableId: "2",
      name: "Speed",
      typeName: "REAL",
      role: "Local",
      inputAuthority: null,
      value: { type: "REAL", value: 12.5 },
    },
    {
      variableId: "3",
      name: "Duration",
      typeName: "TIME",
      role: "Input",
      inputAuthority: "USER",
      value: { type: "TIME", microseconds: "18446744073709551615" },
    },
  ];
  await mountLoaded(s);
  expect(screen.getByLabelText("Value Count")).toHaveTextContent("-12");
  expect(screen.getByLabelText("Value Speed")).toHaveTextContent("12.5");
  expect(screen.getByLabelText("Value Duration")).toHaveTextContent(
    "T#18446744073709551615us",
  );
  expect(screen.getByLabelText("Set Duration (microseconds)")).toHaveValue(
    "18446744073709551615",
  );
  fireEvent.change(screen.getByLabelText("Set Count"), {
    target: { value: "-32768" },
  });
  expect(setRuntimeInput).not.toHaveBeenCalled();
  const next = structuredClone(s);
  next.variables[0]!.value = { type: "INT", value: -32768 };
  vi.mocked(setRuntimeInput).mockResolvedValue(next);
  await click("Apply Count");
  expect(setRuntimeInput).toHaveBeenCalledWith("1", "1", {
    type: "INT",
    value: -32768,
  });
  expect(screen.getByLabelText("Value Count")).toHaveTextContent("-32768");
});
it("invalid numeric input gives feedback without changing authoritative display", async () => {
  const s = readySnapshot();
  s.variables = [
    {
      variableId: "1",
      name: "Count",
      typeName: "INT",
      role: "Input",
      value: { type: "INT", value: 5 },
      inputAuthority: "USER",
    },
  ];
  await mountLoaded(s);
  fireEvent.change(screen.getByLabelText("Set Count"), {
    target: { value: "32768" },
  });
  await click("Apply Count");
  expect(setRuntimeInput).not.toHaveBeenCalled();
  expect(screen.getByRole("alert")).toBeInTheDocument();
  expect(screen.getByLabelText("Value Count")).toHaveTextContent("5");
});
it("runtime faults have separate markers and reset clears them via backend", async () => {
  await mountLoaded();
  const fault = {
    ...readySnapshot(),
    state: "FAULTED" as const,
    fault: runtimeFault,
  };
  vi.mocked(singleScan).mockResolvedValue(fault);
  await click("Single Scan");
  expect(
    within(screen.getByRole("region", { name: "Diagnostics" })).getByText(
      /PLC-RUNTIME-003/,
    ),
  ).toBeInTheDocument();
  expect(screen.getByTestId("st-editor")).toHaveAttribute(
    "data-runtime-markers",
    "1",
  );
  expect(screen.getByTestId("st-editor")).toHaveAttribute(
    "data-compiler-markers",
    "0",
  );
  expect(screen.getByRole("button", { name: "Single Scan" })).toBeDisabled();
  expect(screen.getByTestId("runtime-scan-count")).toHaveTextContent("0");
  expect(screen.getByTestId("runtime-time")).toHaveTextContent("0 ms");
  vi.mocked(resetRuntime).mockResolvedValue(readySnapshot());
  await click("Reset Runtime");
  expect(screen.getByTestId("st-editor")).toHaveAttribute(
    "data-runtime-markers",
    "0",
  );
  expect(screen.getByRole("button", { name: "Single Scan" })).toBeEnabled();
  expect(resetRuntime).toHaveBeenCalledWith("1");
});
it("editing removes runtime markers from new source but labels previous fault", async () => {
  await mountLoaded();
  vi.mocked(singleScan).mockResolvedValue({
    ...readySnapshot(),
    state: "FAULTED",
    fault: runtimeFault,
  });
  await click("Single Scan");
  fireEvent.change(screen.getByLabelText("ST test editor"), {
    target: { value: "PROGRAM Other END_PROGRAM" },
  });
  expect(screen.getByTestId("st-editor")).toHaveAttribute(
    "data-runtime-markers",
    "0",
  );
  expect(screen.getByTestId("runtime-diagnostics")).toHaveTextContent(
    "Previous loaded source",
  );
});
it("TON Q and ET are rendered from supplied immutable FB snapshots", async () => {
  const s = readySnapshot();
  s.functionBlocks = [
    {
      kind: "TON",
      instanceId: "1",
      name: "Delay",
      q: true,
      etMicros: "300000",
    },
  ];
  await mountLoaded(s);
  expect(screen.getByLabelText("Q Delay")).toHaveTextContent("Q = TRUE");
  expect(screen.getByLabelText("ET Delay")).toHaveTextContent("ET = T#300ms");
});
it("unload removes runtime values while preserving editor and compile metadata", async () => {
  await mountLoaded();
  const source = (
    screen.getByLabelText("ST test editor") as HTMLTextAreaElement
  ).value;
  vi.mocked(unloadProgram).mockResolvedValue({
    contractVersion: CONTRACT_VERSION,
    sessionId: "1",
  });
  await click("Unload");
  expect(screen.queryByTestId("runtime-variables")).not.toBeInTheDocument();
  expect(screen.getByTestId("compile-status")).toHaveTextContent(
    "COMPILE SUCCESS",
  );
  expect(screen.getByRole("button", { name: "Single Scan" })).toBeDisabled();
  expect(
    (screen.getByLabelText("ST test editor") as HTMLTextAreaElement).value,
  ).toBe(source);
});
