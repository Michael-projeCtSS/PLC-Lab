/**
 * Online detail of one canonical signal: identity, the runtime's values (scan input,
 * committed, next input), quality, force state with force controls, the I/O binding from
 * the PLC symbol to the device on the drawing, its retention and what the latest start
 * did to it (ADR-0038), and its recent changes from the trace. Everything comes from
 * backend projections; nothing is evaluated here.
 */
import {
  api,
  type HardwareDto,
  type TrendDto,
  type WatchDto,
} from "@plc-lab/ipc";
import { formatValue, seconds, valueClass } from "../format";
import { useInterface } from "../locale";
import type { TranslationKey } from "../locale/en";
import { Badge } from "../ui";
import { ForceCell } from "./Dock";
import { useFrameQuery } from "./useFrameQuery";
import type { Lab } from "./useLab";

const KIND: Record<string, TranslationKey> = {
  PLC_INPUT: "var.kind.PLC_INPUT",
  PLC_OUTPUT: "var.kind.PLC_OUTPUT",
  PLC_LOCAL: "var.kind.PLC_LOCAL",
  BLOCK_OUTPUT: "var.kind.BLOCK_OUTPUT",
  PHYSICAL: "var.kind.PHYSICAL",
  CHANNEL: "var.kind.CHANNEL",
};
const HISTORY_US = 20_000_000n;

export function VariableDetail({ lab, signal }: { lab: Lab; signal: number }) {
  const { t } = useInterface();
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
    return <div className="inspector-section muted">{t("var.missing")}</div>;
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
    <div className="inspector-section" aria-label={t("var.title")}>
      <h4>{descriptor.label}</h4>
      <dl className="kv">
        <dt>{t("var.id")}</dt>
        <dd className="mono">{descriptor.path}</dd>
        <dt>{t("var.kind")}</dt>
        <dd>
          {KIND[descriptor.kind] ? t(KIND[descriptor.kind]!) : descriptor.kind}
        </dd>
        <dt>{t("var.type")}</dt>
        <dd>
          {descriptor.valueType}
          {descriptor.unit ? ` (${descriptor.unit})` : ""}
        </dd>
        <dt>{t("var.value")}</dt>
        <dd className={valueClass(e?.value)}>
          {formatValue(e?.value, descriptor.unit)}
        </dd>
        {e?.scanValue && (
          <>
            <dt>{t("var.scan")}</dt>
            <dd>{formatValue(e.scanValue)}</dd>
          </>
        )}
        {e?.committed && (
          <>
            <dt>{t("var.committed")}</dt>
            <dd>{formatValue(e.committed)}</dd>
          </>
        )}
        {e?.staged && (
          <>
            <dt>{t("var.next")}</dt>
            <dd>{formatValue(e.staged)}</dd>
          </>
        )}
        <dt>{t("var.quality")}</dt>
        <dd>
          {!e?.quality ? (
            "—"
          ) : e.quality === "GOOD" ? (
            t("var.good")
          ) : (
            <Badge kind="bad">
              {e.quality.replaceAll("_", " ").toLowerCase()}
            </Badge>
          )}
        </dd>
        {e?.initial && (
          <>
            <dt>{t("var.retention")}</dt>
            <dd>
              {e.retained ? (
                <Badge kind="info">{t("var.retained")}</Badge>
              ) : (
                t("var.nonRetained")
              )}
            </dd>
            <dt>{t("var.initial")}</dt>
            <dd className="mono">{formatValue(e.initial)}</dd>
            {e.restartEffect && (
              <>
                <dt>{t("var.latestStart")}</dt>
                <dd>
                  {e.restartEffect === "KEPT"
                    ? t("var.kept")
                    : t("var.initialized")}
                </dd>
              </>
            )}
          </>
        )}
        <dt>{t("var.source")}</dt>
        <dd>{e?.source ?? "—"}</dd>
        <dt>{t("var.force")}</dt>
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
            <span className="muted">{t("var.notForceable")}</span>
          )}
        </dd>
      </dl>
      {binding && (
        <>
          <h4 style={{ marginTop: 12 }}>{t("var.binding")}</h4>
          <dl className="kv">
            <dt>{t("var.channel")}</dt>
            <dd className="mono">{binding.channelId ?? "—"}</dd>
            <dt>{t("var.address")}</dt>
            <dd className="mono">{binding.address}</dd>
            <dt>{t("var.device")}</dt>
            <dd>
              {binding.drawnDevice
                ? `${binding.drawnDevice} (${binding.drawnPort ?? ""})`
                : t("var.notDrawn")}
            </dd>
            <dt>{t("var.deviceState")}</dt>
            <dd>{binding.deviceState ?? "—"}</dd>
          </dl>
        </>
      )}
      <h4 style={{ marginTop: 12 }}>{t("var.changes")}</h4>
      {changes.length === 0 ? (
        <p className="muted">{t("var.noChanges")}</p>
      ) : (
        <table className="table" aria-label={t("var.changes")}>
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
