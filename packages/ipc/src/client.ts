/**
 * Typed client for the PLC Lab desktop application (contract v8). Every response is
 * validated before it reaches the user interface. The client contains no PLC,
 * process, network or grading logic.
 */
import { listen } from "@tauri-apps/api/event";
import {
  CONTRACT_VERSION,
  EXECUTION_WAKE_EVENT,
  OPEN_REQUEST_EVENT,
} from "./generated";
import type * as G from "./generated";
import { invokeNative, nativeAvailable } from "./transport";
import { unit, type Validator } from "./validate";
import * as V from "./validators";

/** Native command names; must match the host's `generate_handler!` list. */
export const COMMANDS = [
  "core_info",
  "knowledge_status",
  "knowledge_search",
  "knowledge_article",
  "knowledge_catalog",
  "library",
  "lesson",
  "complete_lesson",
  "exercise",
  "projects",
  "create_project",
  "project",
  "save_project",
  "save_project_as",
  "update_source",
  "delete_project",
  "import_project",
  "export_project",
  "open_workspace",
  "close_workspace",
  "compile",
  "build_and_load",
  "plc_mode",
  "plc_reset",
  "simulation",
  "set_speed",
  "operate",
  "force",
  "frame",
  "cycle",
  "variables",
  "trend",
  "events",
  "hardware",
  "commissioning",
  "network",
  "measure",
  "ping",
  "note_evidence",
  "diagnose",
  "repair",
  "check",
  "take_open_request",
] as const;
export type Command = (typeof COMMANDS)[number];

async function call<T>(
  command: Command,
  request: Record<string, unknown>,
  validator: Validator<T>,
): Promise<T> {
  const response = await invokeNative(command, {
    contractVersion: CONTRACT_VERSION,
    ...request,
  });
  return validator(response, command);
}

type Ws = { workspaceId: G.ExactU64 };

