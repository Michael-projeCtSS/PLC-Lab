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
import { Inspector } from "./Inspector";
import { Dock } from "./Dock";

export type CenterTab =
  | "program"
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
  const lab = useLab(projectId);
  const [tab, setTab] = useState<CenterTab>("program");
  const [selection, setSelection] = useState<Selection>(null);
  const [rightWidth, setRightWidth] = useState(340);
  const [dockHeight, setDockHeight] = useState(240);
  const [mode, setMode] = useState<"basic" | "advanced">("basic");
  const troubleshooting = isFaultFinding(lab.workspace?.exercise?.kind);
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
          Back
        </button>
      </div>
    );
  if (!lab.workspace || !lab.frame || !lab.project)
    return <Empty>Opening the lab…</Empty>;
  const select = (s: Selection) => {
    setSelection(s);
    if (s?.kind === "component") setTab("process");
    if (s?.kind === "module") setTab("hardware");
    if (s?.kind === "device") setTab("network");
  };
  const alarms = lab.frame.alarms.length + lab.frame.activeDiagnostics;
  return (
    <div className="ws">
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
      <div className="ws-body">
        <aside className="ws-left" aria-label="Project">
          <ProjectTree
            lab={lab}
            tab={tab}
            setTab={setTab}
            selection={selection}
            select={select}
          />
        </aside>
        <div className="ws-main">
          <div
            className="row"
            style={{ flex: 1, minHeight: 0, alignItems: "stretch", gap: 0 }}
          >
            <section className="ws-center grow" aria-label="Editor">
              <Tabs<CenterTab>
                label="Workspace views"
                active={tab}
                onSelect={setTab}
                tabs={[
                  { id: "program", label: "Program" },
                  { id: "hardware", label: "Hardware" },
                  { id: "process", label: "Process" },
                  ...(lab.workspace.scene.hasNetwork
                    ? [{ id: "network" as const, label: "Network" }]
                    : []),
                  {
                    id: "diagnostics",
                    label: "Diagnostics",
                    ...(alarms > 0
                      ? { badge: <Badge kind="warning">{alarms}</Badge> }
                      : {}),
                  },
                  { id: "commissioning", label: "Commissioning" },
                ]}
              />
              <div className="ws-center-body">
                {tab === "program" && <ProgramTab lab={lab} />}
                {tab === "process" && (
                  <ProcessTab
                    lab={lab}
                    selection={selection}
                    select={select}
                    troubleshooting={troubleshooting}
                  />
                )}
                {tab === "hardware" && (
                  <HardwareTab
                    lab={lab}
                    selection={selection}
                    select={select}
                    advanced={mode === "advanced"}
                  />
                )}
                {tab === "network" && (
                  <NetworkTab lab={lab} advanced={mode === "advanced"} />
                )}
                {tab === "diagnostics" && <DiagnosticsTab lab={lab} />}
                {tab === "commissioning" && <CommissioningTab lab={lab} />}
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
              aria-label="Inspector"
            >
              <Inspector lab={lab} selection={selection} navigate={navigate} />
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
            <Dock lab={lab} />
          </div>
        </div>
      </div>
    </div>
  );
}
