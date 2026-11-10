/**
 * PID tuning (ADR-0049). The controller's setpoint, process value and output come from
 * the trace, its parts, limits and mode from the PLC, both read from the backend; the
 * view computes no control action. Parameters are changed by writing, online, the
 * program variables the controller's inputs are bound to; the operator controls of the
 * scene (MAN/AUTO selector, the vessel door) are sent like any other gesture.
 */
import { useEffect, useMemo, useRef, useState } from "react";
import {
  api,
  type ControlLoopDto,
  type SignalValueDto,
  type TrendDto,
  type WatchDto,
} from "@plc-lab/ipc";
import { formatValue, numeric, seconds } from "../format";
import { useInterface } from "../locale";
import { Badge } from "../ui";
import type { TranslationKey } from "../locale/en";
import { useFrameQuery } from "./useFrameQuery";
import {
  flag,
  parseParameter,
  real,
  timeSeconds,
  type ParameterKind,
} from "./tuning";
import type { Lab } from "./useLab";

const WINDOWS = [
  { label: "60 s", us: 60_000_000n },
  { label: "5 min", us: 300_000_000n },
  { label: "15 min", us: 900_000_000n },
];

type Parameter = {
  name: string;
  label: TranslationKey;
  kind: ParameterKind;
};
/** The inputs a tuning screen adjusts, in the order an engineer works through them. */
const PARAMETERS: Parameter[] = [
  { name: "SP", label: "tune.param.SP", kind: "REAL" },
  { name: "KP", label: "tune.param.KP", kind: "REAL" },
  { name: "TI", label: "tune.param.TI", kind: "TIME" },
  { name: "TD", label: "tune.param.TD", kind: "TIME" },
  { name: "MV_MIN", label: "tune.param.MV_MIN", kind: "REAL" },
  { name: "MV_MAX", label: "tune.param.MV_MAX", kind: "REAL" },
  { name: "MV_MAN", label: "tune.param.MV_MAN", kind: "REAL" },
  { name: "MANUAL", label: "tune.param.MANUAL", kind: "BOOL" },
];
/** Live state shown next to the trend. */
const STATE: { name: string; label: TranslationKey }[] = [
  { name: "SP", label: "tune.param.SP" },
  { name: "PV", label: "tune.state.PV" },
  { name: "E", label: "tune.state.E" },
  { name: "MV", label: "tune.state.MV" },
  { name: "P", label: "tune.state.P" },
  { name: "I", label: "tune.state.I" },
  { name: "D", label: "tune.state.D" },
  { name: "SAT", label: "tune.state.SAT" },
];

export function TuningTab({ lab }: { lab: Lab }) {
  const { t } = useInterface();
  const ws = lab.ws;
  const loops = lab.controlLoops;
  const [chosen, setChosen] = useState<string | null>(null);
  const loop = loops.find((l) => l.instance === chosen) ?? loops[0] ?? null;
  if (!ws || !lab.frame) return null;
  if (!loop)
    return (
      <div className="empty" role="status">
        {t("tune.none")}
      </div>
    );
  return (
    <div className="tuning">
      {loops.length > 1 && (
        <label className="row" style={{ gap: 6 }}>
          {t("tune.loop")}
          <select
            value={loop.instance}
            onChange={(e) => setChosen(e.target.value)}
          >
            {loops.map((l) => (
              <option key={l.instance} value={l.instance}>
                {l.instance}
              </option>
            ))}
          </select>
        </label>
      )}
      <LoopView key={loop.instance} lab={lab} loop={loop} />
    </div>
  );
}

