import { useState } from "react";
import {
  api,
  type DiagnosisResultDto,
  type ExerciseDto,
  type GradeReportDto,
  type OptionDto,
} from "@plc-lab/ipc";
import { formatValue, humanize, quantityUnit, valueClass } from "../format";
import { Badge, Tabs } from "../ui";
import { isFaultFinding, type Navigate } from "../route";
import { CauseEffect } from "./CauseEffect";
import type { Selection } from "./ProjectTree";
import type { Lab } from "./useLab";

type Tab = "task" | "cause" | "properties";

export function Inspector({
  lab,
  selection,
  navigate,
}: {
  lab: Lab;
  selection: Selection;
  navigate: Navigate;
}) {
  const exercise = lab.workspace?.exercise ?? null;
  const [tab, setTab] = useState<Tab>(exercise ? "task" : "cause");
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <Tabs<Tab>
        label="Inspector"
        active={tab}
        onSelect={setTab}
        tabs={[
          ...(exercise
            ? [
                {
                  id: "task" as const,
                  label: isFaultFinding(exercise.kind)
                    ? "Investigation"
                    : "Task",
                },
              ]
            : []),
          { id: "cause", label: "Cause & effect" },
          { id: "properties", label: "Properties" },
        ]}
      />
      <div style={{ flex: 1, overflow: "auto" }}>
        {tab === "task" &&
          exercise &&
          (isFaultFinding(exercise.kind) ? (
            <Investigation lab={lab} exercise={exercise} navigate={navigate} />
          ) : (
            <TaskBrief
              exercise={exercise}
              report={lab.report}
              navigate={navigate}
            />
          ))}
        {tab === "cause" && (
          <div className="inspector-section">
            <CauseEffect
              ce={lab.frame?.causeEffect ?? null}
              signals={lab.signals}
            />
          </div>
        )}
        {tab === "properties" && <Properties lab={lab} selection={selection} />}
      </div>
    </div>
  );
}

