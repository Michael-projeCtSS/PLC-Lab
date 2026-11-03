import { useInterface } from "../locale";
import { useState } from "react";
import { ErrorNotice, Empty, Splitter, Tabs, Badge } from "../ui";
import { isFaultFinding, type Navigate, type Section } from "../route";
import { useLab } from "./useLab";
import { Toolbar } from "./Toolbar";
import { ProjectTree, type Selection } from "./ProjectTree";
import { ProgramTab } from "./ProgramTab";
import { ProcessTab } from "./ProcessTab";
import { HardwareTab } from "./HardwareTab";
import { NetworkTab } from "./NetworkTab";
import { DiagnosticsTab } from "./DiagnosticsTab";
import { CommissioningTab } from "./CommissioningTab";
import type { AlarmDto } from "@plc-lab/ipc";
import { AlarmIndicator, AlarmsTab } from "./AlarmsTab";
import { needsAcknowledgment } from "./alarmText";
import { Inspector } from "./Inspector";
import { Dock } from "./Dock";
import { ForceBanner } from "./ForceBanner";

export type CenterTab =
  | "program"
  | "alarms"
  | "hardware"
  | "process"
  | "network"
  | "diagnostics"
  | "commissioning";

export function WorkspacePage({
  projectId,
  navigate,
  back,
}: {
  projectId: string;
  navigate: Navigate;
  back: Section;
}) {
  const { t, preferences } = useInterface();
  const lab = useLab(projectId);
  const [tab, setTab] = useState<CenterTab>("program");
  const [selection, setSelection] = useState<Selection>(null);
  const [rightWidth, setRightWidth] = useState(340);
  const [dockHeight, setDockHeight] = useState(240);
  const [mode, setMode] = useState<"basic" | "advanced">("basic");
  const troubleshooting = isFaultFinding(lab.workspace?.exercise?.kind);
  const simple = mode === "basic" && !lab.workspace?.exercise;
  const alarms = lab.frame?.activeDiagnostics ?? 0;
  // Alarms that need attention, and those still waiting for acknowledgment.
  const plantAlarms = lab.frame?.alarms ?? [];
  const waiting = plantAlarms.filter(needsAcknowledgment).length;
  const visibleTab =
    simple &&
    tab !== "process" &&
    !(tab === "diagnostics" && alarms > 0) &&
    !(tab === "alarms" && plantAlarms.length > 0)
      ? "program"
      : tab;
  if (lab.fatal)
    return (
      <div className="page-inner">
        <ErrorNotice error={lab.fatal} />
        <button
          className="btn"
          onClick={() =>
            navigate(
              back === "learn"
                ? { page: "learn", lesson: null }
                : back === "knowledge"
                  ? { page: "knowledge", article: null, query: "" }
                  : { page: back },
            )
          }
        >
          {t("common.back")}
        </button>
      </div>
    );
  if (!lab.workspace || !lab.frame || !lab.project)
    return <Empty>{t("ws.opening")}</Empty>;
  const select = (s: Selection) => {
    setSelection(s);
    if (s?.kind === "component") setTab("process");
    if (s?.kind === "module") setTab("hardware");
    if (s?.kind === "device") setTab("network");
  };
  const inspect = (a: AlarmDto) => {
    if (a.category === "PROCESS")
      select({ kind: "component", id: a.component });
    else if (a.category === "NETWORK" && lab.workspace?.scene.hasNetwork)
      setTab(simple ? "diagnostics" : "network");
    else setTab(simple ? "diagnostics" : "hardware");
  };
  return (
    <div className={`ws${simple ? " ws-basic" : ""}`}>
      <Toolbar
        lab={lab}
        mode={mode}
        setMode={setMode}
        onBack={() =>
          navigate(
            back === "learn"
              ? { page: "learn", lesson: null }
              : back === "knowledge"
                ? { page: "knowledge", article: null, query: "" }
                : { page: back },
          )
        }
      />
      {simple && preferences.explanations && (
        <p className="student-hint">{t("ws.advancedHint")}</p>
      )}
      <ForceBanner lab={lab} />
      {mode === "basic" && visibleTab !== "alarms" && (
        <AlarmIndicator alarms={plantAlarms} open={() => setTab("alarms")} />
      )}
      <div className="ws-body">
        {!simple && (
          <aside className="ws-left" aria-label={t("ws.project")}>
            <ProjectTree
              lab={lab}
              tab={tab}
              setTab={setTab}
              selection={selection}
              select={select}
            />
          </aside>
        )}
        <div className="ws-main">
          <div
            className="row"
            style={{ flex: 1, minHeight: 0, alignItems: "stretch", gap: 0 }}
          >
            <section className="ws-center grow" aria-label={t("ws.editor")}>
              <Tabs<CenterTab>
                label={t("ws.views")}
                active={visibleTab}
                onSelect={setTab}
                tabs={[
                  { id: "program", label: t("ws.program") },
                  ...(!simple
                    ? [{ id: "hardware" as const, label: t("ws.hardware") }]
                    : []),
                  ...(simple && alarms > 0
                    ? [
                        {
                          id: "diagnostics" as const,
                          label: t("ws.diagnostics"),
                          badge: <Badge kind="warning">{alarms}</Badge>,
                        },
                      ]
                    : []),
                  { id: "process", label: t("ws.process") },
                  ...(!simple || plantAlarms.length > 0
                    ? [
                        {
                          id: "alarms" as const,
                          label: t("ws.alarms"),
                          ...(waiting > 0
                            ? { badge: <Badge kind="fault">{waiting}</Badge> }
                            : {}),
                        },
                      ]
                    : []),
                  ...(!simple && lab.workspace.scene.hasNetwork
                    ? [{ id: "network" as const, label: t("ws.network") }]
                    : []),
                  ...(!simple
                    ? [
                        {
                          id: "diagnostics" as const,
                          label: t("ws.diagnostics"),
                          ...(alarms > 0
                            ? { badge: <Badge kind="warning">{alarms}</Badge> }
                            : {}),
                        },
                        {
                          id: "commissioning" as const,
                          label: t("ws.commissioning"),
                        },
                      ]
                    : []),
                ]}
              />
              <div className="ws-center-body">
                {visibleTab === "program" && <ProgramTab lab={lab} />}
                {visibleTab === "process" && (
                  <ProcessTab
                    lab={lab}
                    selection={selection}
                    select={select}
                    troubleshooting={troubleshooting}
                  />
                )}
                {visibleTab === "hardware" && (
                  <HardwareTab
                    lab={lab}
                    selection={selection}
                    select={select}
                    advanced={mode === "advanced"}
                  />
                )}
                {visibleTab === "network" && (
                  <NetworkTab lab={lab} advanced={mode === "advanced"} />
                )}
                {visibleTab === "diagnostics" && <DiagnosticsTab lab={lab} />}
                {visibleTab === "alarms" && (
                  <AlarmsTab
                    lab={lab}
                    advanced={mode === "advanced"}
                    inspect={inspect}
                  />
                )}
                {visibleTab === "commissioning" && (
                  <CommissioningTab lab={lab} />
                )}
              </div>
            </section>
            <Splitter
              direction="vertical"
              size={rightWidth}
              min={260}
              max={620}
              invert
              onResize={setRightWidth}
            />
            <aside
              className="ws-right"
              style={{ width: rightWidth }}
              aria-label={t("ws.inspector")}
            >
              <Inspector
                lab={lab}
                selection={selection}
                navigate={navigate}
                simple={simple}
              />
            </aside>
          </div>
          <Splitter
            direction="horizontal"
            size={dockHeight}
            min={120}
            max={560}
            invert
            onResize={setDockHeight}
          />
          <div className="ws-dock" style={{ height: dockHeight }}>
            <Dock lab={lab} select={select} advanced={!simple} />
          </div>
        </div>
      </div>
    </div>
  );
}
