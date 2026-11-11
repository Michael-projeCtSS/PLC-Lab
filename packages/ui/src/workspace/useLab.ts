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
  type ControlLoopDto,
  type DiagnosticDto,
  type GradeReportDto,
  type LiveFrameDto,
  type OperateActionDto,
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
import { createLane, type LaneEvents } from "./requestLane";
import { reportClosing, takeClosingReports } from "./closingReports";
import { HeldButtons, type Holder, type HolderMatch } from "./heldButtons";

/** After this long without an answer a request is reported as stalled (ADR-0048). */
const STALL_SECONDS = 30;

/** Names of the commands that `act` runs, for reports about them. */
const COMMAND: Record<string, TranslationKey> = {
  build: "request.command.build",
  check: "request.command.check",
  compile: "request.command.compile",
  sim: "request.command.sim",
  mode: "request.command.mode",
  speed: "request.command.speed",
  force: "request.command.force",
  restart: "request.command.restart",
  alarm: "request.command.alarm",
  diagnose: "request.command.diagnose",
  repair: "request.command.repair",
  write: "request.command.write",
};

const ACTION: Record<OperateActionDto["type"], TranslationKey> = {
  press: "process.action.press",
  release: "process.action.release",
  selector: "process.action.selector",
  presence: "process.action.presence",
  door: "process.action.door",
};

