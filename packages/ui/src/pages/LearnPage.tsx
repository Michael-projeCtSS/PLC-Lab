import { useState } from "react";
import { api, type LessonDto, type QuestionDto } from "@plc-lab/ipc";
import { Markdown } from "../Markdown";
import { Badge, Empty, ErrorNotice } from "../ui";
import { useLoad } from "../useLoad";
import { openExercise, type Navigate } from "../route";

export function LearnPage({
  lessonId,
  navigate,
}: {
  lessonId: string | null;
  navigate: Navigate;
}) {
  const library = useLoad(() => api.library(), []);
  const lessons = library.data?.lessons ?? [];
  const current =
    lessonId ?? lessons.find((l) => !l.completed)?.id ?? lessons[0]?.id ?? null;
  return (
    <div className="page-inner">
      <div className="page-header">
        <div>
          <h1>Learn</h1>
          <p>
            A complete path from zero: each lesson explains one idea, shows it,
            lets you try it and links to a practical exercise.
          </p>
        </div>
      </div>
      <ErrorNotice error={library.error} />
      {!library.loading && lessons.length === 0 && (
        <Empty>
          No lessons are installed. Check the knowledge pack under About.
        </Empty>
      )}
      <div className="two-col">
        <nav className="sidebar-list" aria-label="Lessons">
          {lessons.map((l) => (
            <button
              key={l.id}
              aria-current={l.id === current}
              onClick={() => navigate({ page: "learn", lesson: l.id })}
            >
              <span className="row">
                <span className={`lesson-number${l.completed ? " done" : ""}`}>
                  {l.completed ? "✓" : l.number}
                </span>
                <span className="grow">{l.title}</span>
              </span>
            </button>
          ))}
        </nav>
        {current && (
          <LessonView
            key={current}
            id={current}
            navigate={navigate}
            onCompleted={library.reload}
          />
        )}
      </div>
    </div>
  );
}

function Question({
  q,
  onAnswer,
}: {
  q: QuestionDto;
  onAnswer?: (correct: boolean) => void;
}) {
  const [chosen, setChosen] = useState<number | null>(null);
  return (
    <div className="callout">
      <p>
        <strong>{q.question}</strong>
      </p>
      {q.options.map((option, i) => (
        <button
          key={i}
          className={`quiz-option${chosen !== null && i === q.answer ? " correct" : ""}${chosen === i && i !== q.answer ? " wrong" : ""}`}
          disabled={chosen !== null}
          onClick={() => {
            setChosen(i);
            onAnswer?.(i === q.answer);
          }}
        >
          {option}
        </button>
      ))}
      {chosen !== null && (
        <p className="muted" style={{ marginTop: 8 }}>
          {chosen === q.answer ? "Correct. " : "Not quite. "}
          {q.explanation}
        </p>
      )}
    </div>
  );
}

const SECTION_LABEL: Record<string, string> = {
  concept: "Concept",
  visual: "Picture it",
  example: "Example",
  simulation: "Try it in the simulator",
  task: "Practical exercise",
  mistake: "Common mistake",
  challenge: "Challenge",
  summary: "Summary",
};

function LessonView({
  id,
  navigate,
  onCompleted,
}: {
  id: string;
  navigate: Navigate;
  onCompleted: () => void;
}) {
  const lesson = useLoad<LessonDto>(() => api.lesson(id), [id]);
  const [answers, setAnswers] = useState<Record<number, boolean>>({});
  const [saved, setSaved] = useState<string | null>(null);
  const [error, setError] = useState<unknown>(null);
  const l = lesson.data;
  if (lesson.error) return <ErrorNotice error={lesson.error} />;
  if (!l) return <Empty>Loading lesson…</Empty>;
  const answered = Object.keys(answers).length;
  const correct = Object.values(answers).filter(Boolean).length;
  const openScene = async (scene: string) => {
    try {
      const project = await api.createProject({
        name: `${l.title} – sandbox`,
        scene,
      });
      navigate({ page: "workspace", projectId: project.id, section: "learn" });
    } catch (e) {
      setError(e);
    }
  };
  return (
    <article className="prose" aria-label={l.title}>
      <div className="row muted" style={{ fontSize: 12 }}>
        Lesson {l.number} · {l.minutes} min · <Badge>{l.level}</Badge>
      </div>
      <h2 style={{ fontSize: 22, marginTop: 6 }}>{l.title}</h2>
      <p>{l.summary}</p>
      {l.objectives.length > 0 && (
        <div className="callout task">
          <strong>You will learn to</strong>
          <ul>
            {l.objectives.map((o) => (
              <li key={o}>{o}</li>
            ))}
          </ul>
        </div>
      )}
      <ErrorNotice error={error} onDismiss={() => setError(null)} />
      {l.sections.map((s, i) => (
        <section key={i}>
          <h4 style={{ marginTop: 22 }}>
            {SECTION_LABEL[s.sectionType] ?? s.sectionType}
          </h4>
          <h3 style={{ marginTop: 4 }}>{s.heading}</h3>
          <div
            className={
              s.sectionType === "mistake" ? "callout mistake" : undefined
            }
          >
            <Markdown text={s.body} />
          </div>
          {s.diagram && <div className="diagram">{s.diagram}</div>}
          {s.code && (
            <pre>
              <code>{s.code}</code>
            </pre>
          )}
          {s.question && <Question q={s.question} />}
          {s.scene && (
            <button
              className="btn"
              onClick={() => void openScene(s.scene ?? "")}
            >
              Open this machine in a sandbox
            </button>
          )}
          {s.exercise && (
            <button
              className="btn btn-primary"
              onClick={() =>
                void openExercise(s.exercise ?? "", navigate, "learn").catch(
                  setError,
                )
              }
            >
              Start the exercise
            </button>
          )}
        </section>
      ))}
      {l.checkpoint.length > 0 && (
        <section>
          <h4 style={{ marginTop: 28 }}>Checkpoint</h4>
          {l.checkpoint.map((q, i) => (
            <Question
              key={i}
              q={q}
              onAnswer={(ok) => setAnswers((a) => ({ ...a, [i]: ok }))}
            />
          ))}
        </section>
      )}
      <div className="row" style={{ marginTop: 16 }}>
        <button
          className="btn btn-primary"
          disabled={answered < l.checkpoint.length}
          onClick={() =>
            api.completeLesson(l.id, correct, l.checkpoint.length).then(() => {
              setSaved(
                `Lesson completed with ${correct} of ${l.checkpoint.length} checkpoint answers correct.`,
              );
              onCompleted();
            }, setError)
          }
        >
          Complete lesson
        </button>
        {answered < l.checkpoint.length && (
          <span className="muted">
            Answer the checkpoint questions to complete the lesson.
          </span>
        )}
      </div>
      {saved && (
        <div className="notice success" style={{ marginTop: 8 }}>
          {saved}
        </div>
      )}
      {l.knowledgeRefs.length > 0 && (
        <section>
          <h4 style={{ marginTop: 28 }}>References</h4>
          <ul>
            {l.knowledgeRefs.map((r) => (
              <li key={r.id}>
                <button
                  className="btn btn-ghost btn-sm"
                  onClick={() =>
                    navigate({ page: "knowledge", article: r.id, query: "" })
                  }
                >
                  {r.title}
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
    </article>
  );
}
