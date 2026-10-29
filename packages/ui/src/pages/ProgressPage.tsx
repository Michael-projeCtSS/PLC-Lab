import { useInterface } from "../locale";
import { api } from "@plc-lab/ipc";
import { Badge, ErrorNotice } from "../ui";
import { useLoad } from "../useLoad";
import { openExercise, sectionOf, type Navigate } from "../route";

export function ProgressPage({ navigate }: { navigate: Navigate }) {
  const { t } = useInterface();
  const library = useLoad(() => api.library(), []);
  const l = library.data;
  const lessonsDone = l?.lessons.filter((x) => x.completed).length ?? 0;
  const practice = l?.exercises.filter((e) => e.kind === "practice") ?? [];
  const commissioning =
    l?.exercises.filter((e) => e.kind === "commissioning") ?? [];
  const troubleshooting =
    l?.exercises.filter((e) => e.kind === "troubleshooting") ?? [];
  const summary = (items: typeof practice) =>
    t("progress.passed", {
      done: items.filter((e) => e.passed).length,
      total: items.length,
    });
  return (
    <div className="page-inner">
      <div className="page-header">
        <div>
          <h1>{t("progress.title")}</h1>
          <p>{t("progress.local")}</p>
        </div>
      </div>
      <ErrorNotice error={library.error} />
      {l?.notices.map((n) => (
        <div key={n} className="notice warning" style={{ marginBottom: 12 }}>
          {n}
        </div>
      ))}
      {l && (
        <>
          <div className="tiles">
            <div className="tile" style={{ cursor: "default" }}>
              <h4>{t("learn.lessons")}</h4>
              <span className="score">
                {lessonsDone} / {l.lessons.length}
              </span>
              <div className="progress-bar">
                <div
                  style={{
                    width: `${l.lessons.length ? (100 * lessonsDone) / l.lessons.length : 0}%`,
                  }}
                />
              </div>
            </div>
            <div className="tile" style={{ cursor: "default" }}>
              <h4>{t("ex.practice")}</h4>
              <span className="score">{summary(practice)}</span>
            </div>
            <div className="tile" style={{ cursor: "default" }}>
              <h4>{t("ex.commissioning")}</h4>
              <span className="score">{summary(commissioning)}</span>
            </div>
            <div className="tile" style={{ cursor: "default" }}>
              <h4>{t("ex.troubleshooting")}</h4>
              <span className="score">{summary(troubleshooting)}</span>
            </div>
          </div>
          <div className="section-title">
            <h2>{t("learn.lessons")}</h2>
          </div>
          <table className="table panel">
            <thead>
              <tr>
                <th>#</th>
                <th>{t("progress.lesson")}</th>
                <th>{t("progress.status")}</th>
                <th>{t("learn.checkpoint")}</th>
              </tr>
            </thead>
            <tbody>
              {l.lessons.map((x) => (
                <tr
                  key={x.id}
                  onClick={() => navigate({ page: "learn", lesson: x.id })}
                  style={{ cursor: "pointer" }}
                >
                  <td>{x.number}</td>
                  <td>{x.title}</td>
                  <td>
                    {x.completed ? (
                      <Badge kind="active">{t("progress.done")}</Badge>
                    ) : (
                      <Badge>{t("progress.open")}</Badge>
                    )}
                  </td>
                  <td>
                    {x.checkpointTotal > 0
                      ? `${x.checkpointCorrect} / ${x.checkpointTotal}`
                      : "—"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <div className="section-title">
            <h2>{t("progress.exercises")}</h2>
          </div>
          <table className="table panel">
            <thead>
              <tr>
                <th>{t("lab.exercise")}</th>
                <th>{t("progress.type")}</th>
                <th>{t("progress.level")}</th>
                <th>{t("progress.attempts")}</th>
                <th>{t("progress.best")}</th>
                <th>{t("progress.status")}</th>
              </tr>
            </thead>
            <tbody>
              {l.exercises.map((e) => (
                <tr
                  key={e.id}
                  onClick={() =>
                    void openExercise(e.id, navigate, sectionOf(e.kind))
                  }
                  style={{ cursor: "pointer" }}
                >
                  <td>{e.title}</td>
                  <td>{t(`ex.${e.kind}`)}</td>
                  <td>{t(`ex.${e.difficulty}`)}</td>
                  <td>{e.attempts}</td>
                  <td>
                    {e.attempts > 0 ? `${e.bestScore} / ${e.maxScore}` : "—"}
                  </td>
                  <td>
                    {e.passed ? (
                      <Badge kind="active">{t("common.passed")}</Badge>
                    ) : e.attempts > 0 ? (
                      <Badge kind="warning">{t("progress.active")}</Badge>
                    ) : (
                      <Badge>{t("progress.new")}</Badge>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
    </div>
  );
}
