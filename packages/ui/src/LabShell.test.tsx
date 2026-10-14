import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CoreInfoError, getCoreInfo } from "@plc-lab/ipc";
import { LabShell } from "./LabShell";
import { CONTRACT_VERSION } from "../../ipc/src/generated";

vi.mock("@plc-lab/ipc", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@plc-lab/ipc")>()),
  getCoreInfo: vi.fn(),
}));

describe("foundation desktop shell", () => {
  beforeEach(() => {
    vi.mocked(getCoreInfo).mockResolvedValue({
      applicationName: "PLC Lab",
      appVersion: "app-test",
      coreVersion: "core-test",
      contractVersion: CONTRACT_VERSION,
      execution: "unavailable",
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
      execution: "unavailable",
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
      execution: "unavailable",
      buildContext: "release",
    });
    render(<LabShell />);
    await waitFor(() =>
      expect(screen.getByTestId("core-status")).toHaveTextContent("RELEASE"),
    );
    expect(screen.getByRole("button", { name: "Run" })).toBeDisabled();
  });
});
