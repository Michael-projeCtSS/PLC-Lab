import { api } from "@plc-lab/ipc";

export type Section =
  | "home"
  | "learn"
  | "practice"
  | "troubleshoot"
  | "freelab"
  | "knowledge"
  | "progress";

export type Route =
  | { page: "home" }
  | { page: "learn"; lesson: string | null }
  | { page: "practice" }
  | { page: "troubleshoot" }
  | { page: "freelab" }
  | { page: "knowledge"; article: string | null; query: string }
  | { page: "progress" }
  | { page: "about" }
  | { page: "workspace"; projectId: string; section: Section };

export type Navigate = (route: Route) => void;

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
