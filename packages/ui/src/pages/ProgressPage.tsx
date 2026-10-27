import { api } from "@plc-lab/ipc";
import { Badge, ErrorNotice } from "../ui";
import { useLoad } from "../useLoad";
import { openExercise, type Navigate } from "../route";

export function ProgressPage({ navigate }: { navigate: Navigate }) {
  const library = useLoad(() => api.library(), []);
  const l = library.data;
  const lessonsDone = l?.lessons.filter((x) => x.completed).length ?? 0;
  const practice = l?.exercises.filter((e) => e.kind === "practice") ?? [];
  const troubleshooting =
    l?.exercises.filter((e) => e.kind === "troubleshooting") ?? [];
  const summary = (items: typeof practice) =>
    `${items.filter((e) => e.passed).length} / ${items.length} passed`;
  return (
    <div className="page-inner">
      <div className="page-header">
        <div>
          <h1>Progress</h1>
          <p>Stored locally on this computer.</p>
        </div>
      </div>
      <ErrorNotice error={library.error} />
      {l && (
        <>
          <div className="tiles">
            <div className="tile" style={{ cursor: "default" }}>
              <h4>Lessons</h4>
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
              <h4>Practice</h4>
              <span className="score">{summary(practice)}</span>
            </div>
            <div className="tile" style={{ cursor: "default" }}>
              <h4>Troubleshooting</h4>
              <span className="score">{summary(troubleshooting)}</span>
            </div>
          </div>
          <div className="section-title">
            <h2>Lessons</h2>
          </div>
          <table className="table panel">
            <thead>
              <tr>
                <th>#</th>
                <th>Lesson</th>
                <th>Status</th>
                <th>Checkpoint</th>
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
                      <Badge kind="active">Completed</Badge>
                    ) : (
                      <Badge>Open</Badge>
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
            <h2>Exercises</h2>
          </div>
          <table className="table panel">
            <thead>
              <tr>
                <th>Exercise</th>
                <th>Type</th>
                <th>Level</th>
                <th>Attempts</th>
                <th>Best score</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              {l.exercises.map((e) => (
                <tr
                  key={e.id}
                  onClick={() =>
                    void openExercise(
                      e.id,
                      navigate,
                      e.kind === "practice" ? "practice" : "troubleshoot",
                    )
                  }
                  style={{ cursor: "pointer" }}
                >
                  <td>{e.title}</td>
                  <td>{e.kind}</td>
                  <td>{e.difficulty}</td>
                  <td>{e.attempts}</td>
                  <td>
                    {e.attempts > 0 ? `${e.bestScore} / ${e.maxScore}` : "—"}
                  </td>
                  <td>
                    {e.passed ? (
                      <Badge kind="active">Passed</Badge>
                    ) : e.attempts > 0 ? (
                      <Badge kind="warning">In progress</Badge>
                    ) : (
                      <Badge>Not started</Badge>
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
