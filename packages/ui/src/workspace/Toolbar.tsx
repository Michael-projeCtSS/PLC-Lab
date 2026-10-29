import { useState } from "react";
import { useInterface } from "../locale";
import { api } from "@plc-lab/ipc";
import { seconds } from "../format";
import { Badge, PlcModeBadge, SimulationBadge } from "../ui";
import { isFaultFinding } from "../route";
import type { Lab } from "./useLab";

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
  const { t, preferences } = useInterface();
  const [feedback, setFeedback] = useState<"loaded" | "failed" | null>(null);
  const HELP = {
    plcRun: t("help.plcRun"),
    plcStop: t("help.plcStop"),
    simRun: t("help.simRun"),
    simPause: t("help.simPause"),
    simStep: t("help.simStep"),
    simStop: t("help.simStop"),
  };
  const { frame, ws, workspace, project } = lab;
  if (!frame || !ws || !workspace || !project) return null;
  const running = frame.simulation.state === "RUNNING";
  const busy = lab.busy !== null;
  const exercise = workspace.exercise;
  return (
    <div className="ws-toolbar" role="toolbar" aria-label={t("ws.controls")}>
      <div className="group">
        <button
          className="btn btn-ghost btn-sm"
          onClick={onBack}
          aria-label={t("common.back")}
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
        <div className="segmented" role="group" aria-label={t("ws.mode")}>
          <button
            aria-pressed={mode === "basic"}
            onClick={() => setMode("basic")}
          >
            {t("ws.basic")}
          </button>
          <button
            aria-pressed={mode === "advanced"}
            onClick={() => setMode("advanced")}
          >
            {t("ws.advanced")}
          </button>
        </div>
      </div>
      <div className="group">
        {mode === "basic" ? (
          <button
            className="btn btn-primary"
            disabled={busy}
            onClick={() => {
              setFeedback(null);
              void lab
                .buildAndLoad(true)
                .then(
                  (r) =>
                    r &&
                    setFeedback(r.outcome === "LOADED" ? "loaded" : "failed"),
                );
            }}
            title={
              preferences.explanations ? t("control.buildHelp") : undefined
            }
            aria-description={t("control.buildHelp")}
          >
            {t("control.build")}
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
              {t("control.compile")}
            </button>
            <button
              className="btn btn-primary"
              disabled={busy}
              onClick={() => void lab.buildAndLoad(false)}
              title={t("control.downloadHelp")}
            >
              {t("control.download")}
            </button>
          </>
        )}
      </div>
      {feedback && (
        <div
          className={`build-feedback ${feedback === "failed" ? "error" : "success"}`}
          role={feedback === "failed" ? "alert" : "status"}
        >
          {t(feedback === "failed" ? "control.failed" : "control.loaded")}
        </div>
      )}
      <div
        className="group control-group"
        role="group"
        aria-label="PLC"
        title={t("help.plcGroup")}
      >
        <span className="label">PLC</span>
        <PlcModeBadge mode={frame.plc.mode} />
        <div
          className="segmented"
          role="group"
          aria-label={t("control.plcGroup")}
        >
          <button
            aria-pressed={frame.plc.switch === "RUN"}
            disabled={busy}
            title={preferences.explanations ? HELP.plcRun : undefined}
            aria-description={HELP.plcRun}
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
            title={preferences.explanations ? HELP.plcStop : undefined}
            aria-description={HELP.plcStop}
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
            title={t("help.reset")}
          >
            {t("control.reset")}
          </button>
        )}
        {frame.plc.forces > 0 && (
          <Badge kind="forced">{frame.plc.forces} forced</Badge>
        )}
      </div>
      <div
        className="group control-group"
        aria-label={t("control.simulation")}
        role="group"
        title={t("help.simGroup")}
      >
        <span className="label">{t("control.simulation")}</span>
        <SimulationBadge state={frame.simulation.state} />
        {running ? (
          <button
            className="btn btn-sm"
            disabled={busy}
            title={preferences.explanations ? HELP.simPause : undefined}
            aria-description={HELP.simPause}
            onClick={() =>
              void lab.act(
                "sim",
                () => api.simulation(ws, "PAUSE"),
                (f) => f,
              )
            }
          >
            {t("control.pause")}
          </button>
        ) : (
          <button
            className="btn btn-sm"
            disabled={busy}
            title={preferences.explanations ? HELP.simRun : undefined}
            aria-description={HELP.simRun}
            onClick={() =>
              void lab.act(
                "sim",
                () => api.simulation(ws, "RUN"),
                (f) => f,
              )
            }
          >
            {t("control.run")}
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
          title={preferences.explanations ? HELP.simStep : undefined}
          aria-description={HELP.simStep}
        >
          {t("control.step")}
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
          title={preferences.explanations ? HELP.simStop : undefined}
          aria-description={HELP.simStop}
        >
          {t("control.stop")}
        </button>
        <select
          className="input"
          style={{ height: 24, width: 76 }}
          aria-label={t("control.speed")}
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
        <span className="mono muted" title={t("control.time")}>
          {seconds(frame.worldTimeUs)}
        </span>
        <span className="mono dim" title={t("control.cycles")}>
          #{frame.cycle}
        </span>
      </div>
      {exercise && (
        <div className="group" style={{ marginLeft: "auto" }}>
          <button
            className="btn btn-primary"
            disabled={busy}
            onClick={() => void lab.check()}
            title={t("help.check")}
          >
            {lab.busy === "check"
              ? t("control.checking")
              : isFaultFinding(exercise.kind)
                ? t("control.verify")
                : t("control.check")}
          </button>
        </div>
      )}
    </div>
  );
}