function Report({
  report,
  navigate,
}: {
  report: GradeReportDto | null;
  navigate: Navigate;
}) {
  if (!report) return null;
  return (
    <div className="inspector-section" aria-label="Check result">
      <h4>Result</h4>
      {report.outcome === "GRADED" ? (
        <>
          <div className="row">
            <span className="score">
              {report.score}
              <span className="muted" style={{ fontSize: 14 }}>
                {" "}
                / {report.maxScore}
              </span>
            </span>
            <span className="spacer" />
            {report.passed ? (
              <Badge kind="active">Passed</Badge>
            ) : (
              <Badge kind="warning">Not yet</Badge>
            )}
          </div>
          {report.criteria.map((c) => (
            <div key={c.id} className="criterion">
              <span
                className={c.passed ? "dot on" : "dot red"}
                style={{ marginTop: 5 }}
              />
              <div className="grow">
                <div className="row">
                  <strong className="grow">{c.title}</strong>
                  <span className="mono">
                    {c.earned}/{c.points}
                  </span>
                </div>
                {!c.passed && <div>{c.feedback}</div>}
                <div className="dim" style={{ whiteSpace: "normal" }}>
                  {c.detail}
                </div>
              </div>
            </div>
          ))}
          {report.debrief && (
            <div
              className="callout task"
              aria-label="Debrief"
              style={{ marginTop: 10 }}
            >
              <h4>What was wrong</h4>
              <p>{report.debrief.rootCause}</p>
              <h4>Evidence that pointed to it</h4>
              <ul style={{ paddingLeft: 18, margin: "4px 0" }}>
                {report.debrief.evidence.map((e) => (
                  <li key={e}>{e}</li>
                ))}
              </ul>
              <h4>Correct repair</h4>
              <p>{report.debrief.repair}</p>
              <h4>How to reason about it</h4>
              <p style={{ margin: 0 }}>{report.debrief.reasoning}</p>
              {report.debrief.references.length > 0 && (
                <>
                  <h4 style={{ marginTop: 8 }}>Read more about this fault</h4>
                  {report.debrief.references.map((r) => (
                    <button
                      key={r.id}
                      className="btn btn-ghost btn-sm"
                      style={{ justifyContent: "flex-start", width: "100%" }}
                      onClick={() =>
                        navigate({
                          page: "knowledge",
                          article: r.id,
                          query: "",
                        })
                      }
                    >
                      {r.title}
                    </button>
                  ))}
                </>
              )}
            </div>
          )}
        </>
      ) : (
        <div className="notice error">
          {report.outcome === "COMPILE_FAILED" &&
            "The program does not compile. Fix the errors shown in the editor."}
          {report.outcome === "INTERFACE_MISMATCH" &&
            "The program does not declare the required tags:"}
          {report.outcome === "BINDING_FAILED" &&
            "The program does not match the I/O configuration:"}
          <ul>
            {report.problems.map((p) => (
              <li key={p}>{p}</li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

function References({
  exercise,
  navigate,
  heading = "Knowledge",
}: {
  exercise: ExerciseDto;
  navigate: Navigate;
  heading?: string;
}) {
  if (exercise.knowledgeRefs.length === 0) return null;
  return (
    <div className="inspector-section">
      <h4>{heading}</h4>
      {exercise.knowledgeRefs.map((r) => (
        <button
          key={r.id}
          className="btn btn-ghost btn-sm"
          style={{ justifyContent: "flex-start", width: "100%" }}
          onClick={() =>
            navigate({ page: "knowledge", article: r.id, query: "" })
          }
        >
          {r.title}
        </button>
      ))}
    </div>
  );
}

function TaskBrief({
  exercise,
  report,
  navigate,
}: {
  exercise: ExerciseDto;
  report: GradeReportDto | null;
  navigate: Navigate;
}) {
  const [hints, setHints] = useState(0);
  return (
    <>
      <div className="inspector-section">
        <div className="row">
          <Badge>{exercise.difficulty}</Badge>
          <span className="muted">{exercise.minutes} min</span>
        </div>
        <h3 style={{ margin: "6px 0" }}>{exercise.title}</h3>
        <p className="muted">{exercise.summary}</p>
        <h4>What the program must do</h4>
        <ol style={{ paddingLeft: 18, margin: "6px 0" }}>
          {exercise.instructions.map((i) => (
            <li key={i} style={{ marginBottom: 4 }}>
              {i}
            </li>
          ))}
        </ol>
      </div>
      <div className="inspector-section">
        <h4>Required interface</h4>
        <table className="table">
          <tbody>
            {exercise.requiredInterface.map((t) => (
              <tr key={t.tag}>
                <td className="mono">{t.tag}</td>
                <td>{t.dataType}</td>
                <td className="muted">{t.direction}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="inspector-section">
        <h4>Checks</h4>
        {exercise.criteria.map((c) => (
          <div key={c.id} className="row" style={{ fontSize: 12 }}>
            <span className="grow">{c.title}</span>
            <span className="mono dim">{c.points} pt</span>
          </div>
        ))}
      </div>
      {exercise.hints.length > 0 && (
        <div className="inspector-section">
          <h4>Hints</h4>
          {exercise.hints.slice(0, hints).map((h) => (
            <p key={h} className="callout" style={{ margin: "4px 0" }}>
              {h}
            </p>
          ))}
          {hints < exercise.hints.length && (
            <button
              className="btn btn-sm"
              onClick={() => setHints((n) => n + 1)}
            >
              Show hint {hints + 1} of {exercise.hints.length}
            </button>
          )}
        </div>
      )}
      <Report report={report} navigate={navigate} />
      <References exercise={exercise} navigate={navigate} />
    </>
  );
}

function Investigation({
  lab,
  exercise,
  navigate,
}: {
  lab: Lab;
  exercise: ExerciseDto;
  navigate: Navigate;
}) {
  const brief = exercise.troubleshooting;
  const [choice, setChoice] = useState("");
  const [result, setResult] = useState<DiagnosisResultDto | null>(null);
  const [repairs, setRepairs] = useState<string[]>([]);
  // The backend sends the repair options only once a correct diagnosis is on record.
  const [unlocked, setUnlocked] = useState<OptionDto[] | null>(null);
  const ws = lab.ws;
  if (!brief || !ws) return null;
  const diagnosed = brief.diagnosed || unlocked !== null;
  const repairOptions = unlocked ?? brief.repairs;
  return (
    <>
      <div className="inspector-section">
        <h4>Fault report</h4>
        <div className="callout mistake" style={{ margin: "6px 0" }}>
          <p style={{ margin: 0 }}>“{brief.symptom}”</p>
          <div className="dim" style={{ marginTop: 4 }}>
            — {brief.reportedBy}
          </div>
        </div>
        <h4 style={{ marginTop: 10 }}>Method</h4>
        <ol style={{ paddingLeft: 18, margin: "6px 0" }}>
          {exercise.instructions.map((i) => (
            <li key={i} style={{ marginBottom: 4 }}>
              {i}
            </li>
          ))}
        </ol>
      </div>
      <div className="inspector-section">
        <h4>1 · Diagnosis</h4>
        <p className="muted">
          Name the root cause only when your evidence proves it. Every guess is
          recorded.
        </p>
        <select
          className="input"
          style={{ width: "100%" }}
          value={choice}
          onChange={(e) => setChoice(e.target.value)}
          aria-label="Root cause"
        >
          <option value="">Select the root cause…</option>
          {brief.diagnosisOptions.map((o) => (
            <option key={o.id} value={o.id}>
              {o.label}
            </option>
          ))}
        </select>
        <button
          className="btn btn-sm"
          style={{ marginTop: 6 }}
          disabled={!choice}
          onClick={() =>
            void lab
              .act("diagnose", () => api.diagnose(ws, choice))
              .then((r) => {
                if (!r) return;
                setResult(r);
                if (r.correct) setUnlocked(r.repairs);
              })
          }
        >
          Submit diagnosis
        </button>
        {result && (
          <div
            className={`notice ${result.correct ? "success" : "warning"}`}
            style={{ marginTop: 6 }}
          >
            {result.message}{" "}
            <span className="dim">(attempt {result.attempts})</span>
          </div>
        )}
      </div>
      <div className="inspector-section" aria-label="Repair">
        <h4>2 · Repair</h4>
        <p className="muted">
          {!diagnosed
            ? "Submit a diagnosis that explains the evidence to unlock the repair step."
            : repairOptions.length > 0
              ? "A repair changes the installation. Only the right one fixes the machine."
              : "No physical part is broken here. Correct the configuration in the Hardware or Network tab (Advanced mode) and download it."}
        </p>
        {diagnosed &&
          repairOptions.map((r) => (
            <button
              key={r.id}
              className="btn btn-sm"
              style={{
                width: "100%",
                justifyContent: "flex-start",
                marginBottom: 4,
              }}
              disabled={repairs.includes(r.id)}
              onClick={() =>
                void lab
                  .act(
                    "repair",
                    () => api.repair(ws, r.id),
                    (x) => x.frame,
                  )
                  .then((x) => {
                    if (x) {
                      setRepairs((list) => [...list, r.id]);
                      lab.log("info", x.message);
                    }
                  })
              }
            >
              {repairs.includes(r.id) ? "✓ " : ""}
              {r.label}
            </button>
          ))}
      </div>
      <div className="inspector-section">
        <h4>3 · Verify</h4>
        <p className="muted">
          Test the machine yourself, then use “Verify repair” in the toolbar.
          Verification runs the original acceptance test.
        </p>
      </div>
      <Report report={lab.report} navigate={navigate} />
      <References
        exercise={exercise}
        navigate={navigate}
        heading="Helpful concepts"
      />
    </>
  );
}

function Properties({ lab, selection }: { lab: Lab; selection: Selection }) {
  const { workspace, frame } = lab;
  if (!workspace || !frame) return null;
  if (selection?.kind === "component") {
    const c = workspace.scene.components.find((x) => x.id === selection.id);
    const state = frame.components.find((x) => x.id === selection.id);
    if (!c) return null;
    return (
      <div className="inspector-section">
        <h4>{c.label}</h4>
        <dl className="kv">
          <dt>Type</dt>
          <dd>{humanize(c.kind)}</dd>
          {c.config.map((k) => (
            <Fragmentish key={k.key} k={humanize(k.key)} v={k.value} />
          ))}
        </dl>
        <h4 style={{ marginTop: 12 }}>Live state</h4>
        <dl className="kv">
          {state?.quantities.map((q) => (
            <Fragmentish
              key={q.name}
              k={humanize(q.name)}
              v={formatValue(q.value, quantityUnit(q.name))}
              cls={valueClass(q.value)}
            />
          ))}
        </dl>
      </div>
    );
  }
  return (
    <div className="inspector-section">
      <h4>Scene</h4>
      <p className="muted">{workspace.scene.summary}</p>
      <h4 style={{ marginTop: 12 }}>Recommended PLC interface</h4>
      <table className="table">
        <tbody>
          {workspace.scene.interface.map((i) => (
            <tr key={i.tag} title={i.description}>
              <td className="mono">{i.tag}</td>
              <td>{i.dataType}</td>
              <td className="muted" style={{ whiteSpace: "normal" }}>
                {i.description}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <h4 style={{ marginTop: 12 }}>Platform profile</h4>
      <dl className="kv">
        <dt>Profile</dt>
        <dd>{workspace.platform.displayName}</dd>
        <dt>Addressing</dt>
        <dd style={{ whiteSpace: "normal" }}>
          {workspace.platform.addressing}
        </dd>
        <dt>Analog values</dt>
        <dd style={{ whiteSpace: "normal" }}>{workspace.platform.analog}</dd>
        <dt>Fidelity</dt>
        <dd>
          {workspace.platform.fidelity.level.replaceAll("_", " ").toLowerCase()}
        </dd>
      </dl>
    </div>
  );
}

function Fragmentish({ k, v, cls }: { k: string; v: string; cls?: string }) {
  return (
    <>
      <dt>{k}</dt>
      <dd className={cls}>{v}</dd>
    </>
  );
}