const errorText = (e: unknown) => (e instanceof Error ? e.message : String(e));

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
  /** Undo/redo of the ladder editor (ADR-0052), kept while the workspace is open. */
  const [ladHistory, setLadHistory] = useState<{
    past: string[];
    future: string[];
  }>({ past: [], future: [] });
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
  // One use case at a time: a second command while one is pending is ignored. Operator
  // gestures are never ignored (see `operate`).
  const pending = useRef(false);
  // Every request that changes this workspace, commands and gestures alike, reaches the
  // backend in the order it was made (ADR-0048).
  const [lane] = useState(() => createLane(STALL_SECONDS * 1000));
  // The push buttons the operator holds pressed; each press gets exactly one release.
  const [held] = useState(() => new HeldButtons());
  // Wording for reports that arrive later, kept current by an effect below so the
  // request callbacks stay stable while a button is held.
  const wording = useRef({
    say: (key: TranslationKey, values?: Record<string, string | number>) =>
      translate("en", key, values),
    labels: new Map<string, string>(),
  });
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
    let name = projectId;
    (async () => {
      const p = await api.project(projectId);
      const w = await api.openWorkspace(projectId);
      opened = w;
      name = p.name;
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
      // Failures while the previous workspace closed have had no screen until now.
      for (const r of takeClosingReports())
        log(
          "warning",
          r.component
            ? wording.current.say("close.releaseFailed", {
                project: r.project,
                component: r.component,
                error: r.error,
              })
            : wording.current.say("close.failed", {
                project: r.project,
                error: r.error,
              }),
        );
    })().catch((e: unknown) => active && setFatal(e));
    return () => {
      active = false;
      if (!opened) return;
      const target = { workspaceId: opened.workspaceId };
      // Closing saves the project, so held buttons are released first and the close
      // waits for every earlier request of this workspace. Nothing of this workspace is
      // left on screen: a failure goes to the problem log and to the next workspace.
      const labels = wording.current.labels;
      for (const component of held.releaseAll())
        void lane
          .run(() => api.operate(target, component, { type: "release" }))
          .catch((e: unknown) =>
            reportClosing({
              project: name,
              component: labels.get(component) ?? component,
              error: errorText(e),
            }),
          );
      void lane
        .run(() => api.closeWorkspace(target))
        .catch((e: unknown) =>
          reportClosing({ project: name, error: errorText(e) }),
        );
    };
  }, [projectId, applyFrame, nextFrameRequest, lane, held, log]);

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

  // The PID controllers of the program in the PLC (ADR-0049), read again whenever a
  // download changes the program or its signals.
  const [controlLoops, setControlLoops] = useState<ControlLoopDto[]>([]);
  const programRevision = frame?.plc.programRevision ?? null;
  const signalsRevision = frame?.signalsRevision ?? null;
  useEffect(() => {
    if (!ws) return;
    let active = true;
    api.controlLoops(ws).then(
      (r) => {
        if (active) setControlLoops(r.loops);
      },
      (e: unknown) => {
        if (active) log("error", errorText(e));
      },
    );
    return () => {
      active = false;
    };
  }, [ws, programRevision, signalsRevision, log]);

  /** Reports about a request that is slow to answer; `request` names it. */
  const laneEvents = useCallback(
    (request: () => string): LaneEvents => ({
      onStall: (waiting) =>
        log(
          "warning",
          wording.current.say("request.stalled", {
            request: request(),
            seconds: STALL_SECONDS,
            waiting,
          }),
        ),
      onLateAnswer: (ms) =>
        log(
          "info",
          wording.current.say("request.lateAnswer", {
            request: request(),
            seconds: Math.round(ms / 1000),
          }),
        ),
    }),
    [log],
  );
  /** "Start S1: release", in the interface language. */
  const describeGesture = useCallback(
    (component: string, action: OperateActionDto) => {
      const { say, labels } = wording.current;
      return say("request.gesture", {
        component: labels.get(component) ?? component,
        action: say(ACTION[action.type]),
      });
    },
    [],
  );

  /**
   * Run a use case that may return a new frame; report errors as messages. Only one use
   * case runs at a time (further requests are ignored until it settles). It is sent
   * through the workspace lane, after any operator gesture made before it, and takes its
   * frame request number when it is sent. The authoritative frame is always re-read
   * afterwards, also after a failure.
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
      try {
        return await lane.run(
          async () => {
            const request = nextFrameRequest();
            const result = await work();
            const f = toFrame ? toFrame(result) : null;
            if (f) applyFrame(request, f);
            return result;
          },
          laneEvents(() =>
            wording.current.say(COMMAND[label] ?? "request.command.other"),
          ),
        );
      } catch (e) {
        log("error", errorText(e));
        return null;
      } finally {
        pending.current = false;
        setBusy(null);
        refreshFrame();
      }
    },
    [lane, log, laneEvents, refreshFrame, nextFrameRequest, applyFrame],
  );

  /**
   * Send an operator gesture on a process component (push-button press or release,
   * selector, presence, door). Unlike a command, a gesture is never ignored: a release
   * made while an earlier gesture, a build or a toolbar command is pending must still
   * reach the process, or the simulated button stays pressed. It goes through the
   * workspace lane in the order it was made and takes its frame request number when it
   * is sent, so the frame of the newest request still wins; the frame is re-read
   * afterwards, also after a failure, which is reported with the component and action.
   * The callback is stable once the workspace is open.
   */
  const operate = useCallback(
    (
      component: string,
      action: OperateActionDto,
    ): Promise<LiveFrameDto | null> => {
      if (!ws) return Promise.resolve(null);
      return lane
        .run(
          async () => {
            const request = nextFrameRequest();
            const f = await api.operate(ws, component, action);
            applyFrame(request, f);
            return f;
          },
          laneEvents(() => describeGesture(component, action)),
        )
        .catch((e: unknown) => {
          const { say, labels } = wording.current;
          log(
            "error",
            say("process.notApplied", {
              component: labels.get(component) ?? component,
              action: say(ACTION[action.type]),
              error: errorText(e),
            }),
          );
          return null;
        })
        .finally(refreshFrame);
    },
    [
      ws,
      lane,
      log,
      laneEvents,
      describeGesture,
      refreshFrame,
      nextFrameRequest,
      applyFrame,
    ],
  );

  /**
   * Push-button holds (ADR-0048). `hold` presses a button unless it is already held;
   * `letGo` releases it only for the holder that pressed it, so repeated or stale events
   * send nothing; `letGoAll` releases every held button (window blur, the process view
   * closing). Holds left when the workspace closes are released before the close.
   */
  const hold = useCallback(
    (component: string, holder: Holder) => {
      if (held.press(component, holder))
        void operate(component, { type: "press" });
    },
    [held, operate],
  );
  const letGo = useCallback(
    (component: string, match: HolderMatch) => {
      if (held.release(component, match))
        void operate(component, { type: "release" });
    },
    [held, operate],
  );
  const letGoAll = useCallback(() => {
    for (const component of held.releaseAll())
      void operate(component, { type: "release" });
  }, [held, operate]);
  // A window that loses focus may never see the pointerup or keyup of a held button.
  useEffect(() => {
    window.addEventListener("blur", letGoAll);
    return () => window.removeEventListener("blur", letGoAll);
  }, [letGoAll]);

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
  /** The language of the main program: "ST" or "LAD" (ADR-0050). */
  const mainLanguage = project?.programs[0]?.language ?? "ST";
  /** Replace the main program by `text` in `language` and save the project. */
  const switchLanguage = useCallback(
    async (language: "ST" | "LAD", text: string): Promise<boolean> => {
      if (!project) return false;
      const [main, ...rest] = currentPrograms(project);
      if (!main) return false;
      try {
        const saved = await api.saveProject({
          ...project,
          programs: [{ ...main, language, source: text }, ...rest],
        });
        setProject(saved);
        setSource(text);
        setActiveDocument(null);
        return true;
      } catch (e) {
        log("error", errorText(e));
        return false;
      }
    },
    [project, currentPrograms, setSource, log],
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
  useEffect(() => {
    wording.current = {
      say,
      labels: new Map(
        presentedWorkspace?.scene.components.map((c) => [c.id, c.label]) ?? [],
      ),
    };
  }, [say, presentedWorkspace]);
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
    mainLanguage,
    switchLanguage,
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
    ladHistory,
    setLadHistory,
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
    operate,
    controlLoops,
    hold,
    letGo,
    letGoAll,
    buildAndLoad,
    check,
    refresh: refreshFrame,
  };
}
