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
  type SourceDocumentDto,
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

/** A project library document: FUNCTIONs and FUNCTION_BLOCKs (ADR-0045). */
export type LibraryDocument = { id: string; name: string; source: string };

/** Each library document's text by id. */
function texts(documents: LibraryDocument[]): Record<string, string> {
  return Object.fromEntries(documents.map((d) => [d.id, d.source]));
}

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
  // The project library documents and the document shown in the editor (`null` is the
  // main program), with the texts their diagnostics and the PLC's program belong to.
  const [library, setLibrary] = useState<LibraryDocument[]>([]);
  const [activeDocument, setActiveDocument] = useState<string | null>(null);
  const [diagnosticsLibrary, setDiagnosticsLibrary] = useState<
    Record<string, string>
  >({});
  const [loadedLibrary, setLoadedLibrary] = useState<Record<string, string>>(
    {},
  );
  const librarySources = useMemo<SourceDocumentDto[]>(
    () => library.map(({ id, source }) => ({ id, source })),
    [library],
  );
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
      const documents = p.programs
        .slice(1)
        .map(({ id, name, source }) => ({ id, name, source }));
      setLibrary(documents);
      setLoadedLibrary(texts(documents));
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
  /** Show `value` in the active document: the main program or a library document. */
  const setEditorText = useCallback(
    (value: string) => {
      if (activeDocument === null) setSource(value);
      else
        setLibrary((documents) =>
          documents.map((d) =>
            d.id === activeDocument ? { ...d, source: value } : d,
          ),
        );
    },
    [activeDocument, setSource],
  );
  useEffect(() => {
    if (!project) return;
    const timer = setTimeout(() => {
      api.compile(source, librarySources).then(
        (r) => {
          setDiagnostics(r.diagnostics);
          setDiagnosticsSource(source);
          setDiagnosticsLibrary(
            Object.fromEntries(librarySources.map((d) => [d.id, d.source])),
          );
          if (r.success) setPous(r.pous);
          setPousCurrent(r.success);
        },
        () => undefined,
      );
    }, 350);
    return () => clearTimeout(timer);
  }, [source, librarySources, project]);
  // Library documents are saved like the main program, each on its own.
  useEffect(() => {
    if (!project) return;
    const changed = library.filter((d) =>
      project.programs.some((p) => p.id === d.id && p.source !== d.source),
    );
    if (!changed.length) return;
    const timer = setTimeout(() => {
      for (const d of changed) {
        revision.current += 1;
        api.updateSource(project.id, d.id, d.source, revision.current).then(
          () =>
            setProject((p) =>
              p
                ? {
                    ...p,
                    programs: p.programs.map((x) =>
                      x.id === d.id ? { ...x, source: d.source } : x,
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
      }
    }, 800);
    return () => clearTimeout(timer);
  }, [library, project, log, say]);
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

  /** The name of a library document, or `null` for the main program. */
  const documentName = useCallback(
    (id: string | null | undefined) =>
      id ? (library.find((d) => d.id === id)?.name ?? id) : null,
    [library],
  );
  const applyBuild = useCallback(
    (
      r: BuildLoadResultDto,
      built: string,
      builtLibrary: SourceDocumentDto[],
    ) => {
      const builtTexts = Object.fromEntries(
        builtLibrary.map((d) => [d.id, d.source]),
      );
      setSignals(r.signals);
      setDiagnostics(r.diagnostics);
      setDiagnosticsSource(built);
      setDiagnosticsLibrary(builtTexts);
      if (r.outcome === "LOADED") {
        setLoadedSource(built);
        setLoadedLibrary(builtTexts);
      }
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
          r.diagnostics
            .map((d) => diagnosticLine(say, d, documentName(d.document)))
            .join("\n"),
        );
      else if (r.outcome === "BINDING_FAILED")
        log("error", say("build.binding"), issues);
      else if (r.outcome === "HARDWARE_REJECTED")
        log("error", say("build.hardware"), r.message ?? undefined);
      return r.frame;
    },
    [log, say, documentName],
  );

  const buildAndLoad = useCallback(
    (runAfterLoad: boolean, station: StationDto | null = null) => {
      if (!ws) return Promise.resolve(null);
      revision.current += 1;
      const rev = revision.current;
      const built = librarySources;
      return act(
        "build",
        () => api.buildAndLoad(ws, source, rev, station, runAfterLoad, built),
        (r) => applyBuild(r, source, built),
      ).then((r) => {
        if (r?.outcome === "LOADED" && station)
          setProject((p) => (p ? { ...p, station } : p));
        return r;
      });
    },
    [ws, source, librarySources, act, applyBuild],
  );

  const check = useCallback(async () => {
    if (!ws) return;
    const r = await act("check", () =>
      api.check(ws, source, null, librarySources),
    );
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
        [
          ...r.problems,
          ...r.diagnostics.map((d) =>
            diagnosticLine(say, d, documentName(d.document)),
          ),
        ].join("\n"),
      );
  }, [ws, source, librarySources, act, log, say, documentName]);

  /** The project's documents with the texts being edited. */
  const currentPrograms = useCallback(
    (p: ProjectDto, documents: LibraryDocument[] = library) => [
      ...p.programs.slice(0, 1).map((d) => ({ ...d, source })),
      ...documents.map((d) => ({ ...d, language: "ST" })),
    ],
    [source, library],
  );
  /** Save a changed list of library documents in the project (ADR-0045). */
  const saveLibrary = useCallback(
    async (documents: LibraryDocument[]): Promise<boolean> => {
      if (!project) return false;
      try {
        const saved = await api.saveProject({
          ...project,
          programs: currentPrograms(project, documents),
        });
        setProject(saved);
        setLibrary(documents);
        return true;
      } catch (e) {
        log("error", e instanceof Error ? e.message : String(e));
        return false;
      }
    },
    [project, currentPrograms, log],
  );
  const addDocument = useCallback(async () => {
    const ids = new Set(library.map((d) => d.id));
    let n = 1;
    while (ids.has(`lib-${n}`)) n += 1;
    const names = new Set(library.map((d) => d.name.toLowerCase()));
    let k = library.length + 1;
    while (names.has(say("doc.defaultName", { n: k }).toLowerCase())) k += 1;
    const document = {
      id: `lib-${n}`,
      name: say("doc.defaultName", { n: k }),
      source: "",
    };
    if (await saveLibrary([...library, document]))
      setActiveDocument(document.id);
  }, [library, saveLibrary, say]);
  const renameDocument = useCallback(
    (id: string, name: string) =>
      saveLibrary(
        library.map((d) => (d.id === id ? { ...d, name: name.trim() } : d)),
      ),
    [library, saveLibrary],
  );
  const deleteDocument = useCallback(
    async (id: string) => {
      if (await saveLibrary(library.filter((d) => d.id !== id)))
        setActiveDocument((active) => (active === id ? null : active));
    },
    [library, saveLibrary],
  );

  /** Save an edited station configuration into the project. The backend validates it;
   * the returned project is the authoritative state (it reaches the PLC on download). */
  const saveStation = useCallback(
    async (station: StationDto): Promise<boolean> => {
      if (!project) return false;
      const next = {
        ...project,
        station,
        programs: currentPrograms(project),
      };
      try {
        setProject(await api.saveProject(next));
        return true;
      } catch (e) {
        log("error", e instanceof Error ? e.message : String(e));
        return false;
      }
    },
    [log, project, currentPrograms],
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
        programs: currentPrograms(project),
      };
      setProject(next);
      api
        .saveProject(next)
        .then(undefined, (e: unknown) =>
          log("error", e instanceof Error ? e.message : String(e)),
        );
    },
    [log, project, currentPrograms],
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
    library,
    librarySources,
    activeDocument,
    setActiveDocument,
    editorText: library.find((d) => d.id === activeDocument)?.source ?? source,
    setEditorText,
    documentName,
    addDocument,
    renameDocument,
    deleteDocument,
    diagnosticsLibrary,
    loadedLibrary,
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
