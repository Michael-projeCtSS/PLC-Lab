import { useInterface } from "../locale";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  api,
  type EventDto,
  type SignalValueDto,
  type TrendDto,
  type VariablesDto,
} from "@plc-lab/ipc";
import { formatValue, numeric, seconds, valueClass } from "../format";
import { Badge, Tabs, VirtualList } from "../ui";
import { useFrameQuery } from "./useFrameQuery";
import type { Selection } from "./ProjectTree";
import type { Lab } from "./useLab";
import { WatchTables } from "./WatchTables";

type DockTab = "variables" | "watch" | "waves" | "events" | "messages";

export function Dock({
  lab,
  select = () => undefined,
  advanced = true,
}: {
  advanced?: boolean;
  lab: Lab;
  select?: (s: Selection) => void;
}) {
  const { t } = useInterface();
  const [tab, setTab] = useState<DockTab>("variables");
  const [shownError, setShownError] = useState<number | null>(null);
  const latestError =
    [...lab.messages].reverse().find((m) => m.kind === "error")?.id ?? null;
  if (latestError !== shownError) {
    setShownError(latestError);
    if (latestError !== null) setTab("messages");
  }
  const activeTab = !advanced && tab !== "messages" ? "variables" : tab;
  const errors = lab.messages.filter((m) => m.kind === "error").length;
  return (
    <>
      <Tabs<DockTab>
        label={t("dock.monitoring")}
        active={activeTab}
        onSelect={setTab}
        tabs={[
          { id: "variables", label: t("dock.variables") },
          ...(advanced
            ? [
                {
                  id: "watch" as const,
                  label: t("dock.watch"),
                  ...(lab.watch.length
                    ? { badge: <span className="dim">{lab.watch.length}</span> }
                    : {}),
                },
                { id: "waves" as const, label: t("dock.waves") },
                { id: "events" as const, label: t("dock.events") },
              ]
            : []),
          ...(advanced || lab.messages.length > 0
            ? [
                {
                  id: "messages" as const,
                  label: t("dock.messages"),
                  ...(errors
                    ? { badge: <Badge kind="fault">{errors}</Badge> }
                    : {}),
                },
              ]
            : []),
        ]}
      />
      <div className="ws-dock-body">
        {activeTab === "variables" && (
          <Variables lab={lab} select={select} advanced={advanced} />
        )}
        {activeTab === "watch" && <WatchTables lab={lab} select={select} />}
        {activeTab === "waves" && <Waveforms lab={lab} />}
        {activeTab === "events" && <Events lab={lab} />}
        {activeTab === "messages" && <Messages lab={lab} />}
      </div>
    </>
  );
}

export function ForceCell({
  lab,
  signal,
  kind,
  valueType,
  forced,
}: {
  lab: Lab;
  signal: number;
  kind: string;
  valueType: string;
  forced: SignalValueDto | null;
}) {
  const { t } = useInterface();
  const ws = lab.ws;
  if (!ws || (kind !== "PLC_INPUT" && kind !== "PLC_OUTPUT")) return null;
  const force = (value: SignalValueDto | null) =>
    void lab.act(
      "force",
      () => api.force(ws, signal, value),
      (f) => f,
    );
  if (forced)
    return (
      <span className="row" style={{ gap: 4 }}>
        <Badge kind="forced">F {formatValue(forced)}</Badge>
        <button className="btn btn-sm btn-ghost" onClick={() => force(null)}>
          {t("dock.unforce")}
        </button>
      </span>
    );
  if (valueType === "INT" || valueType === "REAL")
    return <NumericForce valueType={valueType} force={force} />;
  if (valueType !== "BOOL") return null;
  return (
    <span className="row" style={{ gap: 2 }}>
      <button
        className="btn btn-sm btn-ghost"
        title={`${t("dock.force")} TRUE`}
        aria-description={t("help.force")}
        onClick={() => force({ type: "BOOL", value: true })}
      >
        F1
      </button>
      <button
        className="btn btn-sm btn-ghost"
        title={`${t("dock.force")} FALSE`}
        aria-description={t("help.force")}
        onClick={() => force({ type: "BOOL", value: false })}
      >
        F0
      </button>
    </span>
  );
}

/** Force an INT or REAL tag to a typed value; the backend checks the type and range. */
export function NumericForce({
  valueType,
  force,
}: {
  valueType: "INT" | "REAL";
  force: (value: SignalValueDto) => void;
}) {
  const [text, setText] = useState("");
  const number = Number(text);
  const valid =
    text.trim() !== "" &&
    Number.isFinite(number) &&
    (valueType === "REAL" ||
      (Number.isInteger(number) && number >= -32768 && number <= 32767));
  return (
    <span className="row" style={{ gap: 2 }}>
      <input
        className="input mono"
        style={{ width: 70, height: 22 }}
        aria-label={`Force value (${valueType})`}
        value={text}
        onChange={(e) => setText(e.target.value)}
      />
      <button
        className="btn btn-sm btn-ghost"
        disabled={!valid}
        onClick={() =>
          force(
            valueType === "INT"
              ? { type: "INT", value: number }
              : { type: "REAL", value: number },
          )
        }
      >
        Force
      </button>
    </span>
  );
}

