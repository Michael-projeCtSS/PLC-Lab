/** Network view: topology, Modbus TCP connection state, recent messages and a ping tool. */
import { useState } from "react";
import { api, type NetworkDto, type PingDto } from "@plc-lab/ipc";
import { seconds } from "../format";
import { Badge, ErrorNotice } from "../ui";
import { useEvidence, useFrameQuery } from "./useFrameQuery";
import type { Lab } from "./useLab";

export function NetworkTab({ lab }: { lab: Lab }) {
  const { ws } = lab;
  const net = useFrameQuery<NetworkDto>(lab, () =>
    ws ? api.network(ws) : Promise.reject(new Error("No workspace")),
  );
  const [target, setTarget] = useState("");
  const [ping, setPing] = useState<PingDto | null>(null);
  useEvidence(lab, "network view");
  const n = net.data;
  if (!n || !ws) return <ErrorNotice error={net.error} />;
  const devices = n.devices;
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
          <h4>Modbus TCP connections (PLC is client)</h4>
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
              <th>Registers (last good)</th>
            </tr>
          </thead>
          <tbody>
            {n.scans.length === 0 && (
              <tr>
                <td colSpan={10} className="muted">
                  No Modbus connection is configured in this station.
                </td>
              </tr>
            )}
            {n.scans.map((s) => (
              <tr key={s.id}>
                <td>{s.id}</td>
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
                <td>
                  {s.lastRoundTripUs
                    ? `${(Number(s.lastRoundTripUs) / 1000).toFixed(1)} ms`
                    : "—"}
                </td>
                <td className="mono">
                  {s.registers.map((r, i) => `${i}: ${r}`).join("  ")}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="row" style={{ alignItems: "flex-start", gap: 16 }}>
        <div className="panel grow">
          <div className="panel-head">
            <h4>Recent messages</h4>
          </div>
          <div style={{ maxHeight: 220, overflow: "auto" }}>
            <table className="table">
              <tbody>
                {n.recent.length === 0 && (
                  <tr>
                    <td className="muted">
                      No traffic yet. Run the simulation.
                    </td>
                  </tr>
                )}
                {n.recent.map((e) => (
                  <tr key={e.seq} className={e.severity}>
                    <td className="mono dim">{seconds(e.timeUs)}</td>
                    <td>{e.text}</td>
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
    </div>
  );
}