export const api = {
  available: nativeAvailable,
  coreInfo: () => call("core_info", {}, V.coreInfo),

  knowledgeStatus: () => call("knowledge_status", {}, V.knowledgeStatus),
  search: (
    query: string,
    filters: {
      category?: string | null;
      vendor?: string | null;
      docType?: string | null;
    } = {},
    limit = 40,
  ) =>
    call(
      "knowledge_search",
      {
        query,
        category: filters.category ?? null,
        vendor: filters.vendor ?? null,
        docType: filters.docType ?? null,
        limit,
      },
      V.searchHits,
    ),
  article: (id: string) => call("knowledge_article", { id }, V.article),
  catalog: () => call("knowledge_catalog", {}, V.catalog),
  library: () => call("library", {}, V.library),
  lesson: (id: string) => call("lesson", { id }, V.lesson),
  completeLesson: (id: string, correct: number, total: number) =>
    call("complete_lesson", { id, correct, total }, V.library),
  exercise: (id: string) => call("exercise", { id }, V.exercise),

  projects: () => call("projects", {}, V.projectList),
  createProject: (request: {
    name: string;
    exercise?: string | null;
    scene?: string | null;
    platform?: string | null;
  }) =>
    call(
      "create_project",
      {
        name: request.name,
        exercise: request.exercise ?? null,
        scene: request.scene ?? null,
        platform: request.platform ?? null,
      },
      V.project,
    ),
  project: (id: string) => call("project", { id }, V.project),
  saveProject: (project: G.ProjectDto) =>
    call("save_project", { project }, V.project),
  saveProjectAs: (projectId: string, name: string) =>
    call("save_project_as", { projectId, name }, V.project),
  updateSource: (
    projectId: string,
    programId: string,
    source: string,
    revision: number,
  ) =>
    call(
      "update_source",
      { projectId, programId, source, revision: String(revision) },
      V.projectSummary,
    ),
  deleteProject: (id: string) => call("delete_project", { id }, V.projectList),
  importProject: (text: string) => call("import_project", { text }, V.project),
  exportProject: (id: string) =>
    call("export_project", { id }, V.projectExport),

  openWorkspace: (projectId: string) =>
    call("open_workspace", { projectId }, V.workspace),
  closeWorkspace: (w: Ws) => call("close_workspace", w, unit),
  compile: (source: string) => call("compile", { source }, V.compileResult),
  buildAndLoad: (
    w: Ws,
    source: string,
    revision: number,
    station: G.StationDto | null,
    runAfterLoad: boolean,
  ) =>
    call(
      "build_and_load",
      { ...w, source, revision: String(revision), station, runAfterLoad },
      V.buildLoadResult,
    ),
  plcMode: (w: Ws, mode: "RUN" | "STOP") =>
    call("plc_mode", { ...w, mode }, V.frame),
  plcReset: (w: Ws) => call("plc_reset", w, V.frame),
  simulation: (w: Ws, command: "RUN" | "PAUSE" | "STEP" | "STOP") =>
    call("simulation", { ...w, command }, V.frame),
  setSpeed: (w: Ws, speedPercent: number) =>
    call("set_speed", { ...w, speedPercent }, V.frame),
  operate: (w: Ws, component: string, action: G.OperateActionDto) =>
    call("operate", { ...w, component, action }, V.frame),
  force: (w: Ws, signal: number, value: G.SignalValueDto | null) =>
    call("force", { ...w, signal, value }, V.frame),
  frame: (w: Ws, watch: number[] = []) =>
    call("frame", { ...w, watch }, V.frame),
  cycle: (w: Ws, cycle: string) =>
    call("cycle", { ...w, cycle }, (v, p) =>
      v === null ? null : V.causeEffect(v, p),
    ),
  variables: (w: Ws, filter: string | null, offset: number, limit: number) =>
    call("variables", { ...w, filter, offset, limit }, V.variables),
  trend: (
    w: Ws,
    signals: number[],
    fromUs: string,
    toUs: string,
    maxPoints: number,
  ) => call("trend", { ...w, signals, fromUs, toUs, maxPoints }, V.trend),
  events: (w: Ws, after: string | null, limit: number) =>
    call("events", { ...w, after, limit }, V.eventsPage),
  hardware: (w: Ws) => call("hardware", w, V.hardware),
  commissioning: (w: Ws) => call("commissioning", w, V.commissioning),
  network: (w: Ws) => call("network", w, V.network),
  measure: (w: Ws, channel: G.ChannelRefDto) =>
    call("measure", { ...w, channel }, V.measurement),
  ping: (w: Ws, ip: string) => call("ping", { ...w, ip }, V.ping),
  noteEvidence: (w: Ws, evidence: string) =>
    call("note_evidence", { ...w, evidence }, unit),
  diagnose: (w: Ws, option: string) =>
    call("diagnose", { ...w, option }, V.diagnosis),
  repair: (w: Ws, repair: string) => call("repair", { ...w, repair }, V.repair),
  check: (w: Ws, source: string, station: G.StationDto | null) =>
    call("check", { ...w, source, station }, V.gradeReport),

  /** The project another launch asked to open, once (resolved by the host). */
  takeOpenRequest: () => call("take_open_request", {}, V.openRequest),
};

export type Api = typeof api;

/** Subscribe to "another launch asked to open a project" notifications. */
export async function onOpenRequest(handler: () => void): Promise<() => void> {
  if (!nativeAvailable()) return () => undefined;
  return listen<unknown>(OPEN_REQUEST_EVENT, (event) => {
    V.openRequestEvent(event.payload, OPEN_REQUEST_EVENT);
    handler();
  });
}

/** Subscribe to simulation wake notifications; the payload is validated. */
export async function onCycleAdvanced(
  handler: (wake: G.ExecutionWakeDto) => void,
): Promise<() => void> {
  if (!nativeAvailable()) return () => undefined;
  return listen<unknown>(EXECUTION_WAKE_EVENT, (event) => {
    handler(V.executionWake(event.payload, EXECUTION_WAKE_EVENT));
  });
}
