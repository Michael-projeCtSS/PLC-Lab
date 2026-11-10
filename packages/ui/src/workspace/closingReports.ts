/**
 * Requests that failed while a workspace was closing (ADR-0048): the release of a held
 * push button or the close itself. Nothing of that workspace is left on screen, so each
 * failure is recorded in the problem log at once and kept here, also across a reload,
 * until the next workspace that opens reports it in its Messages.
 *
 * Nothing stays pressed afterwards: process state is not saved with the project, so a
 * reopened workspace starts with every button released, and opening any workspace
 * saves the previous one if its close did not arrive.
 */
import { recordProblem } from "../error-log";

export type ClosingReport = {
  time: string;
  project: string;
  /** The component whose release failed; absent when the close itself failed. */
  component?: string;
  error: string;
};

const KEY = "plc-lab.closing-reports.v1";
const LIMIT = 20;
let memory: ClosingReport[] = [];

function read(): ClosingReport[] {
  try {
    const raw = localStorage.getItem(KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed)
      ? parsed.filter(
          (r): r is ClosingReport =>
            typeof r === "object" &&
            r !== null &&
            typeof (r as ClosingReport).time === "string" &&
            typeof (r as ClosingReport).project === "string" &&
            typeof (r as ClosingReport).error === "string",
        )
      : [];
  } catch (e) {
    recordProblem(
      "workspace",
      `Closing reports could not be read: ${e instanceof Error ? e.message : String(e)}`,
    );
    return memory;
  }
}

function write(reports: ClosingReport[]) {
  memory = reports;
  try {
    if (reports.length) localStorage.setItem(KEY, JSON.stringify(reports));
    else localStorage.removeItem(KEY);
  } catch (e) {
    // Kept in memory for this session; the problem log says why it was not saved.
    recordProblem(
      "workspace",
      `Closing reports could not be saved: ${e instanceof Error ? e.message : String(e)}`,
    );
  }
}

/** Record a failure while a workspace closed. */
export function reportClosing(report: Omit<ClosingReport, "time">) {
  const entry = { time: new Date().toISOString(), ...report };
  recordProblem(
    "workspace",
    entry.component
      ? `Releasing ${entry.component} while closing ${entry.project} failed: ${entry.error}`
      : `Closing ${entry.project} failed: ${entry.error}`,
  );
  write([...read(), entry].slice(-LIMIT));
}

/** The reports not shown yet, oldest first; they are removed. */
export function takeClosingReports(): ClosingReport[] {
  const reports = read();
  write([]);
  return reports;
}