function LoopView({ lab, loop }: { lab: Lab; loop: ControlLoopDto }) {
  const { t } = useInterface();
  const ws = lab.ws;
  const signal = (name: string) =>
    loop.signals.find((s) => s.parameter === name)?.signal ?? null;
  const binding = (name: string) =>
    loop.bindings.find((s) => s.parameter === name)?.signal ?? null;
  const ids = useMemo(
    () => [
      ...new Set([
        ...loop.signals.map((s) => s.signal),
        ...loop.bindings.map((s) => s.signal),
      ]),
    ],
    [loop],
  );
  const live = useFrameQuery<WatchDto | null>(
    lab,
    () => (ws ? api.watch(ws, ids) : Promise.resolve(null)),
    [ids.join(",")],
  );
  const values = new Map(
    (live.data?.rows ?? []).map((r) => [r.signal, r.value]),
  );
  const value = (name: string) => {
    const id = signal(name);
    return id === null ? null : (values.get(id) ?? null);
  };
  const variableName = (id: number) =>
    lab.signals.find((s) => s.id === id)?.label ?? `#${id}`;
  const manual = flag(value("MANUAL"));
  const sat = flag(value("SAT"));
  const mv = real(value("MV"));
  const lo = real(value("MV_MIN"));
  const hi = real(value("MV_MAX"));
  const ti = timeSeconds(value("TI"));
  const td = timeSeconds(value("TD"));
  const running = lab.frame?.simulation.state === "RUNNING";

  const diagnostics: string[] = [];
  if (manual)
    diagnostics.push(
      t("tune.diag.manual", { value: formatValue(value("MV_MAN")) }),
    );
  else if (manual === false)
    diagnostics.push(t("tune.diag.auto", { error: formatValue(value("E")) }));
  if (sat && mv !== null && hi !== null && mv >= hi)
    diagnostics.push(
      t("tune.diag.satHigh", { limit: formatValue(value("MV_MAX")) }),
    );
  else if (sat && mv !== null && lo !== null && mv <= lo)
    diagnostics.push(
      t("tune.diag.satLow", { limit: formatValue(value("MV_MIN")) }),
    );
  if (ti === 0) diagnostics.push(t("tune.diag.noI"));
  if (td === 0) diagnostics.push(t("tune.diag.noD"));
  if (!running) diagnostics.push(t("tune.diag.paused"));

  return (
    <>
      <div className="row wrap" style={{ gap: 8 }}>
        <strong>{loop.instance}</strong>
        {manual !== null && (
          <Badge kind={manual ? "manual" : "auto"}>
            {manual ? t("tune.manual") : t("tune.auto")}
          </Badge>
        )}
        {sat && <Badge kind="warning">{t("tune.saturated")}</Badge>}
      </div>
      <Trend
        lab={lab}
        sp={signal("SP")}
        pv={signal("PV")}
        mv={signal("MV")}
        limits={[lo ?? 0, hi ?? 100]}
        name={loop.instance}
      />
      <div className="tuning-grid">
        <section aria-label={t("tune.state")}>
          <h3>{t("tune.state")}</h3>
          <table className="table">
            <tbody>
              {STATE.map((s) => (
                <tr key={s.name}>
                  <th scope="row">{t(s.label)}</th>
                  <td className="mono" data-parameter={s.name}>
                    {formatValue(value(s.name))}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
        <section aria-label={t("tune.parameters")}>
          <h3>{t("tune.parameters")}</h3>
          <p className="muted" style={{ fontSize: 12 }}>
            {t("tune.writeHelp")}
          </p>
          <table className="table">
            <tbody>
              {PARAMETERS.map((p) => (
                <ParameterRow
                  key={p.name}
                  lab={lab}
                  parameter={p}
                  current={value(p.name)}
                  variable={binding(p.name)}
                  variableName={variableName}
                />
              ))}
            </tbody>
          </table>
        </section>
        <section aria-label={t("tune.operate")}>
          <h3>{t("tune.operate")}</h3>
          <OperatorControls lab={lab} />
          <SetpointStep
            lab={lab}
            setpoint={real(value("SP"))}
            variable={binding("SP")}
          />
        </section>
        <section aria-label={t("tune.diagnostics")}>
          <h3>{t("tune.diagnostics")}</h3>
          <ul className="tuning-diagnostics" role="status">
            {diagnostics.map((d) => (
              <li key={d}>{d}</li>
            ))}
          </ul>
        </section>
      </div>
    </>
  );
}

function ParameterRow({
  lab,
  parameter,
  current,
  variable,
  variableName,
}: {
  lab: Lab;
  parameter: Parameter;
  current: SignalValueDto | null;
  variable: number | null;
  variableName: (id: number) => string;
}) {
  const { t } = useInterface();
  const [text, setText] = useState("");
  const [invalid, setInvalid] = useState(false);
  const ws = lab.ws;
  const label = t(parameter.label);
  const write = (value: SignalValueDto) => {
    if (!ws || variable === null) return;
    void lab
      .act(
        "write",
        () => api.modify(ws, variable, value),
        (f) => f,
      )
      .then((f) => {
        if (f) setText("");
      });
  };
  const shown =
    parameter.kind === "TIME"
      ? `${timeSeconds(current)?.toString() ?? "—"} s`
      : formatValue(current);
  return (
    <tr>
      <th scope="row">{label}</th>
      <td className="mono" data-parameter={parameter.name}>
        {shown}
      </td>
      <td>
        {variable === null ? (
          <span className="muted">{t("tune.fixed")}</span>
        ) : parameter.kind === "BOOL" ? (
          <button
            className="btn"
            disabled={lab.busy !== null}
            onClick={() =>
              write({ type: "BOOL", value: !(flag(current) ?? false) })
            }
          >
            {flag(current) ? t("tune.toAuto") : t("tune.toManual")}
          </button>
        ) : (
          <form
            className="row"
            style={{ gap: 4 }}
            onSubmit={(e) => {
              e.preventDefault();
              const value = parseParameter(parameter.kind, text);
              setInvalid(value === null);
              if (value) write(value);
            }}
          >
            <input
              className="input"
              size={7}
              inputMode="decimal"
              aria-label={t("tune.newValue", { parameter: label })}
              aria-invalid={invalid}
              placeholder={parameter.kind === "TIME" ? "s" : ""}
              value={text}
              onChange={(e) => setText(e.target.value)}
            />
            <button className="btn" type="submit" disabled={lab.busy !== null}>
              {t("tune.write")}
            </button>
          </form>
        )}
        {variable !== null && (
          <div className="dim" style={{ fontSize: 11 }}>
            {t("tune.boundTo", { variable: variableName(variable) })}
          </div>
        )}
      </td>
    </tr>
  );
}

/** The scene's operator controls a tuning session needs: MAN/AUTO selectors and doors. */
function OperatorControls({ lab }: { lab: Lab }) {
  const { t } = useInterface();
  const components = lab.workspace?.scene.components ?? [];
  const state = (id: string, name: string) =>
    lab.frame?.components
      .find((c) => c.id === id)
      ?.quantities.find((q) => q.name === name)?.value;
  const controls = components.flatMap((c) => {
    if (c.kind === "selectorSwitch" && c.positions.length > 1) {
      const position = state(c.id, "position");
      const at =
        position && position.type !== "BOOL" ? Number(position.value) : 0;
      return [
        <div key={c.id} className="row" style={{ gap: 6 }}>
          <span>{c.label}</span>
          <div className="segmented" role="group" aria-label={c.label}>
            {c.positions.map((p, i) => (
              <button
                key={p}
                aria-pressed={i === at}
                onClick={() =>
                  void lab.operate(c.id, { type: "selector", position: i })
                }
              >
                {p}
              </button>
            ))}
          </div>
        </div>,
      ];
    }
    if (c.kind === "heatedVessel") {
      const open = state(c.id, "doorOpen");
      const isOpen = open?.type === "BOOL" && open.value;
      return [
        <button
          key={c.id}
          className="btn"
          aria-pressed={isOpen}
          onClick={() =>
            void lab.operate(c.id, { type: "door", open: !isOpen })
          }
        >
          {t(isOpen ? "tune.closeDoor" : "tune.openDoor", { label: c.label })}
        </button>,
      ];
    }
    return [];
  });
  if (!controls.length) return <p className="muted">{t("tune.noControls")}</p>;
  return (
    <div className="row wrap" style={{ gap: 8 }}>
      {controls}
    </div>
  );
}

function SetpointStep({
  lab,
  setpoint,
  variable,
}: {
  lab: Lab;
  setpoint: number | null;
  variable: number | null;
}) {
  const { t } = useInterface();
  const ws = lab.ws;
  if (!ws || variable === null || setpoint === null) return null;
  const step = (delta: number) =>
    void lab.act(
      "write",
      () => api.modify(ws, variable, { type: "REAL", value: setpoint + delta }),
      (f) => f,
    );
  return (
    <div className="row" style={{ gap: 6, marginTop: 8 }}>
      <span>{t("tune.spStep")}</span>
      {[-10, -5, 5, 10].map((d) => (
        <button
          key={d}
          className="btn"
          disabled={lab.busy !== null}
          onClick={() => step(d)}
        >
          {d > 0 ? `+${d}` : String(d)}
        </button>
      ))}
    </div>
  );
}

function Trend({
  lab,
  sp,
  pv,
  mv,
  limits,
  name,
}: {
  lab: Lab;
  sp: number | null;
  pv: number | null;
  mv: number | null;
  limits: [number, number];
  name: string;
}) {
  const { t } = useInterface();
  const ws = lab.ws;
  const [windowIndex, setWindowIndex] = useState(0);
  const span = WINDOWS[windowIndex]?.us ?? 60_000_000n;
  const now = BigInt(lab.frame?.worldTimeUs ?? "0");
  const from = now > span ? now - span : 0n;
  const ids = [sp, pv, mv].filter((id): id is number => id !== null);
  const trend = useFrameQuery<TrendDto>(
    lab,
    () =>
      ws && ids.length
        ? api.trend(ws, ids, String(from), String(now), 600)
        : Promise.resolve({ series: [], oldestUs: null, latestUs: null }),
    [ids.join(","), windowIndex],
  );
  const box = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(800);
  useEffect(() => {
    const el = box.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const o = new ResizeObserver(() => setWidth(Math.max(320, el.clientWidth)));
    o.observe(el);
    return () => o.disconnect();
  }, []);
  const points = (id: number | null) =>
    trend.data?.series.find((s) => s.signal === id)?.points ?? [];
  const left = 52;
  const right = 52;
  const plotW = width - left - right;
  const top = 10;
  const upperH = 170;
  const lowerTop = top + upperH + 24;
  const lowerH = 90;
  const height = lowerTop + lowerH + 22;
  const x = (at: bigint) =>
    left + (Number(at - from) / Number(span || 1n)) * plotW;
  // SP and PV share one scale with room around both; MV uses its limits.
  const measured = [...points(sp), ...points(pv)].map((p) => numeric(p.value));
  const lowY = measured.length ? Math.min(...measured) : 0;
  const highY = measured.length ? Math.max(...measured) : 100;
  const pad = Math.max(1, (highY - lowY) * 0.1);
  const [y0, y1] = [lowY - pad, highY + pad];
  const [m0, m1] = limits[1] > limits[0] ? limits : [0, 100];
  const yUpper = (v: number) => top + upperH - ((v - y0) / (y1 - y0)) * upperH;
  const yLower = (v: number) =>
    lowerTop + lowerH - ((v - m0) / (m1 - m0)) * lowerH;
  const path = (id: number | null, y: (v: number) => number) => {
    const ps = points(id);
    let d = "";
    ps.forEach((p, i) => {
      const px = Math.max(left, x(BigInt(p.t)));
      const py = y(numeric(p.value));
      d += i === 0 ? `M ${px} ${py}` : ` H ${px} V ${py}`;
    });
    if (ps.length) d += ` H ${left + plotW}`;
    return d;
  };
  const empty = ids.every((id) => points(id).length === 0);
  return (
    <div>
      <div className="row" style={{ padding: "4px 0", gap: 12 }}>
        <div className="segmented" role="group" aria-label={t("tune.window")}>
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
        <span className="legend sp">{t("tune.legend.sp")}</span>
        <span className="legend pv">{t("tune.legend.pv")}</span>
        <span className="legend mv">{t("tune.legend.mv")}</span>
        {empty && <span className="muted">{t("tune.noData")}</span>}
      </div>
      <div ref={box}>
        <svg
          className="wave tuning-trend"
          width={width}
          height={height}
          role="img"
          aria-label={t("tune.chart", { loop: name })}
          data-points={ids.map((id) => points(id).length).join(",")}
        >
          {[0, 0.25, 0.5, 0.75, 1].map((f) => (
            <g key={f}>
              <line
                className="grid"
                x1={left + f * plotW}
                x2={left + f * plotW}
                y1={top}
                y2={lowerTop + lowerH}
              />
              <text x={left + f * plotW} y={height - 6} textAnchor="middle">
                {seconds(from + BigInt(Math.round(f * Number(span))))}
              </text>
            </g>
          ))}
          {[y0, (y0 + y1) / 2, y1].map((v) => (
            <text key={`u${v}`} x={left - 4} y={yUpper(v) + 3} textAnchor="end">
              {v.toFixed(1)}
            </text>
          ))}
          {[m0, m1].map((v) => (
            <g key={`m${v}`}>
              <line
                className="grid"
                x1={left}
                x2={left + plotW}
                y1={yLower(v)}
                y2={yLower(v)}
              />
              <text x={left + plotW + 4} y={yLower(v) + 3}>
                {v.toFixed(0)}
              </text>
            </g>
          ))}
          <path className="trace-sp" d={path(sp, yUpper)} />
          <path className="trace-num" d={path(pv, yUpper)} />
          <path className="trace-mv" d={path(mv, yLower)} />
        </svg>
      </div>
    </div>
  );
}
