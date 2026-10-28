import { api } from "@plc-lab/ipc";

export type Section =
  | "home"
  | "learn"
  | "practice"
  | "commission"
  | "troubleshoot"
  | "freelab"
  | "knowledge"
  | "progress";

export type Route =
  | { page: "home" }
  | { page: "learn"; lesson: string | null }
  | { page: "practice" }
  | { page: "commission" }
  | { page: "troubleshoot" }
  | { page: "freelab" }
  | { page: "knowledge"; article: string | null; query: string }
  | { page: "progress" }
  | { page: "about" }
  | { page: "workspace"; projectId: string; section: Section };

export type Navigate = (route: Route) => void;

export type ExerciseKind = "practice" | "troubleshooting" | "commissioning";

/** Labs with a hidden installation fault: diagnose, repair, then verify. */
export function isFaultFinding(kind: ExerciseKind | undefined): boolean {
  return kind === "troubleshooting" || kind === "commissioning";
}

/** The navigation section an exercise of this kind belongs to. */
export function sectionOf(kind: ExerciseKind): Section {
  return kind === "practice"
    ? "practice"
    : kind === "commissioning"
      ? "commission"
      : "troubleshoot";
}

/** Continue the newest project for an exercise, or create one, then open its workspace. */
export async function openExercise(
  exercise: string,
  navigate: Navigate,
  section: Section,
) {
  const list = await api.projects();
  const existing = list.projects.find((p) => p.exercise === exercise);
  const projectId = existing
    ? existing.id
    : (await api.createProject({ name: "", exercise })).id;
  navigate({ page: "workspace", projectId, section });
}
