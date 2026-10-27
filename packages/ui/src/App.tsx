import { useCallback, useState } from "react";
import { api } from "@plc-lab/ipc";
import { HomePage } from "./pages/HomePage";
import { LearnPage } from "./pages/LearnPage";
import { ExercisesPage } from "./pages/ExercisesPage";
import { FreeLabPage } from "./pages/FreeLabPage";
import { KnowledgePage } from "./pages/KnowledgePage";
import { ProgressPage } from "./pages/ProgressPage";
import { AboutPage } from "./pages/AboutPage";
import { WorkspacePage } from "./workspace/WorkspacePage";
import type { Navigate, Route } from "./route";

const NAV: { page: Route["page"]; label: string }[] = [
  { page: "home", label: "HOME" },
  { page: "learn", label: "LEARN" },
  { page: "practice", label: "PRACTICE" },
  { page: "troubleshoot", label: "TROUBLESHOOT" },
  { page: "freelab", label: "FREE LAB" },
  { page: "knowledge", label: "KNOWLEDGE" },
  { page: "progress", label: "PROGRESS" },
];

function initial(page: Route["page"]): Route {
  switch (page) {
    case "learn":
      return { page, lesson: null };
    case "knowledge":
      return { page, article: null, query: "" };
    case "workspace":
      return { page: "home" };
    default:
      return { page } as Route;
  }
}

export function App() {
  const [route, setRoute] = useState<Route>({ page: "home" });
  const navigate: Navigate = useCallback((next) => setRoute(next), []);
  const active = route.page === "workspace" ? route.section : route.page;
  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            PL
          </span>
          PLC Lab
        </div>
        <nav className="nav" aria-label="Main">
          {NAV.map((n) => (
            <button
              key={n.page}
              aria-current={active === n.page ? "page" : undefined}
              onClick={() => navigate(initial(n.page))}
            >
              {n.label}
            </button>
          ))}
        </nav>
        <div className="topbar-right">
          {!api.available() && (
            <span className="badge warning">Preview without desktop core</span>
          )}
          <button
            className="btn btn-ghost btn-sm"
            onClick={() => navigate({ page: "about" })}
          >
            About
          </button>
        </div>
      </header>
      <main className="page" id="main">
        {route.page === "home" && <HomePage navigate={navigate} />}
        {route.page === "learn" && (
          <LearnPage lessonId={route.lesson} navigate={navigate} />
        )}
        {route.page === "practice" && (
          <ExercisesPage kind="practice" navigate={navigate} />
        )}
        {route.page === "troubleshoot" && (
          <ExercisesPage kind="troubleshooting" navigate={navigate} />
        )}
        {route.page === "freelab" && <FreeLabPage navigate={navigate} />}
        {route.page === "knowledge" && (
          <KnowledgePage
            articleId={route.article}
            initialQuery={route.query}
            navigate={navigate}
          />
        )}
        {route.page === "progress" && <ProgressPage navigate={navigate} />}
        {route.page === "about" && <AboutPage />}
        {route.page === "workspace" && (
          <WorkspacePage
            key={route.projectId}
            projectId={route.projectId}
            navigate={navigate}
            back={route.section}
          />
        )}
      </main>
    </div>
  );
}
