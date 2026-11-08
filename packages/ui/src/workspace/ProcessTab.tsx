/**
 * HMI-style process view. Every value shown comes from the authoritative frame; the view
 * only maps quantities to symbols and turns operator gestures into operate requests.
 */
import {
  api,
  type ComponentDto,
  type LayoutDto,
  type SignalValueDto,
} from "@plc-lab/ipc";
import { formatQuantity } from "../format";
import { processLabel } from "../locale/messages";
import { useInterface } from "../locale";
import { AlarmSummary } from "./AlarmsTab";
import { sequenceAlarmLabel } from "./alarmText";
import { useSequenceDefinition } from "./sequenceDefinition";
import type { Selection } from "./ProjectTree";
import type { Lab } from "./useLab";

type Q = Record<string, SignalValueDto>;
const LAMP: Record<string, string> = {
  green: "var(--lamp-green)",
  red: "var(--lamp-red)",
  amber: "var(--lamp-amber)",
  white: "var(--lamp-white)",
  blue: "var(--lamp-blue)",
};

function b(q: Q, name: string): boolean {
  const v = q[name];
  return v?.type === "BOOL" && v.value;
}
function n(q: Q, name: string): number {
  const v = q[name];
  return v?.type === "COUNT" || v?.type === "INT" || v?.type === "REAL"
    ? v.value
    : 0;
}

export function ProcessTab({
  lab,
  selection,
  select,
  troubleshooting,
}: {
  lab: Lab;
  selection: Selection;
  select: (s: Selection) => void;
  troubleshooting: boolean;
}) {
  const { t } = useInterface();
  const sequence = useSequenceDefinition(lab);
  const { workspace, frame, ws } = lab;
  if (!workspace || !frame || !ws) return null;
  const states = new Map(
    frame.components.map((c) => [
      c.id,
      Object.fromEntries(c.quantities.map((q) => [q.name, q.value])) as Q,
    ]),
  );
  const layout = new Map(workspace.scene.layout.map((l) => [l.component, l]));
  let fallback = 0;
  const place = (c: ComponentDto): LayoutDto =>
    layout.get(c.id) ?? {
      component: c.id,
      x: 80 + 110 * fallback++,
      y: 560,
      width: null,
      height: null,
    };
  const center = (l: LayoutDto) => ({
    x: l.x + (l.width ?? 0) / 2,
    y: l.y + (l.height ?? 0) / 2,
  });
  const operate = (id: string, action: Parameters<typeof api.operate>[2]) =>
    void lab.act(
      "operate",
      () => api.operate(ws, id, action),
      (f) => f,
    );
  return (
    <div
      style={{
        position: "absolute",
        inset: 0,
        display: "flex",
        flexDirection: "column",
      }}
    >
      {frame.alarms.length > 0 && (
        <div
          className="row wrap"
          style={{ padding: "6px 10px", borderBottom: "1px solid var(--line)" }}
          role="status"
        >
          <AlarmSummary
            alarms={frame.alarms}
            label={sequenceAlarmLabel(sequence)}
          />
        </div>
      )}
      <svg
        className="hmi"
        viewBox="0 0 1000 620"
        preserveAspectRatio="xMidYMid meet"
        role="img"
        aria-label={t("process.view", { title: workspace.scene.title })}
        style={{ flex: 1 }}
      >
        {workspace.scene.couplings.map((c) => {
          const from = layout.get(c.from);
          const to = layout.get(c.to);
          if (!from || !to) return null;
          const a = center(from);
          const t = center(to);
          const flowing =
            c.kind === "pumpDrainsTank"
              ? b(states.get(c.to) ?? {}, "running") &&
                !b(states.get(c.to) ?? {}, "dry")
              : b(states.get(c.from) ?? {}, "flowing");
          const bottom = from.y + (from.height ?? 0);
          const path =
            c.kind === "valveFillsTank"
              ? `M ${a.x + 26} ${a.y} H ${t.x} V ${to.y + 4}`
              : `M ${a.x} ${bottom} V ${Math.max(bottom + 20, t.y)} H ${t.x - 26}`;
          return (
            <path
              key={c.from + c.to}
              className={`pipe${flowing ? " flow" : ""}`}
              d={path}
            />
          );
        })}
        {workspace.scene.components.map((c) => {
          const l = place(c);
          const q = states.get(c.id) ?? {};
          const selected =
            selection?.kind === "component" && selection.id === c.id;
          return (
            <g
              key={c.id}
              className={`${c.operable ? "operable" : ""}${selected ? " selected" : ""}`}
              onClick={() => select({ kind: "component", id: c.id })}
            >
              <Symbol
                c={c}
                l={l}
                q={q}
                operate={operate}
                config={Object.fromEntries(
                  c.config.map((k) => [k.key, k.value]),
                )}
                layout={layout}
              />
            </g>
          );
        })}
        {troubleshooting && (
          <text x={990} y={612} textAnchor="end" className="label">
            {t("process.fieldView")}
          </text>
        )}
      </svg>
    </div>
  );
}

