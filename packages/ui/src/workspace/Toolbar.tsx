import { api } from "@plc-lab/ipc";
import { seconds } from "../format";
import { Badge, PlcModeBadge, SimulationBadge } from "../ui";
import { isFaultFinding } from "../route";
import type { Lab } from "./useLab";

/** Explanations of the two separate control groups (also the buttons' accessible descriptions). */
const HELP = {
  plcRun:
    "PLC mode switch RUN: the PLC executes its program once in every PLC cycle while simulated time passes.",
  plcStop:
    "PLC mode switch STOP: the program is not executed and the outputs go to their configured stop state (normally off). The simulated machine keeps running.",
  simRun:
    "Simulation Run: let simulated time pass. The machine moves and, with the PLC in RUN, the PLC scans every cycle. Does not change the PLC mode switch.",
  simPause:
    "Simulation Pause: freeze the simulated world. Nothing changes until Run or Step.",
  simStep:
    "Simulation Step: advance the simulated world by exactly one PLC cycle, then stay paused.",
  simStop:
    "Simulation Stop: return the machine to its initial state and power-cycle the PLC (program, hardware configuration and mode switch are kept; PLC variables restart). This is not the PLC STOP switch.",
} as const;

const SPEEDS = [25, 50, 100, 200, 500, 1000];

