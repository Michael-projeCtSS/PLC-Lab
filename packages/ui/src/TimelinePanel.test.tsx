import { fireEvent, render, screen, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { TimelinePanel } from "./TimelinePanel";
import { traceEntry, tracePage } from "../../../tests/frontend/trace-fixtures";
import {
  tankSnapshot,
  readySnapshot,
  runtimeFault,
} from "../../../tests/frontend/runtime-fixtures";
const props = () => ({
  page: null,
  current: false,
  pending: false,
  error: null,
  onDiagnostic: vi.fn(),
});
it("shows an empty read-only Timeline before Load", () => {
  render(<TimelinePanel {...props()} />);
  expect(screen.getByText(/No trace recorded/)).toBeInTheDocument();
  expect(screen.queryByRole("button")).not.toBeInTheDocument();
});
it("shows SessionLoaded and exact sequence/scan/simulation time", () => {
  render(<TimelinePanel {...props()} page={tracePage()} current />);
  expect(
    screen.getByRole("button", { name: /#1 SESSION LOADED Scan 0 0 ms READY/ }),
  ).toBeInTheDocument();
});
it("selection displays recorded scan 8/9 observations instead of live state", () => {
  const a = tankSnapshot(),
    b = tankSnapshot();
  a.process = {
    ...a.process!,
    levelPermille: 800,
    highLevel: true,
    inletValveOpen: true,
  };
  b.process = {
    ...b.process!,
    levelPermille: 800,
    highLevel: true,
    inletValveOpen: false,
  };
  a.completedScans = "8";
  a.simulationTimeMicros = "800000";
  b.completedScans = "9";
  b.simulationTimeMicros = "900000";
  const p = tracePage([
    traceEntry("10", "ScanCompleted", a),
    traceEntry("11", "ScanCompleted", b),
  ]);
  const navigate = vi.fn();
  render(
    <TimelinePanel {...props()} page={p} current onDiagnostic={navigate} />,
  );
  fireEvent.click(screen.getAllByRole("button")[0]!);
  let inspector = screen.getByTestId("trace-inspector");
  expect(inspector).toHaveTextContent("Historical Trace · #10");
  expect(inspector).toHaveTextContent("800 ms");
  expect(within(inspector).getByTestId("historical-tank")).toHaveTextContent(
    "Inlet ValveOPEN",
  );
  expect(inspector).toHaveTextContent("80.0 %");
  fireEvent.click(screen.getAllByRole("button")[1]!);
  inspector = screen.getByTestId("trace-inspector");
  expect(inspector).toHaveTextContent("Historical Trace · #11");
  expect(inspector).toHaveTextContent("900 ms");
  expect(within(inspector).getByTestId("historical-tank")).toHaveTextContent(
    "Inlet ValveCLOSED",
  );
  expect(navigate).not.toHaveBeenCalled();
  expect(a.process.inletValveOpen).toBe(true);
});
it("shows historical PLC variables, TON and fault provenance via existing bridge", () => {
  const s = readySnapshot();
  s.state = "FAULTED";
  s.fault = runtimeFault;
  s.functionBlocks = [
    {
      kind: "TON",
      instanceId: "7",
      name: "Delay",
      q: true,
      etMicros: "300000",
    },
  ];
  const onDiagnostic = vi.fn();
  render(
    <TimelinePanel
      {...props()}
      page={tracePage([traceEntry("2", "ScanFaulted", s)])}
      current
      onDiagnostic={onDiagnostic}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: /SCAN FAULTED/ }));
  expect(screen.getByTestId("trace-inspector")).toHaveTextContent(
    "PLC-RUNTIME-003",
  );
  expect(screen.getByTestId("trace-inspector")).toHaveTextContent(
    "Delay · TON · Q TRUE · ET 300 ms",
  );
  fireEvent.click(
    screen.getByRole("button", { name: "Focus historical source expression" }),
  );
  expect(onDiagnostic).toHaveBeenCalledWith(runtimeFault);
});
it("labels stale history and disables source navigation without hiding inspection", () => {
  const s = readySnapshot();
  s.state = "FAULTED";
  s.fault = runtimeFault;
  render(
    <TimelinePanel
      {...props()}
      page={tracePage([traceEntry("2", "ScanFaulted", s)])}
    />,
  );
  expect(screen.getByText(/STALE/)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /SCAN FAULTED/ }));
  expect(
    screen.getByRole("button", { name: "Focus historical source expression" }),
  ).toBeDisabled();
});
it("shows explicit dropped-history and internal saturation status", () => {
  render(
    <TimelinePanel
      {...props()}
      page={{ ...tracePage(), droppedCount: "999", gap: true, saturated: true }}
      current
    />,
  );
  expect(
    screen.getByText(/Earlier trace entries were discarded/),
  ).toHaveTextContent("999");
  expect(screen.getByRole("alert")).toHaveTextContent(
    "INTERNAL TRACE STATUS · SATURATED",
  );
});
it("formats exact large count/time strings without unsafe Number conversion", () => {
  const s = readySnapshot();
  s.completedScans = "9007199254740993";
  s.simulationTimeMicros = "18446744073709551615";
  render(
    <TimelinePanel
      {...props()}
      page={tracePage([traceEntry("9007199254740993", "ScanCompleted", s)])}
      current
    />,
  );
  expect(screen.getByRole("button")).toHaveTextContent("9007199254740993");
  expect(screen.getByRole("button")).toHaveTextContent(
    "18446744073709551615 µs",
  );
});
it("presents retained internal application errors distinctly", () => {
  const s = readySnapshot();
  s.state = "FAULTED";
  s.fault = {
    ...runtimeFault,
    code: "APP-CORE-001",
    category: "internalApplicationError",
  };
  render(
    <TimelinePanel
      {...props()}
      page={tracePage([traceEntry("3", "ProcessReset", s)])}
      current
    />,
  );
  fireEvent.click(screen.getByRole("button"));
  expect(screen.getByTestId("trace-inspector")).toHaveTextContent(
    "APPLICATION ERROR · APP-CORE-001",
  );
});