function Symbol({
  c,
  l,
  q,
  operate,
  config,
  layout,
}: {
  c: ComponentDto;
  l: LayoutDto;
  q: Q;
  operate: (id: string, action: Parameters<typeof api.operate>[2]) => void;
  config: Record<string, string>;
  layout: Map<string, LayoutDto>;
}) {
  const { t } = useInterface();
  const label = (dy: number, x = l.x) => (
    <text x={x} y={l.y + dy} textAnchor="middle" className="label">
      {c.label}
    </text>
  );
  switch (c.kind) {
    case "pushButton": {
      const pressed = b(q, "pressed");
      const color = LAMP[c.color ?? "white"] ?? "var(--symbol-stroke)";
      const press = () => operate(c.id, { type: "press" });
      const release = () => operate(c.id, { type: "release" });
      return (
        <g
          role="button"
          tabIndex={0}
          aria-label={t("process.hold", { label: c.label })}
          aria-pressed={pressed}
          onPointerDown={(e) => {
            (e.target as Element).setPointerCapture?.(e.pointerId);
            press();
          }}
          onPointerUp={release}
          onKeyDown={(e) =>
            (e.key === " " || e.key === "Enter") && !e.repeat && press()
          }
          onKeyUp={(e) => (e.key === " " || e.key === "Enter") && release()}
        >
          <rect
            x={l.x - 30}
            y={l.y - 30}
            width={60}
            height={60}
            rx={6}
            className="symbol"
          />
          <circle
            cx={l.x}
            cy={l.y}
            r={pressed ? 19 : 22}
            style={{ fill: color, stroke: "var(--svg-shadow)" }}
            opacity={pressed ? 1 : 0.75}
            strokeWidth={pressed ? 4 : 2}
          />
          <text x={l.x} y={l.y + 46} textAnchor="middle" className="label">
            {c.label}
          </text>
          <text
            x={l.x}
            y={l.y + 60}
            textAnchor="middle"
            className="label"
            style={{ fontSize: 10 }}
          >
            {t(
              config.contact === "normallyClosed"
                ? "process.ncContact"
                : "process.noContact",
            )}
          </text>
        </g>
      );
    }
    case "selectorSwitch": {
      const pos = n(q, "position");
      const count = Math.max(1, c.positions.length);
      const angle = count === 1 ? 0 : -45 + (90 * pos) / (count - 1);
      return (
        <g
          role="button"
          tabIndex={0}
          aria-label={`${c.label}: ${c.positions[pos] ?? pos}`}
          onClick={() =>
            operate(c.id, { type: "selector", position: (pos + 1) % count })
          }
        >
          <circle cx={l.x} cy={l.y} r={26} className="symbol" />
          <rect
            x={l.x - 4}
            y={l.y - 22}
            width={8}
            height={28}
            rx={3}
            style={{ fill: "var(--hmi-cap)" }}
            transform={`rotate(${angle} ${l.x} ${l.y})`}
          />
          {c.positions.map((p, i) => {
            const a =
              ((count === 1 ? 0 : -45 + (90 * i) / (count - 1)) - 90) *
              (Math.PI / 180);
            return (
              <text
                key={p}
                x={l.x + Math.cos(a) * 40}
                y={l.y + Math.sin(a) * 40 + 4}
                textAnchor="middle"
                className="label"
                style={{ fontWeight: i === pos ? 700 : 400 }}
              >
                {p}
              </text>
            );
          })}
          {label(48)}
        </g>
      );
    }
    case "pilotLamp": {
      const lit = b(q, "lit");
      const color = LAMP[c.color ?? "white"] ?? "var(--lamp-white)";
      return (
        <g
          aria-label={t(lit ? "process.lampOn" : "process.lampOff", {
            label: c.label,
          })}
        >
          <circle cx={l.x} cy={l.y} r={22} className="symbol" />
          <circle
            cx={l.x}
            cy={l.y}
            r={16}
            style={
              lit
                ? { fill: color, filter: `drop-shadow(0 0 8px ${color})` }
                : { fill: "var(--hmi-off)" }
            }
          />
          {label(46)}
        </g>
      );
    }
    case "motorStarter": {
      const running = b(q, "running");
      const tripped = b(q, "tripped");
      const w = l.width ?? 200;
      const h = l.height ?? 140;
      const cx = l.x + w / 2;
      const cy = l.y + h / 2 - 10;
      return (
        <g
          aria-label={`${t(running ? "process.running" : "process.stopped", { label: c.label })}${tripped ? t("process.overloadSuffix") : ""}`}
        >
          <rect
            x={l.x}
            y={l.y}
            width={w}
            height={h}
            rx={4}
            fill="none"
            style={{ stroke: "var(--hmi-edge)" }}
          />
          <circle
            cx={cx}
            cy={cy}
            r={34}
            className={`symbol${running ? " on" : ""}${tripped ? " fault" : ""}`}
          />
          <text
            x={cx}
            y={cy + 6}
            textAnchor="middle"
            style={{ fontSize: 20, fontWeight: 700 }}
          >
            M
          </text>
          <rect
            x={l.x + 16}
            y={l.y + h - 34}
            width={w - 32}
            height={6}
            rx={3}
            style={{ fill: "var(--hmi-panel)" }}
          />
          <rect
            x={l.x + 16}
            y={l.y + h - 34}
            width={((w - 32) * n(q, "speedPermille")) / 1000}
            height={6}
            rx={3}
            style={{ fill: "var(--st-active)" }}
          />
          <text x={l.x + 16} y={l.y + h - 12} className="value-text">
            {t("process.speed", {
              value: (n(q, "speedPermille") / 10).toFixed(0),
              current: formatQuantity(n(q, "currentMa"), "mA"),
            })}
          </text>
          <text x={l.x + 8} y={l.y + 16} className="label">
            K1 {b(q, "command") ? "energized" : "de-energized"}
          </text>
          {tripped && (
            <text
              x={l.x + w - 8}
              y={l.y + 16}
              textAnchor="end"
              style={{ fill: "var(--st-fault)", fontWeight: 700, fontSize: 11 }}
            >
              {t("process.overload")}
            </text>
          )}
          {label(h + 16, cx)}
        </g>
      );
    }
    case "onOffValve": {
      const pos = n(q, "positionPermille");
      const open = pos >= 1000;
      const color = pos > 0 ? "var(--st-active)" : "var(--symbol-stroke)";
      return (
        <g
          aria-label={t("process.valveOpen", {
            label: c.label,
            percent: (pos / 10).toFixed(0),
          })}
        >
          <path
            d={`M ${l.x - 24} ${l.y - 14} L ${l.x + 24} ${l.y + 14} L ${l.x + 24} ${l.y - 14} L ${l.x - 24} ${l.y + 14} Z`}
            style={{ fill: "var(--symbol-fill)", stroke: color }}
            strokeWidth={1.8}
          />
          <line
            x1={l.x}
            y1={l.y}
            x2={l.x}
            y2={l.y - 26}
            style={{ stroke: "var(--symbol-stroke)" }}
            strokeWidth={1.6}
          />
          <rect
            x={l.x - 12}
            y={l.y - 38}
            width={24}
            height={12}
            style={{
              fill: b(q, "command")
                ? "var(--primary-bg)"
                : "var(--symbol-fill)",
              stroke: "var(--symbol-stroke)",
            }}
          />
          <text x={l.x} y={l.y + 34} textAnchor="middle" className="value-text">
            {open
              ? "OPEN"
              : pos === 0
                ? "CLOSED"
                : `${(pos / 10).toFixed(0)} %`}
          </text>
          {label(50)}
        </g>
      );
    }
    case "pump": {
      const running = b(q, "running");
      const dry = b(q, "dry");
      return (
        <g
          aria-label={`${t(running ? "process.running" : "process.stopped", { label: c.label })}${dry ? t("process.drySuffix") : ""}`}
        >
          <circle
            cx={l.x}
            cy={l.y}
            r={26}
            className={`symbol${running ? " on" : ""}${dry ? " fault" : ""}`}
          />
          <path
            d={`M ${l.x - 12} ${l.y - 18} L ${l.x + 22} ${l.y} L ${l.x - 12} ${l.y + 18}`}
            fill="none"
            style={{
              stroke: running ? "var(--st-active)" : "var(--symbol-stroke)",
            }}
            strokeWidth={1.6}
          />
          <text x={l.x} y={l.y + 44} textAnchor="middle" className="value-text">
            {formatQuantity(n(q, "flowMlPerS"), "ml/s")}
          </text>
          {label(60)}
        </g>
      );
    }
    case "tank": {
      const w = l.width ?? 200;
      const h = l.height ?? 260;
      const level = Math.min(1000, n(q, "levelPermille"));
      const fill = (h - 4) * (level / 1000);
      const cap = Number(config.capacityMl ?? n(q, "capacityMl")) || 1;
      const mark = (ml: string | undefined, text: string) => {
        if (!ml) return null;
        const y = l.y + h - (h * Number(ml)) / cap;
        return (
          <g>
            <line
              x1={l.x - 8}
              y1={y}
              x2={l.x}
              y2={y}
              style={{ stroke: "var(--st-warning)" }}
            />
            <text x={l.x - 12} y={y + 4} textAnchor="end" className="label">
              {text}
            </text>
          </g>
        );
      };
      return (
        <g aria-label={`${c.label}: ${(level / 10).toFixed(1)} %`}>
          <rect
            x={l.x}
            y={l.y}
            width={w}
            height={h}
            rx={6}
            className={`symbol${b(q, "overflowing") ? " fault" : ""}`}
          />
          <rect
            x={l.x + 2}
            y={l.y + h - 2 - fill}
            width={w - 4}
            height={fill}
            rx={4}
            className="fluid"
          />
          {mark(config.highHighMl, "HH")}
          {mark(config.lowLowMl, "LL")}
          <text
            x={l.x + w / 2}
            y={l.y + 28}
            textAnchor="middle"
            className="value-text"
            style={{ fontSize: 18 }}
          >
            {(level / 10).toFixed(1)} %
          </text>
          <text
            x={l.x + w / 2}
            y={l.y + 46}
            textAnchor="middle"
            className="label"
          >
            {formatQuantity(n(q, "levelMl"), "ml")} of{" "}
            {formatQuantity(cap, "ml")}
          </text>
          {n(q, "spilledMl") > 0 && (
            <text
              x={l.x + w / 2}
              y={l.y + h + 32}
              textAnchor="middle"
              style={{ fill: "var(--st-fault)", fontSize: 11 }}
            >
              {t("process.spilled", {
                amount: formatQuantity(n(q, "spilledMl"), "ml"),
              })}
            </text>
          )}
          {label(h + 16, l.x + w / 2)}
        </g>
      );
    }
    case "levelSwitch": {
      const conducting = b(q, "conducting");
      const tank = config.tank ? layout.get(config.tank) : undefined;
      return (
        <g
          aria-label={t(
            conducting ? "process.contactClosed" : "process.contactOpen",
            { label: c.label },
          )}
        >
          {tank && (
            <line
              x1={tank.x + (tank.width ?? 0)}
              y1={l.y}
              x2={l.x - 14}
              y2={l.y}
              style={{ stroke: "var(--pipe)" }}
              strokeWidth={2}
            />
          )}
          <rect
            x={l.x - 14}
            y={l.y - 10}
            width={28}
            height={20}
            rx={3}
            className={`symbol${conducting ? " on" : ""}`}
          />
          <circle
            cx={l.x + 22}
            cy={l.y}
            r={4}
            style={{
              fill: conducting ? "var(--st-active)" : "var(--hmi-off)",
            }}
          />
          <text x={l.x + 32} y={l.y + 4} className="label">
            {c.label}
          </text>
          <text
            x={l.x + 32}
            y={l.y + 18}
            className="label"
            style={{ fontSize: 10 }}
          >
            {config.contact === "normallyClosed" ? "NC" : "NO"} ·{" "}
            {(config.output ?? "").toUpperCase()}
          </text>
        </g>
      );
    }
    case "levelTransmitter": {
      const signal = n(q, "signal");
      const voltage = config.signal === "voltage0To10V";
      return (
        <g
          aria-label={`${c.label}: ${formatQuantity(signal, voltage ? "mV" : "µA")}`}
        >
          <circle cx={l.x} cy={l.y} r={22} className="symbol" />
          <text
            x={l.x}
            y={l.y + 4}
            textAnchor="middle"
            style={{ fontSize: 11, fontWeight: 700 }}
          >
            LT
          </text>
          <text x={l.x} y={l.y + 40} textAnchor="middle" className="value-text">
            {formatQuantity(signal, voltage ? "mV" : "µA")}
          </text>
          {label(56)}
        </g>
      );
    }
    case "presenceSensor": {
      const present = b(q, "present");
      const conducting = b(q, "conducting");
      return (
        <g
          role="button"
          tabIndex={0}
          aria-label={t(
            present ? "process.targetPresent" : "process.targetAbsent",
            { label: c.label },
          )}
          onClick={() => operate(c.id, { type: "presence", present: !present })}
        >
          <rect
            x={l.x - 20}
            y={l.y - 14}
            width={40}
            height={28}
            rx={4}
            className={`symbol${conducting ? " on" : ""}`}
          />
          <line
            x1={l.x + 20}
            y1={l.y}
            x2={l.x + 120}
            y2={l.y}
            style={{
              stroke: present ? "var(--st-warning)" : "var(--hmi-beam-off)",
            }}
            strokeDasharray="5 4"
          />
          <rect
            x={l.x + 70}
            y={l.y + 6}
            width={40}
            height={30}
            rx={3}
            style={{
              fill: present ? "var(--hmi-target)" : "transparent",
              stroke: present ? "var(--metal)" : "var(--hmi-edge)",
            }}
            strokeDasharray={present ? undefined : "3 3"}
          />
          <circle
            cx={l.x - 26}
            cy={l.y}
            r={4}
            style={{
              fill: conducting ? "var(--st-warning)" : "var(--hmi-off)",
            }}
          />
          {label(40)}
        </g>
      );
    }
    case "variableFrequencyDrive": {
      const running = b(q, "running");
      const remote = b(q, "remote");
      const w = l.width ?? 240;
      const h = l.height ?? 160;
      const speed = n(q, "speedPermille");
      // Generic VFD behavioral model: display state as the drive shows it.
      const state = !b(q, "supplied")
        ? "OFF"
        : b(q, "tripped")
          ? "FLT"
          : running
            ? "RUN"
            : b(q, "ready")
              ? "RDY"
              : "---";
      const reference = n(q, "referencePermille");
      return (
        <g
          aria-label={t("process.driveLabel", {
            label: c.label,
            state: t(running ? "process.driveRunning" : "process.driveStopped"),
            place: t(remote ? "process.driveRemote" : "process.driveLocal"),
            ready: t(
              b(q, "tripped")
                ? "process.driveFaulted"
                : b(q, "ready")
                  ? "process.driveReady"
                  : "process.driveNotReady",
            ),
          })}
        >
          <rect
            x={l.x}
            y={l.y}
            width={w * 0.42}
            height={h}
            rx={4}
            className="symbol"
          />
          <rect
            x={l.x + 10}
            y={l.y + 12}
            width={w * 0.42 - 20}
            height={44}
            rx={2}
            style={{ fill: "var(--display-bg)", stroke: "var(--hmi-edge)" }}
          />
          <text
            x={l.x + 16}
            y={l.y + 30}
            className="value-text"
            style={{ fontSize: 11, fill: "var(--display-text)" }}
          >
            {remote ? "REM" : "LOC"} {state}
          </text>
          <text
            x={l.x + 16}
            y={l.y + 48}
            className="value-text"
            style={{ fontSize: 11, fill: "var(--display-text)" }}
          >
            {t("process.drive", {
              hz: ((speed * 50) / 1000).toFixed(1),
              ref: ((reference * 50) / 1000).toFixed(1),
            })}
            {b(q, "keypadReference") ? " KP" : ""}
          </text>
          <text
            x={l.x + 10}
            y={l.y + 74}
            className="label"
            style={{ fontSize: 10 }}
          >
            {b(q, "command") ? "run terminal ON" : "run terminal OFF"}
          </text>
          <text
            x={l.x + 10}
            y={l.y + 90}
            className="label"
            style={{ fontSize: 10 }}
          >
            {formatQuantity(n(q, "currentMa"), "mA")}
          </text>
          <line
            x1={l.x + w * 0.42}
            y1={l.y + h / 2}
            x2={l.x + w * 0.62}
            y2={l.y + h / 2}
            className="pipe"
            style={{ strokeWidth: 3 }}
          />
          <circle
            cx={l.x + w * 0.8}
            cy={l.y + h / 2}
            r={Math.min(34, w * 0.17)}
            className={`symbol${running ? " on" : ""}`}
          />
          <text
            x={l.x + w * 0.8}
            y={l.y + h / 2 + 6}
            textAnchor="middle"
            style={{ fontSize: 18, fontWeight: 700 }}
          >
            M
          </text>
          {label(h + 16, l.x + w / 2)}
        </g>
      );
    }
    default:
      return (
        <g>
          <rect
            x={l.x - 30}
            y={l.y - 20}
            width={60}
            height={40}
            className="symbol"
          />
          <text x={l.x} y={l.y + 4} textAnchor="middle" className="label">
            {processLabel(t, "kind", c.kind)}
          </text>
          {label(36)}
        </g>
      );
  }
}
