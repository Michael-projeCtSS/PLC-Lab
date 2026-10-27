import { useEffect, useState } from "react";
import {
  api,
  type ArticleDto,
  type CatalogEntryDto,
  type SearchHitDto,
} from "@plc-lab/ipc";
import { Markdown } from "../Markdown";
import { Badge, Empty, ErrorNotice } from "../ui";
import { useLoad } from "../useLoad";
import { openExercise, type Navigate } from "../route";

const CATEGORIES = [
  { id: null, label: "All" },
  { id: "FUNDAMENTAL", label: "Fundamentals" },
  { id: "VENDOR_SPECIFIC", label: "Vendor specific" },
  { id: "TROUBLESHOOTING", label: "Troubleshooting" },
  { id: "PLC_LAB_SIMULATION", label: "PLC Lab simulation" },
] as const;

const CATEGORY_KIND: Record<string, string> = {
  FUNDAMENTAL: "auto",
  VENDOR_SPECIFIC: "manual",
  TROUBLESHOOTING: "warning",
  PLC_LAB_SIMULATION: "active",
};

export function KnowledgePage({
  articleId,
  initialQuery,
  navigate,
}: {
  articleId: string | null;
  initialQuery: string;
  navigate: Navigate;
}) {
  const [query, setQuery] = useState(initialQuery);
  const [category, setCategory] = useState<string | null>(null);
  const [hits, setHits] = useState<SearchHitDto[] | null>(null);
  const [error, setError] = useState<unknown>(null);
  const catalog = useLoad(() => api.catalog(), []);
  useEffect(() => {
    if (!query.trim()) return;
    let active = true;
    const timer = setTimeout(() => {
      api.search(query, { category }).then(
        (h) => active && setHits(h),
        (e: unknown) => active && setError(e),
      );
    }, 150);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [query, category]);
  const open = (id: string) =>
    navigate({ page: "knowledge", article: id, query });
  return (
    <div className="page-inner">
      <div className="page-header">
        <div>
          <h1>Knowledge base</h1>
          <p>
            Original explanations with provenance. Simplified behavior is marked
            as a simulation approximation; unverified details are labeled.
          </p>
        </div>
      </div>
      <div className="row wrap" style={{ marginBottom: 16 }}>
        <input
          className="input grow"
          style={{ height: 34, fontSize: 14 }}
          type="search"
          placeholder="Search: 4-20 mA, TON, PROFINET, wire break, S7-1500, NO NC…"
          value={query}
          aria-label="Search knowledge"
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="segmented" role="group" aria-label="Category">
          {CATEGORIES.map((c) => (
            <button
              key={c.label}
              aria-pressed={category === c.id}
              onClick={() => setCategory(c.id)}
            >
              {c.label}
            </button>
          ))}
        </div>
      </div>
      <ErrorNotice
        error={error ?? catalog.error}
        onDismiss={() => setError(null)}
      />
      {articleId ? (
        <ArticleView
          key={articleId}
          id={articleId}
          open={open}
          back={() => navigate({ page: "knowledge", article: null, query })}
          navigate={navigate}
        />
      ) : hits && query.trim() ? (
        <SearchResults hits={hits} open={open} navigate={navigate} />
      ) : catalog.data ? (
        <Catalog catalog={catalog.data} open={open} />
      ) : null}
    </div>
  );
}

function SearchResults({
  hits,
  open,
  navigate,
}: {
  hits: SearchHitDto[];
  open: (id: string) => void;
  navigate: Navigate;
}) {
  if (hits.length === 0)
    return <Empty>No matching articles. Try fewer or different words.</Empty>;
  return (
    <div className="list">
      {hits.map((h) => (
        <button
          key={h.id}
          className="list-row"
          onClick={() => {
            if (h.docType === "lesson")
              navigate({ page: "learn", lesson: h.id });
            else if (h.docType === "exercise")
              void openExercise(h.id, navigate, "practice");
            else open(h.id);
          }}
        >
          <div className="grow">
            <div className="row">
              <strong>{h.title}</strong>
              <Badge kind={CATEGORY_KIND[h.category] ?? "normal"}>
                {h.category.replaceAll("_", " ").toLowerCase()}
              </Badge>
              <Badge>{h.docType}</Badge>
              {h.vendor && <Badge>{h.vendor}</Badge>}
            </div>
            <div className="muted" style={{ fontSize: 12 }}>
              {h.snippet || h.summary}
            </div>
          </div>
        </button>
      ))}
    </div>
  );
}

function Catalog({
  catalog,
  open,
}: {
  catalog: {
    vendors: CatalogEntryDto[];
    protocols: CatalogEntryDto[];
    fundamentals: CatalogEntryDto[];
    instructions: CatalogEntryDto[];
    troubleshooting: CatalogEntryDto[];
  };
  open: (id: string) => void;
}) {
  const group = (title: string, items: CatalogEntryDto[], note?: string) =>
    items.length > 0 && (
      <section>
        <div className="section-title">
          <h2>{title}</h2>
          {note && <span className="muted">{note}</span>}
        </div>
        <div className="tiles">
          {items.map((e) => (
            <button key={e.id} className="tile" onClick={() => open(e.id)}>
              <h3>
                {e.title} {e.tier && <Badge>Tier {e.tier}</Badge>}{" "}
                {e.area && <Badge kind="warning">{e.area}</Badge>}
              </h3>
              <p>{e.summary}</p>
              {e.children.length > 0 && (
                <div className="tile-meta">
                  {e.children.map((c) => c.title).join(" · ")}
                </div>
              )}
            </button>
          ))}
        </div>
      </section>
    );
  return (
    <>
      {group("Fundamentals", catalog.fundamentals)}
      {group(
        "PLC platforms",
        catalog.vendors,
        "Tier A: detailed profiles · Tier B: knowledge profiles",
      )}
      {group("Industrial communication", catalog.protocols)}
      {group("Instructions", catalog.instructions)}
      {group("Troubleshooting guides", catalog.troubleshooting)}
    </>
  );
}

function ArticleView({
  id,
  open,
  back,
  navigate,
}: {
  id: string;
  open: (id: string) => void;
  back: () => void;
  navigate: Navigate;
}) {
  const article = useLoad<ArticleDto>(() => api.article(id), [id]);
  const a = article.data;
  if (article.error) return <ErrorNotice error={article.error} />;
  if (!a) return <Empty>Loading…</Empty>;
  return (
    <div className="two-col" style={{ gridTemplateColumns: "1fr 300px" }}>
      <article className="prose">
        <button className="btn btn-ghost btn-sm" onClick={back}>
          ← Back
        </button>
        <div className="row wrap" style={{ marginTop: 10 }}>
          <Badge kind={CATEGORY_KIND[a.category] ?? "normal"}>
            {a.category.replaceAll("_", " ").toLowerCase()}
          </Badge>
          <Badge>{a.kind}</Badge>
          {a.tier && <Badge>Tier {a.tier}</Badge>}
          {a.tags.map((t) => (
            <span key={t} className="dim" style={{ fontSize: 11 }}>
              #{t}
            </span>
          ))}
        </div>
        <h2 style={{ fontSize: 22, marginTop: 8 }}>{a.title}</h2>
        <p>{a.summary}</p>
        {a.sections.map((s) => (
          <section key={s.heading}>
            <h3>{s.heading}</h3>
            <Markdown text={s.body} />
          </section>
        ))}
        {a.instruction && (
          <section>
            <h3>Interface</h3>
            <table>
              <thead>
                <tr>
                  <th>Parameter</th>
                  <th>Type</th>
                  <th>Meaning</th>
                </tr>
              </thead>
              <tbody>
                {[
                  ...a.instruction.inputs.map((p) => ({ ...p, dir: "IN" })),
                  ...a.instruction.outputs.map((p) => ({ ...p, dir: "OUT" })),
                ].map((p) => (
                  <tr key={p.dir + p.name}>
                    <td>
                      <code>{p.name}</code> <span className="dim">{p.dir}</span>
                    </td>
                    <td>{p.dataType}</td>
                    <td>{p.description}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            <p>
              <strong>State:</strong> {a.instruction.state}{" "}
              <strong>Retentivity:</strong> {a.instruction.retentivity}
            </p>
            <p>
              <strong>Timing:</strong> {a.instruction.timing}
            </p>
            <p>
              <strong>PLC Lab support:</strong>{" "}
              {a.instruction.simulationSupport
                .replaceAll("_", " ")
                .toLowerCase()}
            </p>
            {a.instruction.equivalents.length > 0 && (
              <table>
                <thead>
                  <tr>
                    <th>Platform</th>
                    <th>Name</th>
                    <th>Notes</th>
                  </tr>
                </thead>
                <tbody>
                  {a.instruction.equivalents.map((e) => (
                    <tr key={e.platform}>
                      <td>{e.platform}</td>
                      <td>
                        <code>{e.name}</code>
                      </td>
                      <td>{e.notes}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </section>
        )}
        {a.troubleshooting && (
          <section>
            <h3>Symptoms</h3>
            <ul>
              {a.troubleshooting.symptoms.map((s) => (
                <li key={s}>{s}</li>
              ))}
            </ul>
            <h3>Evidence to collect</h3>
            <ul>
              {a.troubleshooting.evidence.map((s) => (
                <li key={s}>{s}</li>
              ))}
            </ul>
            <h3>Diagnostic method</h3>
            <ol>
              {a.troubleshooting.diagnosticMethod.map((s) => (
                <li key={s}>{s}</li>
              ))}
            </ol>
            <h3>Repair</h3>
            <ul>
              {a.troubleshooting.repair.map((s) => (
                <li key={s}>{s}</li>
              ))}
            </ul>
            <h3>Why it happens</h3>
            <p>{a.troubleshooting.explanation}</p>
            {a.troubleshooting.simulatedIn.length > 0 && (
              <p>
                Practice it:{" "}
                {a.troubleshooting.simulatedIn.map((r) => (
                  <button
                    key={r.id}
                    className="btn btn-sm"
                    onClick={() => navigate({ page: "troubleshoot" })}
                  >
                    {r.title}
                  </button>
                ))}
              </p>
            )}
          </section>
        )}
      </article>
      <aside className="col">
        {a.facts.length > 0 && (
          <div className="panel">
            <div className="panel-head">
              <h4>Key facts</h4>
            </div>
            <table className="table fact-table">
              <tbody>
                {a.facts.map((f) => (
                  <tr key={f.label}>
                    <td style={{ whiteSpace: "normal" }}>{f.label}</td>
                    <td style={{ whiteSpace: "normal" }}>
                      {f.value}
                      {f.sources.length > 0 && (
                        <span className="dim"> [{f.sources.join(", ")}]</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        {a.fidelity && (
          <div className="panel">
            <div className="panel-head">
              <h4>Simulation fidelity</h4>
              <Badge
                kind={
                  a.fidelity.level === "EXACT_BEHAVIORAL"
                    ? "active"
                    : a.fidelity.level === "CONCEPTUAL"
                      ? "manual"
                      : "auto"
                }
              >
                {a.fidelity.level.replaceAll("_", " ").toLowerCase()}
              </Badge>
            </div>
            <div className="panel-body" style={{ fontSize: 12 }}>
              {a.fidelity.modeled.length > 0 && (
                <>
                  <strong>Modeled</strong>
                  <ul>
                    {a.fidelity.modeled.map((m) => (
                      <li key={m}>{m}</li>
                    ))}
                  </ul>
                </>
              )}
              {a.fidelity.approximated.length > 0 && (
                <>
                  <strong>Simulation approximation</strong>
                  <ul>
                    {a.fidelity.approximated.map((m) => (
                      <li key={m}>{m}</li>
                    ))}
                  </ul>
                </>
              )}
              {a.fidelity.omitted.length > 0 && (
                <>
                  <strong>Not simulated</strong>
                  <ul>
                    {a.fidelity.omitted.map((m) => (
                      <li key={m}>{m}</li>
                    ))}
                  </ul>
                </>
              )}
            </div>
          </div>
        )}
        {a.sources.length > 0 && (
          <div className="panel">
            <div className="panel-head">
              <h4>Sources</h4>
            </div>
            <div className="panel-body col">
              {a.sources.map((s) => (
                <div key={s.id} className="source-card">
                  <div className="row">
                    <strong className="grow">{s.title}</strong>
                    <Badge
                      kind={
                        s.verification === "VERIFIED" ? "active" : "warning"
                      }
                    >
                      {s.verification.toLowerCase()}
                    </Badge>
                  </div>
                  <div className="muted">
                    {s.documentType}
                    {s.documentNumber && ` · ${s.documentNumber}`} · retrieved{" "}
                    {s.retrieved}
                  </div>
                  <div
                    className="dim mono"
                    style={{ wordBreak: "break-all", userSelect: "all" }}
                  >
                    {s.url}
                  </div>
                  {s.notes && (
                    <div className="muted" style={{ marginTop: 4 }}>
                      {s.notes}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}
        {a.related.length > 0 && (
          <div className="panel">
            <div className="panel-head">
              <h4>Related</h4>
            </div>
            <div className="panel-body col">
              {a.related.map((r) => (
                <button
                  key={r.id}
                  className="btn btn-ghost btn-sm"
                  style={{ justifyContent: "flex-start" }}
                  onClick={() => open(r.id)}
                >
                  {r.title}
                </button>
              ))}
            </div>
          </div>
        )}
      </aside>
    </div>
  );
}
