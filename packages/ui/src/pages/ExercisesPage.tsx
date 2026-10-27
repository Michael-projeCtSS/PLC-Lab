import { useState } from "react";
import { api, type ExerciseSummaryDto } from "@plc-lab/ipc";
import { Badge, Empty, ErrorNotice } from "../ui";
import { useLoad } from "../useLoad";
import { openExercise, type Navigate } from "../route";

const LEVELS = ["beginner", "intermediate", "advanced"] as const;

export function ExercisesPage({
  kind,
  navigate,
}: {
  kind: "practice" | "troubleshooting";
  navigate: Navigate;
}) {
  const library = useLoad(() => api.library(), []);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const items = (library.data?.exercises ?? []).filter((e) => e.kind === kind);
  const section = kind === "practice" ? "practice" : "troubleshoot";
  const start = (e: ExerciseSummaryDto) => {
    setBusy(e.id);
    openExercise(e.id, navigate, section).catch((err: unknown) => {
      setError(err);
      setBusy(null);
    });
  };
  return (
    <div className="page-inner">
      <div className="page-header">
        <div>
          <h1>{kind === "practice" ? "Practice" : "Troubleshooting labs"}</h1>
          <p>
            {kind === "practice"
              ? "Write a PLC program for a real machine. Your program is checked by running it against scripted scenarios in an isolated simulation."
              : "Something is wrong with a working machine. Reproduce the symptom, collect evidence, name the root cause, repair it and prove the repair."}
          </p>
        </div>
      </div>
      <ErrorNotice
        error={library.error ?? error}
        onDismiss={() => setError(null)}
      />
      {!library.loading && items.length === 0 && (
        <Empty>No {kind} exercises are installed.</Empty>
      )}
      {LEVELS.map((level) => {
        const group = items.filter((e) => e.difficulty === level);
        if (group.length === 0) return null;
        return (
          <section key={level}>
            <div className="section-title">
              <h2 style={{ textTransform: "capitalize" }}>{level}</h2>
              <span className="muted">{group.length} exercises</span>
            </div>
            <div className="tiles">
              {group.map((e) => (
                <button
                  key={e.id}
                  className="tile"
                  onClick={() => start(e)}
                  disabled={busy !== null}
                  aria-label={`Start ${e.title}`}
                >
                  <h3>
                    {e.title}
                    {e.passed && <Badge kind="active">Passed</Badge>}
                  </h3>
                  <p>{e.summary}</p>
                  <div className="tile-meta">
                    <span>{e.minutes} min</span>
                    {e.attempts > 0 && (
                      <span>
                        · best {e.bestScore}/{e.maxScore} · {e.attempts} attempt
                        {e.attempts === 1 ? "" : "s"}
                      </span>
                    )}
                    <span className="spacer" />
                    <span>
                      {busy === e.id
                        ? "Opening…"
                        : e.attempts > 0
                          ? "Continue"
                          : "Start"}
                    </span>
                  </div>
                </button>
              ))}
            </div>
          </section>
        );
      })}
    </div>
  );
}
