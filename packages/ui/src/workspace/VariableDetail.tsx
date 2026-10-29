/**
 * Online detail of one canonical signal: identity, the runtime's values (scan input,
 * committed, next input), quality, force state with force controls, the I/O binding from
 * the PLC symbol to the device on the drawing, and its recent changes from the trace.
 * Everything comes from backend projections; nothing is evaluated here.
 */
import {
  api,
  type HardwareDto,
  type TrendDto,
  type WatchDto,
} from "@plc-lab/ipc";
import { formatValue, seconds, valueClass } from "../format";
import { Badge } from "../ui";
import { ForceCell } from "./Dock";
import { useFrameQuery } from "./useFrameQuery";
import type { Lab } from "./useLab";

const KIND: Record<string, string> = {
  PLC_INPUT: "PLC input",
  PLC_OUTPUT: "PLC output",
  PLC_LOCAL: "PLC variable",
  BLOCK_OUTPUT: "Block output",
  PHYSICAL: "Process value",
  CHANNEL: "I/O channel",
};
const HISTORY_US = 20_000_000n;

export function VariableDetail({ lab, signal }: { lab: Lab; signal: number }) {
  const ws = lab.ws;
  const descriptor = lab.signals.find((s) => s.id === signal) ?? null;
  const symbol = descriptor?.path.startsWith("plc.")
    ? descriptor.path.slice(4)
    : null;
  const live = useFrameQuery<WatchDto | null>(
    lab,
    () => (ws ? api.watch(ws, [signal]) : Promise.resolve(null)),
    [signal],
  );
  const now = BigInt(lab.frame?.worldTimeUs ?? "0");
  const from = now > HISTORY_US ? now - HISTORY_US : 0n;
  const history = useFrameQuery<TrendDto | null>(
    lab,
    () =>
      ws
        ? api.trend(ws, [signal], String(from), String(now), 400)
        : Promise.resolve(null),
    [signal],
  );
  const hardware = useFrameQuery<HardwareDto | null>(
    lab,
    () => (ws && symbol ? api.hardware(ws) : Promise.resolve(null)),
    [symbol],
  );
  if (!descriptor)
    return (
      <div className="inspector-section muted">
        This signal is not part of the loaded program or station.
      </div>
    );
  const e = live.data?.rows[0] ?? null;
  const binding = symbol
    ? (hardware.data?.signals.find((s) => s.tag === symbol) ?? null)
    : null;
  // Value changes, newest first, as recorded by the backend's trace.
  const changes = (history.data?.series ?? [])
    .flatMap((series) =>
      series.points
        .filter(
          (p, i) =>
            i === 0 ||
            JSON.stringify(p.value) !==
              JSON.stringify(series.points[i - 1]!.value),
        )
        .map((p) => ({ layer: series.layer, ...p })),
    )
    .sort((a, b) => (BigInt(b.t) > BigInt(a.t) ? 1 : -1))
    .slice(0, 10);
  return (
    <div className="inspector-section" aria-label="Online variable">
      <h4>{descriptor.label}</h4>
      <dl className="kv">
        <dt>Canonical id</dt>
        <dd className="mono">{descriptor.path}</dd>
        <dt>Kind</dt>
        <dd>{KIND[descriptor.kind] ?? descriptor.kind}</dd>
        <dt>Type</dt>
        <dd>
          {descriptor.valueType}
          {descriptor.unit ? ` (${descriptor.unit})` : ""}
        </dd>
        <dt>Value</dt>
        <dd className={valueClass(e?.value)}>
          {formatValue(e?.value, descriptor.unit)}
        </dd>
        {e?.scanValue && (
          <>
            <dt>Scan input</dt>
            <dd>{formatValue(e.scanValue)}</dd>
          </>
        )}
        {e?.committed && (
          <>
            <dt>Committed</dt>
            <dd>{formatValue(e.committed)}</dd>
          </>
        )}
        {e?.staged && (
          <>
            <dt>Next input</dt>
            <dd>{formatValue(e.staged)}</dd>
          </>
        )}
        <dt>Quality</dt>
        <dd>
          {!e?.quality ? (
            "—"
          ) : e.quality === "GOOD" ? (
            "good"
          ) : (
            <Badge kind="bad">
              {e.quality.replaceAll("_", " ").toLowerCase()}
            </Badge>
          )}
        </dd>
        <dt>Source</dt>
        <dd>{e?.source ?? "—"}</dd>
        <dt>Force</dt>
        <dd>
          {descriptor.kind === "PLC_INPUT" ||
          descriptor.kind === "PLC_OUTPUT" ? (
            <ForceCell
              lab={lab}
              signal={signal}
              kind={descriptor.kind}
              valueType={descriptor.valueType}
              forced={e?.forced ?? null}
            />
          ) : (
            <span className="muted">
              only PLC inputs and outputs can be forced
            </span>
          )}
        </dd>
      </dl>
      {binding && (
        <>
          <h4 style={{ marginTop: 12 }}>I/O binding</h4>
          <dl className="kv">
            <dt>Channel</dt>
            <dd className="mono">{binding.channelId ?? "—"}</dd>
            <dt>Address</dt>
            <dd className="mono">{binding.address}</dd>
            <dt>Device on the drawing</dt>
            <dd>
              {binding.drawnDevice
                ? `${binding.drawnDevice} (${binding.drawnPort ?? ""})`
                : "not on the drawing"}
            </dd>
            <dt>Device state</dt>
            <dd>{binding.deviceState ?? "—"}</dd>
          </dl>
        </>
      )}
      <h4 style={{ marginTop: 12 }}>Recent changes</h4>
      {changes.length === 0 ? (
        <p className="muted">No change in the last 20 s of the trace.</p>
      ) : (
        <table className="table" aria-label="Recent changes">
          <tbody>
            {changes.map((c) => (
              <tr key={`${c.layer}-${c.t}`}>
                <td className="mono">{seconds(c.t)}</td>
                <td className="dim">{c.layer.toLowerCase()}</td>
                <td className={valueClass(c.value)}>
                  {formatValue(c.value, descriptor.unit)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
