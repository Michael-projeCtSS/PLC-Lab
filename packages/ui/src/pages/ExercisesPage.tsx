import { useInterface } from "../locale";
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

const INTRO = {
  practice: {
    title: "ex.practice",
    text: "ex.practiceIntro",
  },
  commissioning: {
    title: "ex.commissioning",
    text: "ex.commissioningIntro",
  },
  troubleshooting: {
    title: "ex.troubleshooting",
    text: "ex.troubleshootingIntro",
  },
} as const;

export function ExercisesPage({
  kind,
  navigate,
}: {
  kind: ExerciseKind;
  navigate: Navigate;
}) {
  const { t } = useInterface();
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
          <h1>{t(INTRO[kind].title)}</h1>
          <p>{t(INTRO[kind].text)}</p>
        </div>
      </div>
      <ErrorNotice
        error={library.error ?? error}
        onDismiss={() => setError(null)}
      />
      {!library.loading && items.length === 0 && <Empty>{t("ex.empty")}</Empty>}
      {LEVELS.map((level) => {
        const group = items.filter((e) => e.difficulty === level);
        if (group.length === 0) return null;
        return (
          <section key={level}>
            <div className="section-title">
              <h2 style={{ textTransform: "capitalize" }}>
                {t(`ex.${level}`)}
              </h2>
              <span className="muted">
                {t("ex.count", { count: group.length })}
              </span>
            </div>
            <div className="tiles">
              {group.map((e) => (
                <button
                  key={e.id}
                  className="tile"
                  onClick={() => start(e)}
                  disabled={busy !== null}
                  aria-label={t("ex.start", { title: e.title })}
                >
                  <h3>
                    {e.title}
                    {e.passed && (
                      <Badge kind="active">{t("common.passed")}</Badge>
                    )}
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
                        ? t("common.opening")
                        : e.attempts > 0
                          ? t("common.continue")
                          : t("common.start")}
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