function Variables({
  lab,
  select,
  advanced,
}: {
  advanced: boolean;
  lab: Lab;
  select: (s: Selection) => void;
}) {
  const { t } = useInterface();
  const [filter, setFilter] = useState("");
  const [detail, setDetail] = useState(false);
  const ws = lab.ws;
  const vars = useFrameQuery<VariablesDto>(
    lab,
    () =>
      ws
        ? api.variables(ws, filter.trim() || null, 0, 500)
        : Promise.reject(new Error("No workspace")),
    [filter],
  );
  const signals = useMemo(
    () => new Map(lab.signals.map((s) => [s.id, s])),
    [lab.signals],
  );
  const rows = vars.data?.rows ?? [];
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="row" style={{ padding: "4px 8px" }}>
        <input
          className="input"
          placeholder={t("dock.filter")}
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          aria-label={t("dock.filter")}
          style={{ width: 220 }}
        />
        {advanced && (
          <label className="row muted" style={{ fontSize: 12 }}>
            <input
              type="checkbox"
              checked={detail}
              onChange={(e) => setDetail(e.target.checked)}
            />{" "}
            {t("dock.details")}
          </label>
        )}
        <span className="spacer" />
        <span className="dim" style={{ fontSize: 11 }}>
          {vars.data ? t("dock.count", { count: vars.data.total }) : ""}
        </span>
      </div>
      <div style={{ flex: 1, overflow: "auto" }}>
        {!lab.frame?.plc.programName ? (
          <div className="empty">
            <h3>{t("dock.emptyTitle")}</h3>
            <p>{t("dock.empty")}</p>
            <button
              className="btn btn-primary"
              disabled={lab.busy !== null}
              onClick={() => void lab.buildAndLoad(true)}
            >
              {t("control.build")}
            </button>
          </div>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>{t("dock.name")}</th>
                <th>{t("dock.kind")}</th>
                <th title={t("dock.scanHelp")}>{t("dock.scan")}</th>
                <th title={t("dock.committedHelp")}>{t("dock.committed")}</th>
                <th title={t("dock.nextHelp")}>{t("dock.next")}</th>
                {advanced && detail && <th>I/O</th>}
                {advanced && detail && <th>{t("dock.quality")}</th>}
                {advanced && (
                  <>
                    <th>{t("dock.force")}</th>
                    <th />
                  </>
                )}
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => {
                const s = signals.get(r.signal);
                const watched = s ? lab.watch.includes(s.path) : false;
                return (
                  <tr key={r.signal}>
                    <td className="mono">
                      {advanced ? (
                        <button
                          className="btn btn-sm btn-ghost mono"
                          title="Show the online detail of this variable"
                          onClick={() =>
                            select({ kind: "signal", id: r.signal })
                          }
                        >
                          {r.name}
                        </button>
                      ) : (
                        <span>{r.name}</span>
                      )}
                    </td>
                    <td className="dim">
                      {r.kind === "PLC_INPUT"
                        ? t("dock.input")
                        : r.kind === "PLC_OUTPUT"
                          ? t("dock.output")
                          : r.kind === "BLOCK_OUTPUT"
                            ? t("dock.block")
                            : t("dock.local")}
                    </td>
                    <td className={valueClass(r.scanValue)}>
                      {r.kind === "PLC_INPUT" ? formatValue(r.scanValue) : ""}
                    </td>
                    <td className={valueClass(r.committed)}>
                      {r.kind === "PLC_INPUT" ? "" : formatValue(r.committed)}
                    </td>
                    <td
                      className={`${valueClass(r.staged)}${
                        r.kind === "PLC_INPUT" &&
                        JSON.stringify(r.staged) !== JSON.stringify(r.scanValue)
                          ? " changed"
                          : ""
                      }`}
                    >
                      {r.kind === "PLC_INPUT" ? formatValue(r.staged) : ""}
                    </td>
                    {advanced && detail && (
                      <td className="mono dim">{r.source ?? ""}</td>
                    )}
                    {advanced && detail && (
                      <td>
                        {r.quality && r.quality !== "GOOD" ? (
                          <Badge kind="bad">
                            {r.quality.replaceAll("_", " ")}
                          </Badge>
                        ) : (
                          ""
                        )}
                      </td>
                    )}
                    {advanced && (
                      <>
                        <td>
                          <ForceCell
                            lab={lab}
                            signal={r.signal}
                            kind={r.kind}
                            valueType={s?.valueType ?? ""}
                            forced={r.forced}
                          />
                        </td>
                        <td>
                          {s && (
                            <button
                              className="btn btn-sm btn-ghost"
                              onClick={() =>
                                lab.setWatch(
                                  watched
                                    ? lab.watch.filter((p) => p !== s.path)
                                    : [...lab.watch, s.path].slice(0, 64),
                                )
                              }
                            >
                              {watched ? "Unwatch" : "Watch"}
                            </button>
                          )}
                        </td>
                      </>
                    )}
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

const WINDOWS = [
  { label: "5 s", us: 5_000_000n },
  { label: "20 s", us: 20_000_000n },
  { label: "60 s", us: 60_000_000n },
  { label: "5 min", us: 300_000_000n },
];

function Waveforms({ lab }: { lab: Lab }) {
  const [windowIndex, setWindowIndex] = useState(1);
  const [cursor, setCursor] = useState<number | null>(null);
  const ws = lab.ws;
  const tracked = useMemo(() => lab.watchIds.slice(0, 8), [lab.watchIds]);
  const span = WINDOWS[windowIndex]?.us ?? 20_000_000n;
  const now = BigInt(lab.frame?.worldTimeUs ?? "0");
  const from = now > span ? now - span : 0n;
  const trend = useFrameQuery<TrendDto>(
    lab,
    () =>
      ws && tracked.length
        ? api.trend(ws, tracked, String(from), String(now), 800)
        : Promise.resolve({ series: [], oldestUs: null, latestUs: null }),
    [tracked.join(","), windowIndex],
  );
  const signals = new Map(lab.signals.map((s) => [s.id, s]));
  const box = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(800);
  useEffect(() => {
    const el = box.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const o = new ResizeObserver(() => setWidth(Math.max(300, el.clientWidth)));
    o.observe(el);
    return () => o.disconnect();
  }, []);
  if (tracked.length === 0)
    return (
      <div className="empty">
        Waveforms show watched signals. Add signals in the Watch tab or with
        “Watch” in Variables.
      </div>
    );
  const left = 150;
  const plotW = width - left - 10;
  const rowH = 44;
  const x = (t: bigint) =>
    left + (Number(t - from) / Number(span || 1n)) * plotW;
  const series = trend.data?.series ?? [];
  const cursorT =
    cursor === null
      ? null
      : from + BigInt(Math.round(((cursor - left) / plotW) * Number(span)));
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="row" style={{ padding: "4px 8px" }}>
        <div className="segmented" role="group" aria-label="Time window">
          {WINDOWS.map((w, i) => (
            <button
              key={w.label}
              aria-pressed={i === windowIndex}
              onClick={() => setWindowIndex(i)}
            >
              {w.label}
            </button>
          ))}
        </div>
        <span className="muted" style={{ fontSize: 12 }}>
          {cursorT !== null
            ? `Cursor ${seconds(cursorT > 0n ? cursorT : 0n)}`
            : "Hover the plot for values"}
        </span>
        <span className="spacer" />
        <span className="dim" style={{ fontSize: 11 }}>
          PLC values at scan time · process values at cycle end
        </span>
      </div>
      <div ref={box} style={{ flex: 1, overflow: "auto" }}>
        <svg
          className="wave"
          width={width}
          height={tracked.length * rowH + 20}
          onMouseMove={(e) =>
            setCursor(
              e.nativeEvent.offsetX >= left ? e.nativeEvent.offsetX : null,
            )
          }
          onMouseLeave={() => setCursor(null)}
          role="img"
          aria-label="Waveforms"
        >
          {[0, 0.25, 0.5, 0.75, 1].map((f) => (
            <g key={f}>
              <line
                className="grid"
                x1={left + f * plotW}
                x2={left + f * plotW}
                y1={0}
                y2={tracked.length * rowH}
              />
              <text
                x={left + f * plotW}
                y={tracked.length * rowH + 14}
                textAnchor="middle"
              >
                {seconds(from + BigInt(Math.round(f * Number(span))))}
              </text>
            </g>
          ))}
          {tracked.map((id, row) => {
            const s = series.find((x) => x.signal === id);
            const sig = signals.get(id);
            const top = row * rowH + 6;
            const h = rowH - 14;
            const points = s?.points ?? [];
            const isBool = sig?.valueType === "BOOL";
            const values = points.map((p) => numeric(p.value));
            const min = isBool ? 0 : Math.min(...values, 0);
            const max = isBool ? 1 : Math.max(...values, min + 1);
            const y = (v: number) =>
              top + h - ((v - min) / (max - min || 1)) * h;
            let d = "";
            points.forEach((p, i) => {
              const px = Math.max(left, x(BigInt(p.t)));
              const py = y(numeric(p.value));
              if (i === 0) d += `M ${px} ${py}`;
              else d += isBool ? ` H ${px} V ${py}` : ` L ${px} ${py}`;
            });
            if (points.length) d += ` H ${left + plotW}`;
            const atCursor =
              cursorT === null
                ? null
                : ([...points].reverse().find((p) => BigInt(p.t) <= cursorT) ??
                  null);
            return (
              <g key={id}>
                <line
                  className="grid"
                  x1={left}
                  x2={left + plotW}
                  y1={top + h + 4}
                  y2={top + h + 4}
                />
                <text x={8} y={top + 14} style={{ fill: "var(--text-1)" }}>
                  {(sig?.label ?? `#${id}`).slice(0, 22)}
                </text>
                <text x={8} y={top + 28}>
                  {atCursor
                    ? formatValue(atCursor.value, sig?.unit ?? null)
                    : points.length
                      ? formatValue(
                          points[points.length - 1]?.value,
                          sig?.unit ?? null,
                        )
                      : "no data"}
                </text>
                <path className={isBool ? "trace-bool" : "trace-num"} d={d} />
              </g>
            );
          })}
          {cursor !== null && (
            <line
              className="cursor"
              x1={cursor}
              x2={cursor}
              y1={0}
              y2={tracked.length * rowH}
            />
          )}
        </svg>
      </div>
    </div>
  );
}

function Events({ lab }: { lab: Lab }) {
  const [events, setEvents] = useState<EventDto[]>([]);
  const [gap, setGap] = useState(false);
  const [filter, setFilter] = useState<string>("all");
  const last = useRef<string | null>(null);
  const ws = lab.ws;
  const latest = lab.frame?.latestEvent ?? null;
  useEffect(() => {
    if (!ws || latest === last.current) return;
    let active = true;
    (async () => {
      let after = last.current;
      for (let page = 0; page < 10 && active; page++) {
        const p = await api.events(ws, after, 500);
        if (!active) return;
        if (p.gap) setGap(true);
        if (p.events.length) {
          after = p.events[p.events.length - 1]?.seq ?? after;
          setEvents((list) => [...list, ...p.events].slice(-5000));
        }
        last.current = after;
        if (p.events.length < 500) break;
      }
    })().catch((e: unknown) =>
      lab.log("error", e instanceof Error ? e.message : String(e)),
    );
    return () => {
      active = false;
    };
  }, [ws, latest, lab]);
  const shown =
    filter === "all"
      ? events
      : events.filter(
          (e) =>
            e.kind === filter || (filter === "alarms" && e.severity !== "info"),
        );
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="row" style={{ padding: "4px 8px" }}>
        <select
          className="input"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          aria-label="Event filter"
        >
          <option value="all">All events</option>
          <option value="alarms">Warnings and errors</option>
          <option value="PROCESS">Process</option>
          <option value="DIAGNOSTIC">Diagnostics</option>
          <option value="NETWORK">Network</option>
          <option value="PLC_MODE">PLC mode</option>
          <option value="OPERATOR">Operator</option>
        </select>
        {gap && (
          <span className="badge warning">
            Older events were discarded (bounded log)
          </span>
        )}
        <span className="spacer" />
        <span className="dim" style={{ fontSize: 11 }}>
          {shown.length} events
        </span>
      </div>
      <div style={{ flex: 1, minHeight: 0 }}>
        <VirtualList
          label="Event log"
          items={shown}
          rowHeight={24}
          stickToEnd
          render={(e) => (
            <div className={`event-row ${e.severity}`} style={{ height: 24 }}>
              <span className="mono dim">{seconds(e.timeUs)}</span>
              <span className="mono dim">#{e.cycle}</span>
              <span className="dim">
                {e.kind.replaceAll("_", " ").toLowerCase()}
              </span>
              <span className="text" title={e.source ?? undefined}>
                {e.text}
                {e.source ? <span className="dim"> · {e.source}</span> : null}
              </span>
            </div>
          )}
        />
      </div>
    </div>
  );
}

function Messages({ lab }: { lab: Lab }) {
  const { t } = useInterface();
  if (lab.messages.length === 0)
    return <div className="empty">{t("dock.messagesEmpty")}</div>;
  return (
    <div style={{ padding: 8 }} className="col">
      <div className="row">
        <span className="spacer" />
        <button className="btn btn-sm btn-ghost" onClick={lab.clearMessages}>
          {t("common.clear")}
        </button>
      </div>
      {[...lab.messages].reverse().map((m) => (
        <div key={m.id} className={`notice ${m.kind === "info" ? "" : m.kind}`}>
          <span className="dim mono">{m.time}</span> {m.text}
          {m.detail && (
            <pre style={{ margin: "4px 0 0", whiteSpace: "pre-wrap" }}>
              {m.detail}
            </pre>
          )}
        </div>
      ))}
    </div>
  );
}
