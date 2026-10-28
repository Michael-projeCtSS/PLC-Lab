import { useState } from "react";
import { api, type ExerciseSummaryDto } from "@plc-lab/ipc";
import { Badge, Empty, ErrorNotice } from "../ui";
import { useLoad } from "../useLoad";
import {
  openExercise,
  sectionOf,
  type ExerciseKind,
  type Navigate,
} from "../route";

const LEVELS = ["beginner", "intermediate", "advanced"] as const;

const INTRO: Record<ExerciseKind, { title: string; text: string }> = {
  practice: {
    title: "Practice",
    text: "Write a PLC program for a real machine. Your program is checked by running it against scripted scenarios in an isolated simulation.",
  },
  commissioning: {
    title: "Commissioning labs",
    text: "A newly installed machine does not work yet. Work through the commissioning checklist: power, hardware, network, inputs, outputs, drives and process. Test every device, find the installation fault, repair it and prove the machine works.",
  },
  troubleshooting: {
    title: "Troubleshooting labs",
    text: "Something is wrong with a working machine. Reproduce the symptom, collect evidence, name the root cause, repair it and prove the repair.",
  },
};

export function ExercisesPage({
  kind,
  navigate,
}: {
  kind: ExerciseKind;
  navigate: Navigate;
}) {
  const library = useLoad(() => api.library(), []);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const items = (library.data?.exercises ?? []).filter((e) => e.kind === kind);
  const section = sectionOf(kind);
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
          <h1>{INTRO[kind].title}</h1>
          <p>{INTRO[kind].text}</p>
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
