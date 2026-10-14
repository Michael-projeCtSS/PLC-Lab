import { render, screen, fireEvent } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { ProcessPanel } from "./ProcessPanel";
import { TANK_EXAMPLE_SOURCE, tankInterfaceCompatible } from "./tank-example";
import { RuntimeVariables } from "./RuntimeVariables";
import {
  readySnapshot,
  tankSnapshot,
} from "../../../tests/frontend/runtime-fixtures";

const props = () => ({
  source: "PROGRAM Existing END_PROGRAM",
  snapshot: null,
  current: false,
  canManage: false,
  onExample: vi.fn(),
  onAttach: vi.fn(async () => {}),
  onDetach: vi.fn(async () => {}),
  onReset: vi.fn(async () => {}),
});
it("offers the demo and interface without automatically replacing source or attaching", () => {
  const p = props();
  render(<ProcessPanel {...p} />);
  expect(screen.getByText("No process scene attached")).toBeInTheDocument();
  expect(screen.getByText("Tank Filling Demo")).toBeInTheDocument();
  expect(screen.getByText(/Required PLC interface/)).toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: "Attach Tank Scene" }),
  ).toBeDisabled();
  expect(
    screen.getByText(/Compile and load a compatible PLC program first/),
  ).toBeInTheDocument();
  expect(p.onExample).not.toHaveBeenCalled();
  expect(p.onAttach).not.toHaveBeenCalled();
});
it("requires explicit confirmation before replacing different editor source and supports cancel", () => {
  const p = props();
  render(<ProcessPanel {...p} />);
  fireEvent.click(screen.getByRole("button", { name: "Use Example" }));
  expect(p.onExample).not.toHaveBeenCalled();
  expect(screen.getByRole("alertdialog")).toHaveTextContent("source only");
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(p.onExample).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Use Example" }));
  fireEvent.click(screen.getByRole("button", { name: "Replace Source" }));
  expect(p.onExample).toHaveBeenCalledTimes(1);
  expect(p.onAttach).not.toHaveBeenCalled();
  expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
});
it("an explicit example action on already identical text needs no replacement confirmation", () => {
  const p = { ...props(), source: TANK_EXAMPLE_SOURCE };
  render(<ProcessPanel {...p} />);
  fireEvent.click(screen.getByRole("button", { name: "Use Example" }));
  expect(p.onExample).toHaveBeenCalledTimes(1);
  expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
});
it("checks interface metadata case-insensitively and enables only current ready attachment", () => {
  const s = tankSnapshot();
  s.process = null;
  s.variables.forEach((v) => {
    if (v.role === "Input") v.inputAuthority = "USER";
    v.name = v.name.toLowerCase();
  });
  expect(tankInterfaceCompatible(s)).toBe(true);
  expect(tankInterfaceCompatible(readySnapshot())).toBe(false);
  const p = { ...props(), snapshot: s, current: true, canManage: true };
  const r = render(<ProcessPanel {...p} />);
  expect(
    screen.getByRole("button", { name: "Attach Tank Scene" }),
  ).toBeEnabled();
  fireEvent.click(screen.getByRole("button", { name: "Attach Tank Scene" }));
  expect(p.onAttach).toHaveBeenCalledTimes(1);
  r.rerender(<ProcessPanel {...p} current={false} />);
  expect(
    screen.getByRole("button", { name: "Attach Tank Scene" }),
  ).toBeDisabled();
  r.rerender(<ProcessPanel {...p} snapshot={{ ...s, state: "FAULTED" }} />);
  expect(
    screen.getByRole("button", { name: "Attach Tank Scene" }),
  ).toBeDisabled();
});
it("renders tank geometry and all labels from a fixed authoritative snapshot", () => {
  render(
    <ProcessPanel {...props()} snapshot={tankSnapshot()} current canManage />,
  );
  expect(screen.getByTestId("tank-level")).toHaveTextContent("30.0 %");
  expect(screen.getByTestId("tank-fill")).toHaveAttribute("height", "31.5");
  expect(screen.getByTestId("tank-fill")).toHaveAttribute("y", "109.5");
  expect(screen.getByTestId("inlet-state")).toHaveTextContent("OPEN");
  expect(screen.getByTestId("pump-state")).toHaveTextContent("STOPPED");
  expect(screen.getByTestId("low-state")).toHaveTextContent("INACTIVE");
  expect(screen.getByTestId("high-state")).toHaveTextContent("INACTIVE");
});
it("uses supplied sensor/actuator booleans without deriving them from tank level", () => {
  const s = tankSnapshot();
  s.process = {
    ...s.process!,
    levelPermille: 0,
    lowLevel: false,
    highLevel: true,
    inletValveOpen: false,
    drainPumpRunning: true,
  };
  render(<ProcessPanel {...props()} snapshot={s} current canManage />);
  expect(screen.getByTestId("tank-fill")).toHaveAttribute("height", "0");
  expect(screen.getByTestId("low-state")).toHaveTextContent("INACTIVE");
  expect(screen.getByTestId("high-state")).toHaveTextContent("ACTIVE");
  expect(screen.getByTestId("inlet-state")).toHaveTextContent("CLOSED");
  expect(screen.getByTestId("pump-state")).toHaveTextContent("RUNNING");
});
it("reset process delegates without locally changing physical or PLC state", () => {
  const p = {
    ...props(),
    snapshot: tankSnapshot(),
    current: true,
    canManage: true,
  };
  render(<ProcessPanel {...p} />);
  fireEvent.click(screen.getByRole("button", { name: "Reset Process" }));
  expect(p.onReset).toHaveBeenCalledTimes(1);
  expect(screen.getByTestId("tank-level")).toHaveTextContent("30.0 %");
  expect(p.snapshot.completedScans).toBe("17");
});
it("supplied runtime-reset and fault observations preserve the rendered plant", () => {
  const s = tankSnapshot();
  const p = { ...props(), snapshot: s, current: true, canManage: true };
  const r = render(<ProcessPanel {...p} />);
  r.rerender(
    <ProcessPanel
      {...p}
      snapshot={{ ...s, completedScans: "0", simulationTimeMicros: "0" }}
    />,
  );
  expect(screen.getByTestId("tank-level")).toHaveTextContent("30.0 %");
  r.rerender(<ProcessPanel {...p} snapshot={{ ...s, state: "FAULTED" }} />);
  expect(screen.getByTestId("tank-level")).toHaveTextContent("30.0 %");
  expect(screen.getByTestId("inlet-state")).toHaveTextContent("OPEN");
});
it("old-source plant stays visible and stale while process mutations are disabled", () => {
  render(
    <ProcessPanel
      {...props()}
      snapshot={tankSnapshot()}
      current={false}
      canManage={false}
    />,
  );
  expect(
    screen.getByText(/STALE.*process execution disabled/),
  ).toBeInTheDocument();
  expect(screen.getByTestId("tank-level")).toHaveTextContent("30.0 %");
  expect(screen.getByRole("button", { name: "Reset Process" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Detach Process" })).toBeDisabled();
});
it("owned Inputs show PROCESS and no editor while ordinary Inputs retain editors", () => {
  render(
    <RuntimeVariables
      snapshot={tankSnapshot()}
      editable
      observationKey="1"
      setInput={vi.fn(async () => {})}
    />,
  );
  for (const name of ["LevelLow", "LevelHigh", "LevelPercent"]) {
    expect(screen.getByLabelText(`Authority ${name}`)).toHaveTextContent(
      "PROCESS",
    );
    expect(screen.queryByLabelText(`Set ${name}`)).not.toBeInTheDocument();
  }
  expect(screen.getByLabelText("Authority User")).toHaveTextContent("USER");
  expect(screen.getByLabelText("Set User")).toBeEnabled();
});
it("detach action delegates and a supplied detached snapshot restores USER inputs", () => {
  const p = {
    ...props(),
    snapshot: tankSnapshot(),
    current: true,
    canManage: true,
  };
  const r = render(<ProcessPanel {...p} />);
  fireEvent.click(screen.getByRole("button", { name: "Detach Process" }));
  expect(p.onDetach).toHaveBeenCalledTimes(1);
  const detached = { ...p.snapshot, process: null };
  detached.variables = detached.variables.map((v) => ({
    ...v,
    inputAuthority: v.role === "Input" ? ("USER" as const) : null,
  }));
  r.rerender(
    <>
      <ProcessPanel {...p} snapshot={detached} />
      <RuntimeVariables
        snapshot={detached}
        editable
        observationKey="2"
        setInput={vi.fn(async () => {})}
      />
    </>,
  );
  expect(screen.queryByTestId("tank-scene")).not.toBeInTheDocument();
  expect(screen.getByLabelText("Set LevelLow")).toBeEnabled();
});
