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
import { formatQuantity, humanize } from "../format";
import type { Selection } from "./ProjectTree";
import type { Lab } from "./useLab";

type Q = Record<string, SignalValueDto>;
const LAMP: Record<string, string> = {
  green: "#3dbb8a",
  red: "#e5534b",
  amber: "#e2a43a",
  white: "#e8edf1",
  blue: "#4f9be0",
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
  const byId = new Map(workspace.scene.components.map((c) => [c.id, c]));
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
          {frame.alarms.map((a) => (
            <span
              key={a.component + a.kind}
              className={`badge ${a.kind === "OVERFLOW" || a.kind === "DRY_RUN" || a.kind === "OVERLOAD" ? "fault" : "warning"}`}
            >
              {byId.get(a.component)?.label ?? a.component}: {a.text}
            </span>
          ))}
        </div>
      )}
      <svg
        className="hmi"
        viewBox="0 0 1000 620"
        preserveAspectRatio="xMidYMid meet"
        role="img"
        aria-label={`Process view: ${workspace.scene.title}`}
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
            Field view: what you would see and hear at the machine
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
  const label = (dy: number, x = l.x) => (
    <text x={x} y={l.y + dy} textAnchor="middle" className="label">
      {c.label}
    </text>
  );
  switch (c.kind) {
    case "pushButton": {
      const pressed = b(q, "pressed");
      const color = LAMP[c.color ?? "white"] ?? "#8a98a5";
      const press = () => operate(c.id, { type: "press" });
      const release = () => operate(c.id, { type: "release" });
      return (
        <g
          role="button"
          tabIndex={0}
          aria-label={`${c.label} (hold to press)`}
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
            fill={color}
            opacity={pressed ? 1 : 0.75}
            stroke="#0008"
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
            {config.contact === "normallyClosed" ? "NC contact" : "NO contact"}
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
            fill="#c9d2da"
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
      const color = LAMP[c.color ?? "white"] ?? "#e8edf1";
      return (
        <g aria-label={`${c.label}: ${lit ? "on" : "off"}`}>
          <circle cx={l.x} cy={l.y} r={22} className="symbol" />
          <circle
            cx={l.x}
            cy={l.y}
            r={16}
            fill={lit ? color : "#2a3138"}
            style={
              lit ? { filter: `drop-shadow(0 0 8px ${color})` } : undefined
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
          aria-label={`${c.label}: ${running ? "running" : "stopped"}${tripped ? ", overload tripped" : ""}`}
        >
          <rect
            x={l.x}
            y={l.y}
            width={w}
            height={h}
            rx={4}
            fill="none"
            stroke="#2a3540"
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
            fill="#232c35"
          />
          <rect
            x={l.x + 16}
            y={l.y + h - 34}
            width={((w - 32) * n(q, "speedPermille")) / 1000}
            height={6}
            rx={3}
            fill="#3dbb8a"
          />
          <text x={l.x + 16} y={l.y + h - 12} className="value-text">
            {(n(q, "speedPermille") / 10).toFixed(0)} % speed ·{" "}
            {formatQuantity(n(q, "currentMa"), "mA")}
          </text>
          <text x={l.x + 8} y={l.y + 16} className="label">
            K1 {b(q, "command") ? "energized" : "de-energized"}
          </text>
          {tripped && (
            <text
              x={l.x + w - 8}
              y={l.y + 16}
              textAnchor="end"
              style={{ fill: "#e5534b", fontWeight: 700, fontSize: 11 }}
            >
              OVERLOAD
            </text>
          )}
          {label(h + 16, cx)}
        </g>
      );
    }
    case "onOffValve": {
      const pos = n(q, "positionPermille");
      const open = pos >= 1000;
      const color = pos > 0 ? "#3dbb8a" : "#8a98a5";
      return (
        <g aria-label={`${c.label}: ${(pos / 10).toFixed(0)} % open`}>
          <path
            d={`M ${l.x - 24} ${l.y - 14} L ${l.x + 24} ${l.y + 14} L ${l.x + 24} ${l.y - 14} L ${l.x - 24} ${l.y + 14} Z`}
            fill="#1e262e"
            stroke={color}
            strokeWidth={1.8}
          />
          <line
            x1={l.x}
            y1={l.y}
            x2={l.x}
            y2={l.y - 26}
            stroke="#8a98a5"
            strokeWidth={1.6}
          />
          <rect
            x={l.x - 12}
            y={l.y - 38}
            width={24}
            height={12}
            fill={b(q, "command") ? "#2b6aa5" : "#1e262e"}
            stroke="#8a98a5"
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
          aria-label={`${c.label}: ${running ? "running" : "stopped"}${dry ? ", running dry" : ""}`}
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
            stroke={running ? "#3dbb8a" : "#8a98a5"}
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
            <line x1={l.x - 8} y1={y} x2={l.x} y2={y} stroke="#e2a43a" />
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
              style={{ fill: "#e5534b", fontSize: 11 }}
            >
              spilled {formatQuantity(n(q, "spilledMl"), "ml")}
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
        <g aria-label={`${c.label}: contact ${conducting ? "closed" : "open"}`}>
          {tank && (
            <line
              x1={tank.x + (tank.width ?? 0)}
              y1={l.y}
              x2={l.x - 14}
              y2={l.y}
              stroke="#4a5866"
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
            fill={conducting ? "#3dbb8a" : "#2a3138"}
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
          aria-label={`${c.label}: target ${present ? "present" : "absent"} (click to change)`}
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
            stroke={present ? "#e2a43a" : "#3a4652"}
            strokeDasharray="5 4"
          />
          <rect
            x={l.x + 70}
            y={l.y + 6}
            width={40}
            height={30}
            rx={3}
            fill={present ? "#4b5a68" : "transparent"}
            stroke={present ? "#7d8a96" : "#2a3540"}
            strokeDasharray={present ? undefined : "3 3"}
          />
          <circle
            cx={l.x - 26}
            cy={l.y}
            r={4}
            fill={conducting ? "#e2a43a" : "#2a3138"}
          />
          {label(40)}
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
            {humanize(c.kind)}
          </text>
          {label(36)}
        </g>
      );
  }
}