export function Toolbar({
  lab,
  mode,
  setMode,
  onBack,
}: {
  lab: Lab;
  mode: "basic" | "advanced";
  setMode: (m: "basic" | "advanced") => void;
  onBack: () => void;
}) {
  const { frame, ws, workspace, project } = lab;
  if (!frame || !ws || !workspace || !project) return null;
  const running = frame.simulation.state === "RUNNING";
  const busy = lab.busy !== null;
  const exercise = workspace.exercise;
  return (
    <div className="ws-toolbar" role="toolbar" aria-label="Workspace controls">
      <div className="group">
        <button
          className="btn btn-ghost btn-sm"
          onClick={onBack}
          aria-label="Back"
        >
          ←
        </button>
        <div className="col" style={{ gap: 0 }}>
          <strong style={{ fontSize: 13 }}>{project.name}</strong>
          <span className="dim" style={{ fontSize: 11 }}>
            {workspace.scene.title} · {workspace.platform.displayName}
          </span>
        </div>
      </div>
      <div className="group">
        <div className="segmented" role="group" aria-label="Interface mode">
          <button
            aria-pressed={mode === "basic"}
            onClick={() => setMode("basic")}
          >
            BASIC
          </button>
          <button
            aria-pressed={mode === "advanced"}
            onClick={() => setMode("advanced")}
          >
            ADVANCED
          </button>
        </div>
      </div>
      <div className="group">
        {mode === "basic" ? (
          <button
            className="btn btn-primary"
            disabled={busy}
            onClick={() => void lab.buildAndLoad(true)}
            title="Compile, load into the PLC, switch the PLC to RUN and start the simulation"
          >
            Build &amp; Load to PLC
          </button>
        ) : (
          <>
            <button
              className="btn"
              disabled={busy}
              onClick={() =>
                void lab
                  .act("compile", () => api.compile(lab.source))
                  .then((r) => {
                    if (r)
                      lab.log(
                        r.success ? "success" : "error",
                        r.success
                          ? `Compiled ${r.programName ?? ""} without errors.`
                          : `${r.diagnostics.length} compiler error(s).`,
                        r.diagnostics
                          .map(
                            (d) =>
                              `${d.span ? `line ${d.span.start.line}: ` : ""}${d.message}`,
                          )
                          .join("\n") || undefined,
                      );
                  })
              }
            >
              Compile
            </button>
            <button
              className="btn btn-primary"
              disabled={busy}
              onClick={() => void lab.buildAndLoad(false)}
              title="Download the program to the PLC without changing the mode switch"
            >
              Download
            </button>
          </>
        )}
      </div>
      <div
        className="group"
        aria-label="PLC"
        title="The PLC's own mode switch. It decides whether the program runs; the simulation controls decide whether time passes."
      >
        <span className="label">PLC</span>
        <PlcModeBadge mode={frame.plc.mode} />
        <div className="segmented" role="group" aria-label="PLC mode switch">
          <button
            aria-pressed={frame.plc.switch === "RUN"}
            disabled={busy}
            title={HELP.plcRun}
            onClick={() =>
              void lab.act(
                "mode",
                () => api.plcMode(ws, "RUN"),
                (f) => f,
              )
            }
          >
            RUN
          </button>
          <button
            aria-pressed={frame.plc.switch === "STOP"}
            disabled={busy}
            title={HELP.plcStop}
            onClick={() =>
              void lab.act(
                "mode",
                () => api.plcMode(ws, "STOP"),
                (f) => f,
              )
            }
          >
            STOP
          </button>
        </div>
        {(frame.plc.mode === "FAULT" || mode === "advanced") && (
          <button
            className={`btn btn-sm${frame.plc.mode === "FAULT" ? " btn-danger" : ""}`}
            disabled={busy}
            onClick={() =>
              void lab.act(
                "reset",
                () => api.plcReset(ws),
                (f) => f,
              )
            }
            title="Memory reset: clears the fault and non-retentive data"
          >
            Reset PLC
          </button>
        )}
        {frame.plc.forces > 0 && (
          <Badge kind="forced">{frame.plc.forces} forced</Badge>
        )}
      </div>
      <div
        className="group"
        aria-label="Simulation"
        title="The simulated world: whether time passes for the machine and the PLC. Separate from the PLC mode switch."
      >
        <span className="label">Simulation</span>
        <SimulationBadge state={frame.simulation.state} />
        {running ? (
          <button
            className="btn btn-sm"
            disabled={busy}
            title={HELP.simPause}
            onClick={() =>
              void lab.act(
                "sim",
                () => api.simulation(ws, "PAUSE"),
                (f) => f,
              )
            }
          >
            Pause
          </button>
        ) : (
          <button
            className="btn btn-sm"
            disabled={busy}
            title={HELP.simRun}
            onClick={() =>
              void lab.act(
                "sim",
                () => api.simulation(ws, "RUN"),
                (f) => f,
              )
            }
          >
            Run
          </button>
        )}
        <button
          className="btn btn-sm"
          disabled={busy || running}
          onClick={() =>
            void lab.act(
              "sim",
              () => api.simulation(ws, "STEP"),
              (f) => f,
            )
          }
          title={HELP.simStep}
        >
          Step
        </button>
        <button
          className="btn btn-sm"
          disabled={busy}
          onClick={() =>
            void lab.act(
              "sim",
              () => api.simulation(ws, "STOP"),
              (f) => f,
            )
          }
          title={HELP.simStop}
        >
          Stop
        </button>
        <select
          className="input"
          style={{ height: 24, width: 76 }}
          aria-label="Simulation speed"
          value={frame.simulation.speedPercent}
          onChange={(e) =>
            void lab.act(
              "speed",
              () => api.setSpeed(ws, Number(e.target.value)),
              (f) => f,
            )
          }
        >
          {SPEEDS.map((s) => (
            <option key={s} value={s}>
              {s} %
            </option>
          ))}
        </select>
        <span className="mono muted" title="Simulation time (world time)">
          {seconds(frame.worldTimeUs)}
        </span>
        <span className="mono dim" title="Completed PLC cycles">
          #{frame.cycle}
        </span>
      </div>
      {exercise && (
        <div className="group" style={{ marginLeft: "auto" }}>
          <button
            className="btn btn-primary"
            disabled={busy}
            onClick={() => void lab.check()}
            title="Run the exercise checks in an isolated simulation"
          >
            {lab.busy === "check"
              ? "Checking…"
              : isFaultFinding(exercise.kind)
                ? "Verify repair"
                : "Check solution"}
          </button>
        </div>
      )}
    </div>
  );
}
