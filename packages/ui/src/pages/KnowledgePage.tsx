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
import { useInterface } from "../locale";
import {
  englishSearchAlias,
  localizeContent,
  searchPolishContent,
  hasCompleteTranslation,
  fidelityLabel,
  contentKindLabel,
} from "../locale/content";
import { openExercise, type Navigate } from "../route";

const CATEGORIES = [
  { id: null, label: "knowledge.all" },
  { id: "FUNDAMENTAL", label: "knowledge.fundamentals" },
  { id: "VENDOR_SPECIFIC", label: "knowledge.vendor" },
  { id: "TROUBLESHOOTING", label: "knowledge.troubleshooting" },
  { id: "PLC_LAB_SIMULATION", label: "knowledge.simulation" },
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
  const { preferences, t } = useInterface();
  const [query, setQuery] = useState(initialQuery);
  const [category, setCategory] = useState<string | null>(null);
  const [hits, setHits] = useState<SearchHitDto[] | null>(null);
  const [error, setError] = useState<unknown>(null);
  const catalog = useLoad(() => api.catalog(), []);
  const library = useLoad(() => api.library(), []);
  useEffect(() => {
    if (!query.trim()) return;
    let active = true;
    const timer = setTimeout(() => {
      api
        .search(
          preferences.language === "pl" ? englishSearchAlias(query) : query,
          { category },
        )
        .then(
          (h) => {
            if (!active) return;
            const collect = (entries: CatalogEntryDto[]): CatalogEntryDto[] =>
              entries.flatMap((entry) => [entry, ...collect(entry.children)]);
            const available = [
              ...(library.data?.lessons ?? []),
              ...(library.data?.exercises ?? []),
              ...collect(Object.values(catalog.data ?? {}).flat()),
            ];
            const polish =
              preferences.language === "pl"
                ? searchPolishContent(query, available, category)
                : [];
            const ids = new Set(polish.map((hit) => hit.id));
            setHits([
              ...polish,
              ...localizeContent(h, preferences.language).filter(
                (hit) => !ids.has(hit.id),
              ),
            ]);
          },
          (e: unknown) => active && setError(e),
        );
    }, 150);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [query, category, preferences.language, catalog.data, library.data]);
  const open = (id: string) =>
    navigate({ page: "knowledge", article: id, query });
  return (
    <div className="page-inner">
      <div className="page-header">
        <div>
          <h1>{t("knowledge.title")}</h1>
          <p>{t("knowledge.intro")}</p>
        </div>
      </div>
      <div className="row wrap" style={{ marginBottom: 16 }}>
        <input
          className="input grow"
          style={{ height: 34, fontSize: 14 }}
          type="search"
          placeholder={t("knowledge.placeholder")}
          value={query}
          aria-label={t("knowledge.search")}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div
          className="segmented"
          role="group"
          aria-label={t("knowledge.category")}
        >
          {CATEGORIES.map((c) => (
            <button
              key={c.id ?? "all"}
              aria-pressed={category === c.id}
              onClick={() => setCategory(c.id)}
            >
              {t(c.label)}
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
  const { t } = useInterface();
  if (hits.length === 0) return <Empty>{t("knowledge.none")}</Empty>;
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
                {t(
                  CATEGORIES.find((c) => c.id === h.category)?.label ??
                    "knowledge.all",
                )}
              </Badge>
              <Badge>{contentKindLabel(h.docType, t)}</Badge>
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
  const { t } = useInterface();
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
      {group(t("knowledge.fundamentals"), catalog.fundamentals)}
      {group(t("knowledge.platforms"), catalog.vendors, t("knowledge.tiers"))}
      {group(t("knowledge.protocols"), catalog.protocols)}
      {group(t("knowledge.instructions"), catalog.instructions)}
      {group(t("knowledge.guides"), catalog.troubleshooting)}
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
  const { t, preferences } = useInterface();
  const article = useLoad<ArticleDto>(() => api.article(id), [id]);
  const a = article.data;
  if (article.error) return <ErrorNotice error={article.error} />;
  if (!a) return <Empty>{t("knowledge.loading")}</Empty>;
  return (
    <div className="two-col article-layout">
      <article className="prose">
        <button className="btn btn-ghost btn-sm" onClick={back}>
          ← {t("common.back")}
        </button>
        <div className="row wrap" style={{ marginTop: 10 }}>
          <Badge kind={CATEGORY_KIND[a.category] ?? "normal"}>
            {t(
              CATEGORIES.find((c) => c.id === a.category)?.label ??
                "knowledge.all",
            )}
          </Badge>
          <Badge>{contentKindLabel(a.kind, t)}</Badge>
          {a.tier && <Badge>Tier {a.tier}</Badge>}
          {a.tags.map((t) => (
            <span key={t} className="dim" style={{ fontSize: 11 }}>
              #{t}
            </span>
          ))}
        </div>
        <h2 style={{ fontSize: 22, marginTop: 8 }}>{a.title}</h2>
        <p>{a.summary}</p>
        {preferences.language === "pl" && !hasCompleteTranslation(id) && (
          <p className="callout">{t("knowledge.fallback")}</p>
        )}
        {a.sections.map((s) => (
          <section key={s.heading}>
            <h3>{s.heading}</h3>
            <Markdown text={s.body} />
          </section>
        ))}
        {a.instruction && (
          <section>
            <h3>{t("knowledge.interface")}</h3>
            <table>
              <thead>
                <tr>
                  <th>{t("knowledge.parameter")}</th>
                  <th>{t("knowledge.type")}</th>
                  <th>{t("knowledge.meaning")}</th>
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
              <strong>{t("knowledge.state")}</strong> {a.instruction.state}{" "}
              <strong>{t("knowledge.retentive")}</strong>{" "}
              {a.instruction.retentivity}
            </p>
            <p>
              <strong>{t("knowledge.timing")}</strong> {a.instruction.timing}
            </p>
            <p>
              <strong>{t("knowledge.support")}</strong>{" "}
              {fidelityLabel(a.instruction.simulationSupport, t)}
            </p>
            {a.instruction.equivalents.length > 0 && (
              <table>
                <thead>
                  <tr>
                    <th>{t("knowledge.platform")}</th>
                    <th>{t("knowledge.name")}</th>
                    <th>{t("knowledge.notes")}</th>
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
            <h3>{t("knowledge.symptoms")}</h3>
            <ul>
              {a.troubleshooting.symptoms.map((s) => (
                <li key={s}>{s}</li>
              ))}
            </ul>
            <h3>{t("knowledge.evidence")}</h3>
            <ul>
              {a.troubleshooting.evidence.map((s) => (
                <li key={s}>{s}</li>
              ))}
            </ul>
            <h3>{t("knowledge.method")}</h3>
            <ol>
              {a.troubleshooting.diagnosticMethod.map((s) => (
                <li key={s}>{s}</li>
              ))}
            </ol>
            <h3>{t("knowledge.repair")}</h3>
            <ul>
              {a.troubleshooting.repair.map((s) => (
                <li key={s}>{s}</li>
              ))}
            </ul>
            <h3>{t("knowledge.why")}</h3>
            <p>{a.troubleshooting.explanation}</p>
            {a.troubleshooting.simulatedIn.length > 0 && (
              <p>
                {t("knowledge.practice")}{" "}
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
              <h4>{t("knowledge.facts")}</h4>
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
              <h4>{t("knowledge.fidelity")}</h4>
              <Badge
                kind={
                  a.fidelity.level === "EXACT_BEHAVIORAL"
                    ? "active"
                    : a.fidelity.level === "CONCEPTUAL"
                      ? "manual"
                      : "auto"
                }
              >
                {fidelityLabel(a.fidelity.level, t)}
              </Badge>
            </div>
            <div className="panel-body" style={{ fontSize: 12 }}>
              {a.fidelity.modeled.length > 0 && (
                <>
                  <strong>{t("knowledge.modeled")}</strong>
                  <ul>
                    {a.fidelity.modeled.map((m) => (
                      <li key={m}>{m}</li>
                    ))}
                  </ul>
                </>
              )}
              {a.fidelity.approximated.length > 0 && (
                <>
                  <strong>{t("knowledge.approximation")}</strong>
                  <ul>
                    {a.fidelity.approximated.map((m) => (
                      <li key={m}>{m}</li>
                    ))}
                  </ul>
                </>
              )}
              {a.fidelity.omitted.length > 0 && (
                <>
                  <strong>{t("knowledge.omitted")}</strong>
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
              <h4>{t("knowledge.sources")}</h4>
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
                      {t(
                        s.verification === "VERIFIED"
                          ? "source.verified"
                          : "source.unverified",
                      )}
                    </Badge>
                  </div>
                  <div className="muted">
                    {s.documentType}
                    {s.documentNumber && ` · ${s.documentNumber}`} ·{" "}
                    {t("knowledge.retrieved")} {s.retrieved}
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
              <h4>{t("knowledge.related")}</h4>
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
