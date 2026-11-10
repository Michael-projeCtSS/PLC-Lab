import { beforeEach, describe, expect, it, vi } from "vitest";
import { recentProblems } from "../error-log";
import { reportClosing, takeClosingReports } from "./closingReports";

beforeEach(() => {
  vi.restoreAllMocks();
  localStorage.clear();
  takeClosingReports();
});

describe("closing reports", () => {
  it("records a failed release in the problem log and keeps it for the next workspace", () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    reportClosing({
      project: "Motor start/stop",
      component: "Start S1",
      error: "The workspace is no longer open.",
    });
    expect(recentProblems().at(-1)).toMatchObject({
      kind: "workspace",
      message:
        "Releasing Start S1 while closing Motor start/stop failed: The workspace is no longer open.",
    });
    const [report, ...rest] = takeClosingReports();
    expect(rest).toEqual([]);
    expect(report).toMatchObject({
      project: "Motor start/stop",
      component: "Start S1",
      error: "The workspace is no longer open.",
    });
    // Shown once.
    expect(takeClosingReports()).toEqual([]);
  });

  it("survives a reload through local storage", () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    reportClosing({ project: "Tank", error: "Timeout" });
    const saved = JSON.parse(
      localStorage.getItem("plc-lab.closing-reports.v1") ?? "[]",
    ) as unknown[];
    expect(saved).toHaveLength(1);
    expect(takeClosingReports()).toMatchObject([
      { project: "Tank", error: "Timeout" },
    ]);
    expect(localStorage.getItem("plc-lab.closing-reports.v1")).toBeNull();
  });

  it("keeps the reports in memory when storage fails, and says so", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("quota");
    });
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("denied");
    });
    reportClosing({ project: "Tank", error: "Timeout" });
    expect(warn).toHaveBeenCalledWith(
      expect.stringContaining("Closing reports could not be saved: quota"),
    );
    expect(takeClosingReports()).toMatchObject([
      { project: "Tank", error: "Timeout" },
    ]);
  });

  it("ignores stored entries that are not reports", () => {
    localStorage.setItem(
      "plc-lab.closing-reports.v1",
      JSON.stringify([
        { project: 1 },
        "x",
        { time: "t", project: "P", error: "E" },
      ]),
    );
    expect(takeClosingReports()).toEqual([
      { time: "t", project: "P", error: "E" },
    ]);
  });
});
