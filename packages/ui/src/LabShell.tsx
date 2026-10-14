import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { CoreInfoError, getCoreInfo } from "@plc-lab/ipc";
import type { CoreInfo } from "@plc-lab/ipc";

type Connection =
  | { kind: "connecting" }
  | { kind: "connected"; info: CoreInfo }
  | { kind: "unavailable"; code: string; message: string };

function Panel({
  id,
  title,
  detail,
  children,
  className = "",
}: {
  id: string;
  title: string;
  detail: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={`panel ${className}`} aria-labelledby={`${id}-title`}>
      <header className="panel-heading">
        <h2 id={`${id}-title`}>{title}</h2>
        <span>{detail}</span>
      </header>
      {children}
    </section>
  );
}

function EmptyState({
  mark,
  title,
  children,
}: {
  mark: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <div className="empty-state">
      <span className="empty-mark" aria-hidden="true">
        {mark}
      </span>
      <h3>{title}</h3>
      <p>{children}</p>
    </div>
  );
}

export function LabShell() {
  const [connection, setConnection] = useState<Connection>({
    kind: "connecting",
  });
  useEffect(() => {
    let active = true;
    getCoreInfo().then(
      (info) => {
        if (active) setConnection({ kind: "connected", info });
      },
      (error: unknown) => {
        if (active)
          setConnection({
            kind: "unavailable",
            code:
              error instanceof CoreInfoError ? error.code : "APP-INTERNAL-001",
            message:
              error instanceof Error
                ? error.message
                : "The application could not read core information.",
          });
      },
    );
    return () => {
      active = false;
    };
  }, []);

  return (
    <div className="lab-shell">
      <header className="app-header">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            ⌁
          </span>
          <div>
            <h1>PLC Lab</h1>
            <p>Industrial automation workspace</p>
          </div>
        </div>
        <div className="header-meta">
          <span className="foundation-badge">FOUNDATION</span>
          <span className="session-label">No project open</span>
        </div>
      </header>
      <nav className="toolbar" aria-label="Execution controls">
        <div className="execution-buttons">
          {[
            ["▷", "Run"],
            ["Ⅱ", "Pause"],
            ["□", "Stop"],
            ["↦", "Single Scan"],
          ].map(([icon, label]) => (
            <button
              key={label}
              type="button"
              disabled
              aria-describedby="execution-status"
            >
              <span aria-hidden="true">{icon}</span>
              {label}
            </button>
          ))}
        </div>
        <p id="execution-status">
          <span className="status-dot" />
          Execution unavailable in this foundation build
        </p>
      </nav>
      <main className="workspace">
        <aside className="sidebar">
          <Panel id="project" title="Project" detail="EXPLORER">
            <div className="project-empty">
              <span className="folder-mark" aria-hidden="true">
                ▱
              </span>
              <h3>Your workspace starts here</h3>
              <p>
                Project files and exercises will appear here when project
                support is available.
              </p>
              <span className="muted-label">No project loaded</span>
            </div>
            <div className="project-footnote">
              <span className="eyebrow">LANGUAGE ROADMAP</span>
              <p>
                ST <span>→</span> LAD <span>→</span> FBD <span>→</span> STL
              </p>
            </div>
          </Panel>
          <Panel id="variables" title="Variables" detail="WATCH">
            <div className="table-heading" aria-hidden="true">
              <span>Name</span>
              <span>Type</span>
              <span>Value</span>
            </div>
            <div className="compact-empty">
              <p>No runtime variables</p>
              <span>Values will come from the PLC core.</span>
            </div>
          </Panel>
        </aside>
        <div className="workbench">
          <Panel
            id="program"
            title="Program"
            detail="ST FIRST"
            className="program-panel"
          >
            <EmptyState mark="{ }" title="No program open">
              The program editor will live here.
              <br />
              Language support begins with Structured Text.
            </EmptyState>
            <div className="panel-footer">
              <span>Editor unavailable</span>
              <span>Structured Text</span>
            </div>
          </Panel>
          <div className="simulation-panels">
            <Panel id="process" title="Process" detail="SCENE">
              <EmptyState mark="◉" title="No process scene">
                Devices and feedback will be visualized from authoritative
                simulation state.
              </EmptyState>
            </Panel>
            <Panel id="network" title="Network" detail="TOPOLOGY">
              <EmptyState mark="⌘" title="No network configured">
                Virtual nodes, links, and communication events will appear here.
              </EmptyState>
            </Panel>
          </div>
        </div>
        <div className="bottom-panels">
          <Panel id="timeline" title="Timeline" detail="TRACE">
            <div className="timeline-empty">
              <span className="trace-line" aria-hidden="true" />
              <div>
                <h3>No trace recorded</h3>
                <p>
                  Scans, transitions, and events will share one simulation
                  timeline.
                </p>
              </div>
              <span className="muted-label">Awaiting runtime</span>
            </div>
          </Panel>
          <Panel id="diagnostics" title="Diagnostics" detail="APPLICATION">
            <div
              className="diagnostic"
              role="status"
              aria-live="polite"
              data-testid="core-status"
            >
              {connection.kind === "connecting" && (
                <>
                  <span className="diagnostic-code">CONNECTING</span>
                  <p>Reading native core information…</p>
                </>
              )}
              {connection.kind === "connected" && (
                <>
                  <span className="diagnostic-code connected">
                    CORE CONNECTED
                  </span>
                  <p>
                    Rust backend connected · IPC v
                    {connection.info.contractVersion}
                  </p>
                  <span className="diagnostic-detail">
                    Application {connection.info.appVersion} · Core{" "}
                    {connection.info.coreVersion} ·{" "}
                    {connection.info.buildContext.toUpperCase()}
                  </span>
                </>
              )}
              {connection.kind === "unavailable" && (
                <>
                  <span className="diagnostic-code">{connection.code}</span>
                  <p>{connection.message}</p>
                </>
              )}
            </div>
          </Panel>
        </div>
      </main>
      <footer className="statusbar">
        <span>
          <span className="status-dot" />
          {connection.kind === "connected"
            ? "Native core connected"
            : connection.kind === "connecting"
              ? "Connecting to native core"
              : "Native core unavailable"}
        </span>
        <span>Simulation not initialized</span>
        <span>Sprint 0 · Foundation</span>
      </footer>
    </div>
  );
}
