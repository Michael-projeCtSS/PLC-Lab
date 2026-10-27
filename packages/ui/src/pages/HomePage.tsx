import { api, type LibraryDto, type ProjectListDto } from "@plc-lab/ipc";
import { ErrorNotice } from "../ui";
import { useLoad } from "../useLoad";
import type { Navigate } from "../route";

const AREAS = [
  {
    page: "learn",
    title: "Learn from zero",
    text: "Twenty short lessons from “What is a PLC?” to commissioning and fault finding. No prior knowledge needed.",
  },
  {
    page: "practice",
    title: "Practice",
    text: "Write Structured Text for real machines, load it into the simulated PLC and get behavior-based feedback.",
  },
  {
    page: "troubleshoot",
    title: "Troubleshooting labs",
    text: "A machine has a fault. Collect evidence with LEDs, meters and diagnostics, find the root cause and repair it.",
  },
  {
    page: "freelab",
    title: "Free lab",
    text: "An engineering workspace with a PLC, I/O hardware, a process and a network to experiment freely.",
  },
  {
    page: "knowledge",
    title: "Knowledge base",
    text: "Fundamentals, PLC platforms from eight vendors, industrial protocols and troubleshooting guides with sources.",
  },
  {
    page: "progress",
    title: "Progress",
    text: "Completed lessons, exercise scores and troubleshooting results.",
  },
] as const;

export function HomePage({ navigate }: { navigate: Navigate }) {
  const data = useLoad(
    async () => ({
      library: await api.library(),
      projects: await api.projects(),
    }),
    [],
  );
  const library: LibraryDto | undefined = data.data?.library;
  const projects: ProjectListDto | undefined = data.data?.projects;
  const next = library?.lessons.find((l) => !l.completed) ?? null;
  const done = library?.lessons.filter((l) => l.completed).length ?? 0;
  const total = library?.lessons.length ?? 0;
  const recent = projects?.projects[0] ?? null;
  return (
    <div className="page-inner">
      <ErrorNotice error={data.error} />
      <section className="hero">
        <div className="hero-main">
          <h1>Learn industrial automation by doing</h1>
          <p>
            Program a PLC, watch the machine respond, and learn to find faults
            the way commissioning engineers do. Every exercise runs on a
            deterministic simulation of the PLC, its I/O hardware and the
            process.
          </p>
          <div className="row" style={{ marginTop: 16 }}>
            {next ? (
              <button
                className="btn btn-primary btn-lg"
                onClick={() => navigate({ page: "learn", lesson: next.id })}
              >
                {done === 0
                  ? "Start lesson 1"
                  : `Continue: lesson ${next.number}`}
              </button>
            ) : (
              <button
                className="btn btn-primary btn-lg"
                onClick={() => navigate({ page: "practice" })}
              >
                Practice
              </button>
            )}
            <button
              className="btn btn-lg"
              onClick={() => navigate({ page: "freelab" })}
            >
              Open free lab
            </button>
          </div>
        </div>
        <div className="panel">
          <div className="panel-head">
            <h3>Continue</h3>
          </div>
          <div className="panel-body col">
            {next && (
              <button
                className="list-row"
                style={{ border: "1px solid var(--line)", borderRadius: 4 }}
                onClick={() => navigate({ page: "learn", lesson: next.id })}
              >
                <span className="lesson-number">{next.number}</span>
                <span className="grow">
                  <strong>{next.title}</strong>
                  <br />
                  <span className="muted">{next.minutes} min lesson</span>
                </span>
              </button>
            )}
            {recent ? (
              <button
                className="list-row"
                style={{ border: "1px solid var(--line)", borderRadius: 4 }}
                onClick={() =>
                  navigate({
                    page: "workspace",
                    projectId: recent.id,
                    section: recent.kind === "freeLab" ? "freelab" : "practice",
                  })
                }
              >
                <span className="grow">
                  <strong>{recent.name}</strong>
                  <br />
                  <span className="muted">
                    Last project, saved{" "}
                    {recent.updatedAt.replace("T", " ").replace("Z", " UTC")}
                  </span>
                </span>
              </button>
            ) : (
              <p className="muted">
                Your projects appear here once you start an exercise or a free
                lab.
              </p>
            )}
            {total > 0 && (
              <div>
                <div
                  className="row muted"
                  style={{ fontSize: 12, marginBottom: 4 }}
                >
                  Curriculum <span className="spacer" /> {done} / {total}{" "}
                  lessons
                </div>
                <div className="progress-bar">
                  <div style={{ width: `${(100 * done) / total}%` }} />
                </div>
              </div>
            )}
          </div>
        </div>
      </section>
      <div className="tiles">
        {AREAS.map((a) => (
          <button
            key={a.page}
            className="tile"
            onClick={() =>
              navigate(
                a.page === "learn"
                  ? { page: "learn", lesson: null }
                  : a.page === "knowledge"
                    ? { page: "knowledge", article: null, query: "" }
                    : { page: a.page },
              )
            }
          >
            <h3>{a.title}</h3>
            <p>{a.text}</p>
          </button>
        ))}
      </div>
    </div>
  );
}
