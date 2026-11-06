/** Network view: topology, nodes, Modbus TCP connections with a request/response
 * inspector, the bounded Modbus traffic and a ping tool. Everything shown comes from the
 * backend's network model; the view never invents protocol traffic. */
import { useState } from "react";
import {
  api,
  type NetworkDto,
  type PingDto,
  type ScanViewDto,
  type TrafficDto,
} from "@plc-lab/ipc";
import { seconds } from "../format";
import { useInterface } from "../locale";
import { isTranslationKey, type Translate } from "../locale/messages";
import { Badge, ErrorNotice } from "../ui";
import { useEvidence, useFrameQuery } from "./useFrameQuery";
import type { Lab } from "./useLab";
import { NetworkConfig } from "./NetworkConfig";

export function NetworkTab({ lab, advanced }: { lab: Lab; advanced: boolean }) {
  const { t } = useInterface();
  const { ws } = lab;
  const net = useFrameQuery<NetworkDto>(lab, () =>
    ws ? api.network(ws) : Promise.reject(new Error("No workspace")),
  );
  const [target, setTarget] = useState("");
  const [ping, setPing] = useState<{ target: string; dto: PingDto } | null>(
    null,
  );
  const [selected, setSelected] = useState<string | null>(null);
  useEvidence(lab, "network view");
  const n = net.data;
  if (!n || !ws) return <ErrorNotice error={net.error} />;
  const devices = n.devices;
  const scan = n.scans.find((x) => x.id === selected) ?? n.scans[0];
  const width = Math.max(700, 200 + devices.length * 200);
  return (
    <div
      style={{ position: "absolute", inset: 0, overflow: "auto", padding: 16 }}
      className="col"
    >
      <svg
        className="topology panel"
        viewBox={`0 0 ${width} 260`}
        role="img"
        aria-label={t("net.topology")}
      >
        <line
          className={`link ${n.plcLinkUp ? "up" : "down"}`}
          x1={140}
          y1={130}
          x2={width / 2}
          y2={130}
        />
        <rect
          className={`node${n.plcLinkUp ? "" : " down"}`}
          x={40}
          y={95}
          width={110}
          height={70}
          rx={4}
        />
        <text x={95} y={122} textAnchor="middle" style={{ fontWeight: 700 }}>
          PLC
        </text>
        <text x={95} y={140} textAnchor="middle">
          {n.plcInterface ? n.plcInterface.ip : t("net.noIp")}
        </text>
        <text x={95} y={155} textAnchor="middle" className="dim">
          {n.plcInterface ? `/${n.plcInterface.mask}` : ""}
        </text>
        <rect
          className="node"
          x={width / 2 - 40}
          y={112}
          width={80}
          height={36}
          rx={4}
        />
        <text x={width / 2} y={134} textAnchor="middle">
          {t("net.switch")}
        </text>
        {devices.map((d, i) => {
          const x = width / 2 + 120 + i * 190;
          return (
            <g key={d.id}>
              <line
                className={`link ${d.linkUp ? "up" : "down"}`}
                x1={width / 2 + 40}
                y1={130}
                x2={x}
                y2={130}
              />
              <rect
                className={`node${d.linkUp && d.online ? "" : " down"}`}
                x={x}
                y={90}
                width={170}
                height={80}
                rx={4}
              />
              <text
                x={x + 85}
                y={114}
                textAnchor="middle"
                style={{ fontWeight: 600 }}
              >
                {d.label.length > 26 ? `${d.label.slice(0, 25)}…` : d.label}
              </text>
              <text x={x + 85} y={132} textAnchor="middle">
                {d.ip}
              </text>
              <text x={x + 85} y={150} textAnchor="middle" className="dim">
                {d.online
                  ? d.linkUp
                    ? t("net.served", { count: d.requestsServed })
                    : t("net.linkDown")
                  : t("net.notResponding")}
              </text>
            </g>
          );
        })}
      </svg>
      <div className="panel">
        <div className="panel-head">
          <h4>{t("net.nodes")}</h4>
          <span className="spacer" />
          <span className="muted">{t("net.nodesHelp")}</span>
        </div>
        <table className="table">
          <thead>
            <tr>
              <th>{t("net.node")}</th>
              <th>{t("net.ip")}</th>
              <th>{t("net.mask")}</th>
              <th title={t("net.plcSubnetHelp")}>{t("net.plcSubnet")}</th>
              <th>{t("net.status")}</th>
              <th>{t("net.requestsServed")}</th>
              <th>{t("net.lastActivity")}</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>PLC</td>
              <td className="mono">{n.plcInterface?.ip ?? t("net.noIp")}</td>
              <td className="mono">{n.plcInterface?.mask ?? ""}</td>
              <td>—</td>
              <td>{n.plcLinkUp ? t("net.linkUp") : t("net.noLink")}</td>
              <td>—</td>
              <td>—</td>
            </tr>
            {devices.map((d) => (
              <tr key={d.id}>
                <td>{d.label}</td>
                <td className="mono">{d.ip}</td>
                <td className="mono">{d.mask}</td>
                <td>
                  {d.sameSubnet === null
                    ? "—"
                    : d.sameSubnet
                      ? t("net.yes")
                      : t("net.no")}
                </td>
                <td>
                  {!d.online
                    ? t("net.notResponding")
                    : d.linkUp
                      ? t("net.linkUp")
                      : t("net.noLink")}
                </td>
                <td>{d.modbusServer ? d.requestsServed : "—"}</td>
                <td className="mono">
                  {d.lastActivityUs ? seconds(d.lastActivityUs) : "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="panel">
        <div className="panel-head">
          <h4>{t("net.connections")}</h4>
          <span className="spacer" />
          <span className="muted">{t("net.selectHelp")}</span>
        </div>
        <table className="table">
          <thead>
            <tr>
              <th>{t("net.connection")}</th>
              <th>{t("net.server")}</th>
              <th>{t("net.unit")}</th>
              <th>{t("net.status")}</th>
              <th>{t("net.requests")}</th>
              <th>{t("net.responses")}</th>
              <th>{t("net.timeouts")}</th>
              <th>{t("net.exceptions")}</th>
              <th>{t("net.roundTrip")}</th>
            </tr>
          </thead>
          <tbody>
            {n.scans.length === 0 && (
              <tr>
                <td colSpan={9} className="muted">
                  {t("net.noConnections")}
                </td>
              </tr>
            )}
            {n.scans.map((s) => (
              <tr
                key={s.id}
                className={s.id === scan?.id ? "selected" : undefined}
              >
                <td>
                  <button
                    className="btn btn-ghost btn-sm"
                    aria-pressed={s.id === scan?.id}
                    onClick={() => setSelected(s.id)}
                  >
                    {s.id}
                  </button>
                </td>
                <td className="mono">{s.server}</td>
                <td>{s.unitId}</td>
                <td>
                  <Badge
                    kind={
                      s.status === "OK"
                        ? "active"
                        : s.status === "IDLE"
                          ? "normal"
                          : "comm"
                    }
                  >
                    {scanStatus(t, s.status)}
                  </Badge>
                </td>
                <td>{s.requests}</td>
                <td>{s.responses}</td>
                <td>{s.timeouts}</td>
                <td>{s.exceptions}</td>
                <td>{milliseconds(s.lastRoundTripUs)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {scan && <Inspector scan={scan} t={t} />}
      <div className="row" style={{ alignItems: "flex-start", gap: 16 }}>
        <div className="panel grow">
          <div className="panel-head">
            <h4>{t("net.traffic")}</h4>
            <span className="spacer" />
            <span className="muted">
              {t("net.trafficCount", { count: n.traffic.length })}
            </span>
          </div>
          <div style={{ maxHeight: 260, overflow: "auto" }}>
            <table className="table" aria-label={t("net.traffic")}>
              <thead>
                <tr>
                  <th>{t("net.time")}</th>
                  <th>{t("net.connection")}</th>
                  <th>TID</th>
                  <th />
                  <th>{t("net.message")}</th>
                </tr>
              </thead>
              <tbody>
                {n.traffic.length === 0 && (
                  <tr>
                    <td colSpan={5} className="muted">
                      {t("net.noTraffic")}
                    </td>
                  </tr>
                )}
                {n.traffic.map((m) => (
                  <tr
                    key={m.seq}
                    className={
                      m.kind === "request" || m.kind === "response"
                        ? undefined
                        : "warning"
                    }
                  >
                    <td className="mono dim">{seconds(m.timeUs)}</td>
                    <td>{m.connection}</td>
                    <td className="mono">{m.transaction}</td>
                    <td>{m.kind === "request" ? "→" : "←"}</td>
                    <td>{trafficText(t, m)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
        <div className="panel" style={{ width: 300 }}>
          <div className="panel-head">
            <h4>{t("net.ping")}</h4>
          </div>
          <div className="panel-body col">
            <input
              className="input mono"
              placeholder="192.168.0.20"
              value={target}
              onChange={(e) => setTarget(e.target.value)}
              aria-label={t("net.ip")}
            />
            <button
              className="btn"
              disabled={!target.trim()}
              onClick={() =>
                void api.ping(ws, target.trim()).then(
                  (dto) => setPing({ target: target.trim(), dto }),
                  (e: unknown) =>
                    lab.log(
                      "error",
                      e instanceof Error ? e.message : String(e),
                    ),
                )
              }
            >
              {t("net.pingButton")}
            </button>
            {ping && (
              <div
                className={`notice ${ping.dto.result === "REPLY" ? "success" : "warning"}`}
                title={ping.dto.text}
              >
                {pingText(t, ping.target, ping.dto)}
              </div>
            )}
          </div>
        </div>
      </div>
      {advanced ? (
        <NetworkConfig lab={lab} />
      ) : (
        <p className="muted">{t("net.advancedHint")}</p>
      )}
    </div>
  );
}

function hex(value: number, digits: number) {
  return value.toString(16).toUpperCase().padStart(digits, "0");
}

function milliseconds(us: string | null) {
  return us ? `${(Number(us) / 1000).toFixed(1)} ms` : "—";
}

/** "OK", "NO_ROUTE", "EXCEPTION_0B", ... from the backend's connection state. */
function scanStatus(t: Translate, status: string): string {
  const exception = /^EXCEPTION_([0-9A-F]{2})$/.exec(status);
  if (exception) return t("net.scan.EXCEPTION", { code: exception[1]! });
  const key = `net.scan.${status}`;
  return isTranslationKey(key) ? t(key) : status.replaceAll("_", " ");
}

function pingText(t: Translate, target: string, ping: PingDto): string {
  const key = `net.ping.${ping.result}`;
  if (!isTranslationKey(key)) return ping.text;
  return t(key, {
    target,
    time: ping.roundTripUs ? (Number(ping.roundTripUs) / 1000).toFixed(1) : "",
  });
}

/** Standard Modbus exception names (Modbus application protocol), by code. */
function exceptionText(t: Translate, code: number | null) {
  if (code === null) return t("net.exception");
  const key = `modbus.exception.${hex(code, 2)}h`;
  return t("net.exceptionCode", {
    code: hex(code, 2),
    name: isTranslationKey(key) ? t(key) : t("net.notStandard"),
  });
}

function trafficText(t: Translate, m: TrafficDto): string {
  switch (m.kind) {
    case "request":
      return t("net.traffic.request", {
        fc: hex(m.functionCode ?? 0, 2),
        quantity: m.quantity ?? 0,
        address: m.address ?? 0,
        unit: m.unitId ?? 0,
        peer: m.peer,
      });
    case "response":
      return t("net.traffic.response", {
        peer: m.peer,
        time: milliseconds(m.roundTripUs),
      });
    case "exception":
      return t("net.traffic.exception", {
        exception: exceptionText(t, m.exceptionCode),
        peer: m.peer,
      });
    default:
      return t(`net.traffic.${m.kind}`, { peer: m.peer });
  }
}

/** Documentation register number in the common 4xxxx / 3xxxx convention. */
function documentNumber(functionCode: number, address: number) {
  return (functionCode === 3 ? 40001 : 30001) + address;
}

function Inspector({ scan, t }: { scan: ScanViewDto; t: Translate }) {
  const r = scan.request;
  const label = t("net.inspector", { id: scan.id });
  return (
    <section className="panel" aria-label={label}>
      <div className="panel-head">
        <h4>{label}</h4>
        <span className="spacer" />
        <span className="muted">
          {t("net.lastRequest", {
            time: scan.lastRequestUs ? seconds(scan.lastRequestUs) : "—",
          })}{" "}
          ·{" "}
          {t("net.lastResponse", {
            time: scan.lastResponseUs ? seconds(scan.lastResponseUs) : "—",
          })}
        </span>
      </div>
      <div
        className="row wrap panel-body"
        style={{ alignItems: "flex-start", gap: 24 }}
      >
        <dl className="kv">
          <dt>{t("net.request")}</dt>
          <dd>
            FC{hex(r.functionCode, 2)}{" "}
            {r.function === "readHoldingRegisters"
              ? t("net.readHolding")
              : t("net.readInput")}
          </dd>
          <dt>{t("net.server")}</dt>
          <dd className="mono">
            {t("net.serverUnit", {
              server: scan.server,
              port: r.port,
              unit: scan.unitId,
            })}
          </dd>
          <dt>{t("net.protocolAddress")}</dt>
          <dd className="mono">
            {t(r.quantity === 1 ? "net.registers.one" : "net.registers.many", {
              first: r.startAddress,
              last: r.startAddress + r.quantity - 1,
              count: r.quantity,
            })}
          </dd>
          <dt title={t("net.notationHelp")}>{t("net.notation")}</dt>
          <dd className="mono">
            {documentNumber(r.functionCode, r.startAddress)} …{" "}
            {documentNumber(r.functionCode, r.startAddress + r.quantity - 1)}
          </dd>
          <dt>{t("net.period")}</dt>
          <dd>
            {r.periodMs} ms / {r.timeoutMs} ms
          </dd>
          <dt>{t("net.status")}</dt>
          <dd>{scanStatus(t, scan.status)}</dd>
        </dl>
        <div>
          <strong>{t("net.raw")}</strong>
          {scan.registers.length === 0 ? (
            <p className="muted">{t("net.noGood")}</p>
          ) : (
            <table className="table">
              <thead>
                <tr>
                  <th>{t("net.address")}</th>
                  <th>{t("net.decimal")}</th>
                  <th>{t("net.hex")}</th>
                </tr>
              </thead>
              <tbody>
                {scan.registers.map((w, i) => (
                  <tr key={i}>
                    <td className="mono">{r.startAddress + i}</td>
                    <td className="mono">{w}</td>
                    <td className="mono">{hex(w, 4)}h</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
        <div>
          <strong>{t("net.decoded")}</strong>
          <table className="table">
            <thead>
              <tr>
                <th>{t("net.tag")}</th>
                <th>{t("net.offset")}</th>
                <th>{t("net.type")}</th>
                <th>{t("net.value")}</th>
              </tr>
            </thead>
            <tbody>
              {scan.values.map((v) => (
                <tr key={v.tag}>
                  <td>{v.tag}</td>
                  <td className="mono">+{v.offset}</td>
                  <td>
                    {v.dataType === "int16"
                      ? t("net.int16")
                      : t("net.real", {
                          order: t(
                            v.wordOrder === "lowWordFirst"
                              ? "net.lowFirst"
                              : "net.highFirst",
                          ),
                        })}
                  </td>
                  <td className="mono">{v.value ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
      <p className="muted panel-body" style={{ marginTop: 0 }}>
        {t("net.addressHelp")}
      </p>
    </section>
  );
}
