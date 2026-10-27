import type { CenterTab } from "./WorkspacePage";
import type { Lab } from "./useLab";

export type Selection =
  | { kind: "component"; id: string }
  | { kind: "module"; rack: number; slot: number }
  | { kind: "device"; id: string }
  | { kind: "signal"; id: number }
  | null;

export function ProjectTree({
  lab,
  tab,
  setTab,
  selection,
  select,
}: {
  lab: Lab;
  tab: CenterTab;
  setTab: (t: CenterTab) => void;
  selection: Selection;
  select: (s: Selection) => void;
}) {
  const { workspace, project } = lab;
  if (!workspace || !project) return null;
  const station = project.station;
  return (
    <div className="tree">
      <div className="tree-title">Program</div>
      <button
        aria-current={tab === "program"}
        onClick={() => setTab("program")}
      >
        ▤ {project.programs[0]?.name ?? "Main"}{" "}
        <span className="dim">(ST)</span>
      </button>
      <div className="tree-title" style={{ marginTop: 8 }}>
        Hardware
      </div>
      <button
        aria-current={tab === "hardware" && selection === null}
        onClick={() => {
          select(null);
          setTab("hardware");
        }}
      >
        ▣ {station.name} <span className="dim">CPU</span>
      </button>
      {station.modules.map((m) => (
        <button
          key={`${m.position.rack}-${m.position.slot}`}
          className="indent"
          aria-current={
            selection?.kind === "module" &&
            selection.rack === m.position.rack &&
            selection.slot === m.position.slot
          }
          onClick={() =>
            select({
              kind: "module",
              rack: m.position.rack,
              slot: m.position.slot,
            })
          }
        >
          <span className="dim">{m.position.slot}</span> {m.module}
        </button>
      ))}
      <div className="tree-title" style={{ marginTop: 8 }}>
        Process
      </div>
      <button
        aria-current={tab === "process" && selection === null}
        onClick={() => {
          select(null);
          setTab("process");
        }}
      >
        ◧ {workspace.scene.title}
      </button>
      {workspace.scene.components.map((c) => (
        <button
          key={c.id}
          className="indent"
          aria-current={
            selection?.kind === "component" && selection.id === c.id
          }
          onClick={() => select({ kind: "component", id: c.id })}
        >
          {c.label}
        </button>
      ))}
      {workspace.scene.hasNetwork && (
        <>
          <div className="tree-title" style={{ marginTop: 8 }}>
            Network
          </div>
          <button
            aria-current={tab === "network"}
            onClick={() => setTab("network")}
          >
            ⇄ Ethernet{" "}
            {station.network.interface
              ? station.network.interface.ip
              : "(not configured)"}
          </button>
          {station.network.modbusTcp.map((s) => (
            <button
              key={s.id}
              className="indent"
              onClick={() => setTab("network")}
            >
              Modbus TCP → {s.server}
            </button>
          ))}
        </>
      )}
      <div className="tree-title" style={{ marginTop: 8 }}>
        Online
      </div>
      <button
        aria-current={tab === "diagnostics"}
        onClick={() => setTab("diagnostics")}
      >
        ⚑ Diagnostics
      </button>
    </div>
  );
}
