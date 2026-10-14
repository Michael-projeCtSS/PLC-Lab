import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CoreInfoError, getCoreInfo, compileSt } from "@plc-lab/ipc";
import type { CompilerDiagnostic } from "@plc-lab/ipc";
import { LabShell } from "./LabShell";
import { CONTRACT_VERSION } from "../../ipc/src/generated";

vi.mock("@plc-lab/ipc", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@plc-lab/ipc")>()),
  getCoreInfo: vi.fn(),
  compileSt: vi.fn(),
}));
// Monaco is a presentation dependency; actual Monaco/native IPC are tested in desktop smoke.
vi.mock("./StructuredTextEditor", () => ({
  StructuredTextEditor: ({
    source,
    onChange,
    diagnostics,
  }: {
    source: string;
    onChange: (value: string) => void;
    diagnostics: CompilerDiagnostic[];
  }) => (
    <textarea
      aria-label="ST test editor"
      data-testid="st-editor"
      data-marker-count={diagnostics.length}
      value={source}
      onChange={(event) => onChange(event.target.value)}
    />
  ),
}));

describe("foundation desktop shell", () => {
  beforeEach(() => {
    vi.mocked(getCoreInfo).mockResolvedValue({
      applicationName: "PLC Lab",
      appVersion: "app-test",
      coreVersion: "core-test",
      contractVersion: CONTRACT_VERSION,
      execution: "manual-single-scan",
      buildContext: "development",
    });
  });

  it("reserves all seven areas and disables execution controls", async () => {
    render(<LabShell />);
    for (const name of [
      "Project",
      "Program",
      "Variables",
      "Process",
      "Network",
      "Timeline",
      "Diagnostics",
    ]) {
      expect(screen.getByRole("region", { name })).toBeInTheDocument();
    }
    for (const name of ["Run", "Pause", "Stop", "Single Scan"]) {
      expect(screen.getByRole("button", { name })).toBeDisabled();
    }
    await waitFor(() =>
      expect(screen.getByTestId("core-status")).toHaveTextContent(
        "Rust backend connected",
      ),
    );
    expect(screen.getByTestId("core-status")).toHaveTextContent(
      "Application app-test · Core core-test",
    );
    expect(screen.getByText("No trace recorded")).toBeInTheDocument();
    expect(screen.getByText("No runtime variables")).toBeInTheDocument();
    expect(screen.getByTestId("core-status")).toHaveTextContent("DEVELOPMENT");
  });

  it("shows native errors without pretending to connect or execute", async () => {
    vi.mocked(getCoreInfo).mockRejectedValue(
      new CoreInfoError("APP-IPC-003", "Native core unavailable."),
    );
    render(<LabShell />);
    await waitFor(() =>
      expect(screen.getByTestId("core-status")).toHaveTextContent(
        "APP-IPC-003",
      ),
    );
    expect(screen.queryByText("CORE CONNECTED")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Run" })).toBeDisabled();
  });

  it("does not update after unmount while the IPC request is pending", async () => {
    let resolveRequest:
      ((value: Awaited<ReturnType<typeof getCoreInfo>>) => void) | undefined;
    vi.mocked(getCoreInfo).mockReturnValue(
      new Promise((resolve) => {
        resolveRequest = resolve;
      }),
    );
    const view = render(<LabShell />);
    expect(screen.getByTestId("core-status")).toHaveTextContent(
      "Reading native core information",
    );
    view.unmount();
    resolveRequest?.({
      applicationName: "PLC Lab",
      appVersion: "0.1.0",
      coreVersion: "0.1.0",
      contractVersion: CONTRACT_VERSION,
      execution: "manual-single-scan",
      buildContext: "development",
    });
    await Promise.resolve();
    expect(screen.queryByTestId("core-status")).not.toBeInTheDocument();
  });

  it("shows the Rust-supplied release context without changing execution controls", async () => {
    vi.mocked(getCoreInfo).mockResolvedValue({
      applicationName: "PLC Lab",
      appVersion: "release-app",
      coreVersion: "release-core",
      contractVersion: CONTRACT_VERSION,
      execution: "manual-single-scan",
      buildContext: "release",
    });
    render(<LabShell />);
    await waitFor(() =>
      expect(screen.getByTestId("core-status")).toHaveTextContent("RELEASE"),
    );
    expect(screen.getByRole("button", { name: "Run" })).toBeDisabled();
  });

  it("shows compile metadata and success without runtime values or enabled controls", async () => {
    vi.mocked(compileSt).mockResolvedValue({
      success: true,
      programName: "Main",
      symbols: [{ name: "Motor", typeName: "BOOL", role: "Output" }],
      diagnostics: [],
    });
    render(<LabShell />);
    expect(
      (screen.getByLabelText("ST test editor") as HTMLTextAreaElement).value,
    ).toContain("PROGRAM Main");
    expect(screen.getByTestId("compile-status")).toHaveTextContent(
      "NOT COMPILED",
    );
    fireEvent.click(screen.getByRole("button", { name: "Compile" }));
    await waitFor(() =>
      expect(screen.getByTestId("compile-status")).toHaveTextContent(
        "COMPILE SUCCESS",
      ),
    );
    expect(
      within(screen.getByTestId("compiled-symbols")).getByText("Motor"),
    ).toBeInTheDocument();
    expect(
      within(screen.getByTestId("compiled-symbols")).getByText("Output"),
    ).toBeInTheDocument();
    expect(
      screen.getByLabelText("Runtime value unavailable"),
    ).toHaveTextContent("—");
    for (const name of ["Run", "Pause", "Stop", "Single Scan"])
      expect(screen.getByRole("button", { name })).toBeDisabled();
    expect(screen.getByText("No process scene")).toBeInTheDocument();
  });
  it("displays backend diagnostics and sends them to the editor; editing clears obsolete errors", async () => {
    vi.mocked(compileSt).mockResolvedValue({
      success: false,
      diagnostics: [
        {
          code: "PLC-ST-006",
          severity: "error",
          message: "Unknown variable 'Missing'.",
          span: {
            start: { line: 3, column: 10 },
            end: { line: 3, column: 17 },
          },
        },
      ],
    });
    render(<LabShell />);
    fireEvent.click(screen.getByRole("button", { name: "Compile" }));
    await waitFor(() =>
      expect(screen.getByTestId("compile-status")).toHaveTextContent(
        "COMPILE FAILED",
      ),
    );
    expect(screen.getByTestId("compiler-diagnostics")).toHaveTextContent(
      "ERROR · PLC-ST-006",
    );
    expect(screen.getByTestId("compiler-diagnostics")).toHaveTextContent(
      "Line 3, column 10",
    );
    expect(screen.getByTestId("st-editor")).toHaveAttribute(
      "data-marker-count",
      "1",
    );
    fireEvent.change(screen.getByLabelText("ST test editor"), {
      target: { value: "PROGRAM Fixed END_PROGRAM" },
    });
    expect(screen.getByTestId("compile-status")).toHaveTextContent(
      "NOT COMPILED",
    );
    expect(screen.getByTestId("compiler-diagnostics")).not.toHaveTextContent(
      "PLC-ST-006",
    );
  });
});
