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
import { Badge, ErrorNotice } from "../ui";
import { useEvidence, useFrameQuery } from "./useFrameQuery";
import type { Lab } from "./useLab";
import { NetworkConfig } from "./NetworkConfig";

export function NetworkTab({ lab, advanced }: { lab: Lab; advanced: boolean }) {
  const { ws } = lab;
  const net = useFrameQuery<NetworkDto>(lab, () =>
    ws ? api.network(ws) : Promise.reject(new Error("No workspace")),
  );
  const [target, setTarget] = useState("");
  const [ping, setPing] = useState<PingDto | null>(null);
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
        aria-label="Network topology"
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
          {n.plcInterface ? n.plcInterface.ip : "no IP"}
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
          Switch
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
                    ? `${d.requestsServed} requests served`
                    : "link down"
                  : "not responding"}
              </text>
            </g>
          );
        })}
      </svg>
      <div className="panel">
        <div className="panel-head">
          <h4>Nodes</h4>
          <span className="spacer" />
          <span className="muted">as each node reports itself</span>
        </div>
        <table className="table">
          <thead>
            <tr>
              <th>Node</th>
              <th>IP address</th>
              <th>Subnet mask</th>
              <th title="Is the address inside the PLC interface's subnet?">
                PLC subnet
              </th>
              <th>Status</th>
              <th>Requests served</th>
              <th>Last communication</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>PLC</td>
              <td className="mono">{n.plcInterface?.ip ?? "no IP"}</td>
              <td className="mono">{n.plcInterface?.mask ?? ""}</td>
              <td>—</td>
              <td>{n.plcLinkUp ? "link up" : "no link"}</td>
              <td>—</td>
              <td>—</td>
            </tr>
            {devices.map((d) => (
              <tr key={d.id}>
                <td>{d.label}</td>
                <td className="mono">{d.ip}</td>
                <td className="mono">{d.mask}</td>
                <td>
                  {d.sameSubnet === null ? "—" : d.sameSubnet ? "yes" : "no"}
                </td>
                <td>
                  {!d.online
                    ? "not responding"
                    : d.linkUp
                      ? "link up"
                      : "no link"}
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
          <h4>Modbus TCP connections (PLC is client)</h4>
          <span className="spacer" />
          <span className="muted">select one to inspect it</span>
        </div>
        <table className="table">
          <thead>
            <tr>
              <th>Connection</th>
              <th>Server</th>
              <th>Unit</th>
              <th>Status</th>
              <th>Requests</th>
              <th>Responses</th>
              <th>Timeouts</th>
              <th>Exceptions</th>
              <th>Round trip</th>
            </tr>
          </thead>
          <tbody>
            {n.scans.length === 0 && (
              <tr>
                <td colSpan={9} className="muted">
                  No Modbus connection is configured in this station.
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
                    {s.status.replaceAll("_", " ")}
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
      {scan && <Inspector scan={scan} />}
      <div className="row" style={{ alignItems: "flex-start", gap: 16 }}>
        <div className="panel grow">
          <div className="panel-head">
            <h4>Modbus traffic</h4>
            <span className="spacer" />
            <span className="muted">
              newest {n.traffic.length}, oldest first
            </span>
          </div>
          <div style={{ maxHeight: 260, overflow: "auto" }}>
            <table className="table" aria-label="Modbus traffic">
              <thead>
                <tr>
                  <th>Time</th>
                  <th>Connection</th>
                  <th>TID</th>
                  <th />
                  <th>Message</th>
                </tr>
              </thead>
              <tbody>
                {n.traffic.length === 0 && (
                  <tr>
                    <td colSpan={5} className="muted">
                      No traffic yet. The PLC polls its connections in RUN while
                      the simulation runs.
                    </td>
                  </tr>
                )}
                {n.traffic.map((t) => (
                  <tr
                    key={t.seq}
                    className={
                      t.kind === "request" || t.kind === "response"
                        ? undefined
                        : "warning"
                    }
                  >
                    <td className="mono dim">{seconds(t.timeUs)}</td>
                    <td>{t.connection}</td>
                    <td className="mono">{t.transaction}</td>
                    <td>{t.kind === "request" ? "→" : "←"}</td>
                    <td>{trafficText(t)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
        <div className="panel" style={{ width: 300 }}>
          <div className="panel-head">
            <h4>Ping from the PLC</h4>
          </div>
          <div className="panel-body col">
            <input
              className="input mono"
              placeholder="192.168.0.20"
              value={target}
              onChange={(e) => setTarget(e.target.value)}
              aria-label="IP address"
            />
            <button
              className="btn"
              disabled={!target.trim()}
              onClick={() =>
                void api
                  .ping(ws, target.trim())
                  .then(setPing, (e: unknown) =>
                    lab.log(
                      "error",
                      e instanceof Error ? e.message : String(e),
                    ),
                  )
              }
            >
              Ping
            </button>
            {ping && (
              <div
                className={`notice ${ping.result === "REPLY" ? "success" : "warning"}`}
              >
                {ping.text}
              </div>
            )}
          </div>
        </div>
      </div>
      {advanced ? (
        <NetworkConfig lab={lab} />
      ) : (
        <p className="muted">
          Switch to Advanced to edit the IP and Modbus configuration.
        </p>
      )}
    </div>
  );
}

/** Standard Modbus exception codes (Modbus application protocol). */
const EXCEPTION: Record<number, string | undefined> = {
  1: "illegal function",
  2: "illegal data address",
  3: "illegal data value",
  4: "server device failure",
  10: "gateway path unavailable",
  11: "gateway target device failed to respond",
};

function hex(value: number, digits: number) {
  return value.toString(16).toUpperCase().padStart(digits, "0");
}

function milliseconds(us: string | null) {
  return us ? `${(Number(us) / 1000).toFixed(1)} ms` : "—";
}

function exceptionText(code: number | null) {
  if (code === null) return "exception";
  return `exception ${hex(code, 2)}h: ${EXCEPTION[code] ?? "not a standard code"}`;
}

function trafficText(t: TrafficDto): string {
  switch (t.kind) {
    case "request":
      return `FC${hex(t.functionCode ?? 0, 2)} read ${t.quantity ?? 0} register(s) from address ${t.address ?? 0}, unit ${t.unitId ?? 0}, to ${t.peer}`;
    case "response":
      return `response from ${t.peer} after ${milliseconds(t.roundTripUs)}`;
    case "exception":
      return `${exceptionText(t.exceptionCode)} from ${t.peer}`;
    case "timeout":
      return `no response from ${t.peer} within the timeout`;
    case "noRoute":
      return `no route to ${t.peer}: outside the PLC subnet and no gateway`;
    case "arpUnresolved":
      return `address ${t.peer} not resolved: no device answers at this IP on the segment`;
    case "refused":
      return `${t.peer} refused the connection`;
    case "addressConflict":
      return `IP address conflict for ${t.peer}`;
  }
}

/** Documentation register number in the common 4xxxx / 3xxxx convention. */
function documentNumber(functionCode: number, address: number) {
  return (functionCode === 3 ? 40001 : 30001) + address;
}

function Inspector({ scan }: { scan: ScanViewDto }) {
  const r = scan.request;
  return (
    <section className="panel" aria-label={`Connection ${scan.id}`}>
      <div className="panel-head">
        <h4>Connection {scan.id}</h4>
        <span className="spacer" />
        <span className="muted">
          last request {scan.lastRequestUs ? seconds(scan.lastRequestUs) : "—"}{" "}
          · last response{" "}
          {scan.lastResponseUs ? seconds(scan.lastResponseUs) : "—"}
        </span>
      </div>
      <div
        className="row wrap panel-body"
        style={{ alignItems: "flex-start", gap: 24 }}
      >
        <dl className="kv">
          <dt>Request</dt>
          <dd>
            FC{hex(r.functionCode, 2)}{" "}
            {r.function === "readHoldingRegisters"
              ? "read holding registers"
              : "read input registers"}
          </dd>
          <dt>Server</dt>
          <dd className="mono">
            {scan.server}:{r.port}, unit {scan.unitId}
          </dd>
          <dt>Protocol address</dt>
          <dd className="mono">
            {r.startAddress} … {r.startAddress + r.quantity - 1} ({r.quantity}{" "}
            register{r.quantity === 1 ? "" : "s"})
          </dd>
          <dt title="Common documentation convention; device manuals differ">
            In 4xxxx notation
          </dt>
          <dd className="mono">
            {documentNumber(r.functionCode, r.startAddress)} …{" "}
            {documentNumber(r.functionCode, r.startAddress + r.quantity - 1)}
          </dd>
          <dt>Poll period / timeout</dt>
          <dd>
            {r.periodMs} ms / {r.timeoutMs} ms
          </dd>
          <dt>Status</dt>
          <dd>{scan.status.replaceAll("_", " ")}</dd>
        </dl>
        <div>
          <strong>Raw words (last good response)</strong>
          {scan.registers.length === 0 ? (
            <p className="muted">No good response yet.</p>
          ) : (
            <table className="table">
              <thead>
                <tr>
                  <th>Address</th>
                  <th>Decimal</th>
                  <th>Hex</th>
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
          <strong>Decoded values</strong>
          <table className="table">
            <thead>
              <tr>
                <th>Tag</th>
                <th>Offset</th>
                <th>Type</th>
                <th>Value</th>
              </tr>
            </thead>
            <tbody>
              {scan.values.map((v) => (
                <tr key={v.tag}>
                  <td>{v.tag}</td>
                  <td className="mono">+{v.offset}</td>
                  <td>
                    {v.dataType === "int16"
                      ? "INT (16 bit)"
                      : `REAL (32 bit, ${v.wordOrder === "lowWordFirst" ? "low word first" : "high word first"})`}
                  </td>
                  <td className="mono">{v.value ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
      <p className="muted panel-body" style={{ marginTop: 0 }}>
        Modbus messages carry a zero-based protocol address. Many device manuals
        number holding registers from 40001 (input registers from 30001), so
        protocol address 0 is register 40001 there; other manuals count from 0
        or use six digits (400001). Check which convention the manual uses
        before entering an address.
      </p>
    </section>
  );
}
