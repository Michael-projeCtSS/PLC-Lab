/**
 * A bounded log of interface problems (render errors, uncaught exceptions, rejected
 * promises, editor problems) and the latest program draft, kept outside React so they
 * survive a component that stopped rendering. Messages and stacks only: no program text,
 * settings or other user input is logged.
 */
export type Problem = {
  time: string;
  kind: "render" | "error" | "rejection" | "editor";
  message: string;
  detail?: string;
};

const LIMIT = 50;
const DETAIL_LIMIT = 4000;
const problems: Problem[] = [];

export function recordProblem(
  kind: Problem["kind"],
  message: string,
  detail?: string,
) {
  problems.push({
    time: new Date().toISOString(),
    kind,
    message: message.slice(0, 500),
    ...(detail ? { detail: detail.slice(0, DETAIL_LIMIT) } : {}),
  });
  if (problems.length > LIMIT) problems.shift();
  console.warn(`[PLC Lab ${kind}] ${message}`);
}

export function recentProblems(): readonly Problem[] {
  return [...problems];
}

let installed = false;
/** Uncaught exceptions and unhandled promise rejections go to the bounded log. */
export function installProblemLog() {
  if (installed || typeof window === "undefined") return;
  installed = true;
  window.addEventListener("error", (e) => {
    const error = e.error as unknown;
    recordProblem(
      "error",
      e.message || String(error),
      error instanceof Error ? error.stack : undefined,
    );
  });
  window.addEventListener("unhandledrejection", (e) => {
    const reason = e.reason as unknown;
    recordProblem(
      "rejection",
      reason instanceof Error ? reason.message : String(reason),
      reason instanceof Error ? reason.stack : undefined,
    );
  });
}

/** The program text currently in the editor of an open project. */
export type Draft = { projectId: string; source: string; time: string };
let draft: Draft | null = null;

export function rememberDraft(projectId: string, source: string) {
  draft = { projectId, source, time: new Date().toISOString() };
}

export function currentDraft(): Draft | null {
  return draft;
}
