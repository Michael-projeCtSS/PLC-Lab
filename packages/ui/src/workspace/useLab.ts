/**
 * Workspace state: one open lab workspace, its latest authoritative frame, the program
 * draft and user messages. All PLC/process truth comes from the desktop core; this hook
 * only requests use cases and keeps the latest validated projections.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  api,
  onCycleAdvanced,
  type BuildLoadResultDto,
  type DiagnosticDto,
  type GradeReportDto,
  type LiveFrameDto,
  type PouDto,
  type ProjectDto,
  type SignalDto,
  type StationDto,
  type WatchTableDto,
  type WorkspaceDto,
} from "@plc-lab/ipc";
import { useCoalescedRead } from "../useCoalescedRead";
import { translate, useInterface } from "../locale";
import type { TranslationKey } from "../locale/en";
import { diagnosticLine } from "../locale/messages";
import { rememberDraft } from "../error-log";
import { localizeContent } from "../locale/content";
import { newTableId, newTableName, setRows, updateTable } from "./watchEdit";

export type Message = {
  id: number;
  time: string;
  kind: "info" | "success" | "warning" | "error";
  text: string;
  detail?: string;
};

export type Lab = ReturnType<typeof useLab>;

export function useLab(projectId: string) {
  const { preferences } = useInterface();
  const say = useCallback(
    (key: TranslationKey, values?: Record<string, string | number>) =>
      translate(preferences.language, key, values),
    [preferences.language],
  );
  const [workspace, setWorkspace] = useState<WorkspaceDto | null>(null);
  const [project, setProject] = useState<ProjectDto | null>(null);
  const [frame, setFrame] = useState<LiveFrameDto | null>(null);
  const [signals, setSignals] = useState<SignalDto[]>([]);
  const [source, setSourceState] = useState("");
  const [diagnostics, setDiagnostics] = useState<DiagnosticDto[]>([]);
  // The exact text `diagnostics` were computed for, and the program the PLC runs: the
  // editor marks positions only in the text they belong to.
  // The program's own functions and function blocks, from the latest successful
  // compile, and whether the current text compiles (ADR-0044).
  const [pous, setPous] = useState<PouDto[]>([]);
  const [pousCurrent, setPousCurrent] = useState(true);
  const [diagnosticsSource, setDiagnosticsSource] = useState<string | null>(
    null,
  );
  const [loadedSource, setLoadedSource] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [report, setReport] = useState<GradeReportDto | null>(null);
  const [watchTables, setWatchTables] = useState<WatchTableDto[]>([]);
  const [activeTable, setActiveTable] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [fatal, setFatal] = useState<unknown>(null);
  const revision = useRef(0);
  // Frames come from reads and from use cases. Each request takes a number when it is
  // sent; a response older than the newest applied one is dropped, so a slow read can
  // never overwrite the frame a later use case returned.
  const frameRequests = useRef(0);
  const frameApplied = useRef(0);
  const nextFrameRequest = useCallback(() => ++frameRequests.current, []);
  const applyFrame = useCallback((request: number, f: LiveFrameDto) => {
    if (request < frameApplied.current) return;
    frameApplied.current = request;
    setFrame(f);
  }, []);
  // One use case at a time: a second request while one is pending is ignored.
  const pending = useRef(false);
  const ws = useMemo(
    () => (workspace ? { workspaceId: workspace.workspaceId } : null),
    [workspace],
  );
  // The active watch table drives the live frame's watch values and the waveforms.
  const table =
    watchTables.find((t) => t.id === activeTable) ?? watchTables[0] ?? null;
  const watch = useMemo(() => table?.rows.map((r) => r.signal) ?? [], [table]);
  const watchIds = useMemo(
    () =>
      watch
        .map((path) => signals.find((s) => s.path === path)?.id)
        .filter((id): id is number => id !== undefined),
    [watch, signals],
  );
  const wsId = ws?.workspaceId ?? null;
  const [wakes, setWakes] = useState(0);

  const log = useCallback(
    (kind: Message["kind"], text: string, detail?: string) => {
      setMessages((m) => [
        ...m.slice(-199),
        {
          id: Date.now() + Math.random(),
          time: new Date().toLocaleTimeString(),
          kind,
          text,
          ...(detail ? { detail } : {}),
        },
      ]);
    },
    [],
  );

  // Open on mount, close (and persist) on unmount.
  useEffect(() => {
    let active = true;
    let opened: WorkspaceDto | null = null;
    (async () => {
      const p = await api.project(projectId);
      const w = await api.openWorkspace(projectId);
      opened = w;
      if (!active) {
        void api
          .closeWorkspace({ workspaceId: w.workspaceId })
          .catch(() => undefined);
        return;
      }
      setProject(p);
      setWorkspace(w);
      applyFrame(nextFrameRequest(), w.frame);
      setSignals(w.signals);
      setSourceState(p.programs[0]?.source ?? "");
      setLoadedSource(p.programs[0]?.source ?? null);
      setWatchTables(p.watchTables);
      setActiveTable(p.watchTables[0]?.id ?? null);
    })().catch((e: unknown) => active && setFatal(e));
    return () => {
      active = false;
      if (opened)
        void api
          .closeWorkspace({ workspaceId: opened.workspaceId })
          .catch(() => undefined);
    };
  }, [projectId, applyFrame, nextFrameRequest]);

  // Frame refresh: one read in flight, coalesced; driven by wake events and actions.
  const refreshFrame = useCallback(() => setWakes((n) => n + 1), []);
  useCoalescedRead(async () => {
    if (!ws) return;
    const request = nextFrameRequest();
    try {
      applyFrame(request, await api.frame(ws, watchIds));
    } catch (e) {
      log("error", e instanceof Error ? e.message : String(e));
    }
  }, [wakes, ws, watchIds.join(",")]);
  useEffect(() => {
    if (wsId === null) return;
    let unlisten: (() => void) | null = null;
    let active = true;
    onCycleAdvanced((wake) => {
      if (wake.workspaceId === wsId) refreshFrame();
    }).then(
      (u) => (active ? (unlisten = u) : u()),
      () => undefined,
    );
    return () => {
      active = false;
      unlisten?.();
    };
  }, [wsId, refreshFrame]);

  /**
   * Run a use case that may return a new frame; report errors as messages. Only one use
   * case runs at a time (further requests are ignored until it settles), and the
   * authoritative frame is always re-read afterwards, also after a failure.
   */
  const act = useCallback(
    async <T>(
      label: string,
      work: () => Promise<T>,
      toFrame?: (r: T) => LiveFrameDto | null,
    ): Promise<T | null> => {
      if (pending.current) return null;
      pending.current = true;
      setBusy(label);
      const request = nextFrameRequest();
      try {
        const result = await work();
        const f = toFrame ? toFrame(result) : null;
        if (f) applyFrame(request, f);
        return result;
      } catch (e) {
        log("error", e instanceof Error ? e.message : String(e));
        return null;
      } finally {
        pending.current = false;
        setBusy(null);
        refreshFrame();
      }
    },
    [log, refreshFrame, nextFrameRequest, applyFrame],
  );

  // Program draft: live compile diagnostics and autosave, both debounced.
  // The latest text is kept outside React as it arrives, so the error screen can show it
  // even if a view stops rendering in the same update.
  const setSource = useCallback(
    (value: string) => {
      rememberDraft(projectId, value);
      setSourceState(value);
    },
    [projectId],
  );
  useEffect(() => {
    if (!project) return;
    const timer = setTimeout(() => {
      api.compile(source).then(
        (r) => {
          setDiagnostics(r.diagnostics);
          setDiagnosticsSource(source);
          if (r.success) setPous(r.pous);
          setPousCurrent(r.success);
        },
        () => undefined,
      );
    }, 350);
    return () => clearTimeout(timer);
  }, [source, project]);
  useEffect(() => {
    const doc = project?.programs[0];
    if (!project || !doc || doc.source === source) return;
    const timer = setTimeout(() => {
      revision.current += 1;
      api.updateSource(project.id, doc.id, source, revision.current).then(
        () =>
          setProject((p) =>
            p
              ? {
                  ...p,
                  programs: p.programs.map((d, i) =>
                    i === 0 ? { ...d, source } : d,
                  ),
                }
              : p,
          ),
        (e: unknown) =>
          log(
            "error",
            say("build.autosave", {
              error: e instanceof Error ? e.message : String(e),
            }),
          ),
      );
    }, 800);
    return () => clearTimeout(timer);
  }, [source, project, log, say]);

  const applyBuild = useCallback(
    (r: BuildLoadResultDto, built: string) => {
      setSignals(r.signals);
      setDiagnostics(r.diagnostics);
      setDiagnosticsSource(built);
      if (r.outcome === "LOADED") setLoadedSource(built);
      const issues = r.bindingIssues
        .map((i) => `${i.tag}: ${i.message}`)
        .join("\n");
      const program = r.frame.plc.programName;
      if (r.outcome === "LOADED")
        log(
          "success",
          program
            ? say("build.loaded", { name: program })
            : say("build.loadedPlain"),
          issues || undefined,
        );
      else if (r.outcome === "COMPILE_FAILED")
        log(
          "error",
          say("build.failed", { count: r.diagnostics.length }),
          r.diagnostics.map((d) => diagnosticLine(say, d)).join("\n"),
        );
      else if (r.outcome === "BINDING_FAILED")
        log("error", say("build.binding"), issues);
      else if (r.outcome === "HARDWARE_REJECTED")
        log("error", say("build.hardware"), r.message ?? undefined);
      return r.frame;
    },
    [log, say],
  );

  const buildAndLoad = useCallback(
    (runAfterLoad: boolean, station: StationDto | null = null) => {
      if (!ws) return Promise.resolve(null);
      revision.current += 1;
      const rev = revision.current;
      return act(
        "build",
        () => api.buildAndLoad(ws, source, rev, station, runAfterLoad),
        (r) => applyBuild(r, source),
      ).then((r) => {
        if (r?.outcome === "LOADED" && station)
          setProject((p) => (p ? { ...p, station } : p));
        return r;
      });
    },
    [ws, source, act, applyBuild],
  );

  const check = useCallback(async () => {
    if (!ws) return;
    const r = await act("check", () => api.check(ws, source, null));
    if (!r) return;
    setReport(r);
    if (r.outcome === "GRADED")
      log(
        r.passed ? "success" : "warning",
        say(r.passed ? "check.passed" : "check.score", {
          score: r.score,
          max: r.maxScore,
        }),
      );
    else
      log(
        "error",
        say("check.notRun", {
          reason: r.outcome.replaceAll("_", " ").toLowerCase(),
        }),
        [...r.problems, ...r.diagnostics.map((d) => d.message)].join("\n"),
      );
  }, [ws, source, act, log, say]);

  /** Save an edited station configuration into the project. The backend validates it;
   * the returned project is the authoritative state (it reaches the PLC on download). */
  const saveStation = useCallback(
    async (station: StationDto): Promise<boolean> => {
      if (!project) return false;
      const next = {
        ...project,
        station,
        programs: project.programs.map((d, i) =>
          i === 0 ? { ...d, source } : d,
        ),
      };
      try {
        setProject(await api.saveProject(next));
        return true;
      } catch (e) {
        log("error", e instanceof Error ? e.message : String(e));
        return false;
      }
    },
    [log, project, source],
  );

  /** Save the watch tables in the project; the backend validates the limits. */
  const saveWatchTables = useCallback(
    (tables: WatchTableDto[], active?: string | null) => {
      setWatchTables(tables);
      if (active !== undefined) setActiveTable(active);
      if (!project) return;
      const next = {
        ...project,
        watchTables: tables,
        programs: project.programs.map((d, i) =>
          i === 0 ? { ...d, source } : d,
        ),
      };
      setProject(next);
      api
        .saveProject(next)
        .then(undefined, (e: unknown) =>
          log("error", e instanceof Error ? e.message : String(e)),
        );
    },
    [log, project, source],
  );

  /** Watch exactly these signal paths in the active table (created when none exists). */
  const setWatch = useCallback(
    (paths: string[]) => {
      const current = table ?? {
        id: newTableId(watchTables),
        name: newTableName(watchTables),
        rows: [],
      };
      const next = setRows(current, paths);
      saveWatchTables(updateTable(watchTables, next), next.id);
    },
    [table, watchTables, saveWatchTables],
  );

  const presentedWorkspace = useMemo(
    () => localizeContent(workspace, preferences.language),
    [workspace, preferences.language],
  );
  const presentedReport = useMemo(
    () =>
      localizeContent(report, preferences.language, workspace?.exercise?.id),
    [report, preferences.language, workspace?.exercise?.id],
  );
  return {
    projectId,
    workspace: presentedWorkspace,
    project,
    frame,
    signals,
    ws,
    source,
    setSource,
    diagnostics,
    diagnosticsSource,
    pous,
    pousCurrent,
    loadedSource,
    messages,
    clearMessages: () => setMessages([]),
    log,
    report: presentedReport,
    watch,
    setWatch,
    watchTables,
    activeTable: table?.id ?? null,
    selectTable: setActiveTable,
    saveWatchTables,
    saveStation,
    watchIds,
    busy,
    fatal,
    act,
    buildAndLoad,
    check,
    refresh: refreshFrame,
  };
}
