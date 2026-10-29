/**
 * Advanced network configuration: the PLC's Ethernet interface and its Modbus TCP
 * client connections. Edits a draft of the station that is downloaded with the program;
 * the backend validates it before anything changes.
 */
import { useState } from "react";
import type { StationDto } from "@plc-lab/ipc";
import type { Lab } from "./useLab";

function num(value: string, fallback: number) {
  const n = Number(value);
  return Number.isInteger(n) && n >= 0 ? n : fallback;
}

export function NetworkConfig({ lab }: { lab: Lab }) {
  const station = lab.project?.station ?? null;
  const [draft, setDraft] = useState<StationDto | null>(null);
  if (!station) return null;
  const s = draft ?? station;
  const edit = (change: (next: StationDto) => void) => {
    const next: StationDto = structuredClone(s);
    change(next);
    setDraft(next);
  };
  const iface = s.network.interface;
  return (
    <div className="panel">
      <div className="panel-head">
        <h4>Network configuration</h4>
        <span className="spacer" />
        {draft && (
          <>
            <button
              className="btn btn-sm btn-primary"
              disabled={lab.busy !== null}
              onClick={() =>
                void lab
                  // Only the network part: hardware edits are saved in the
                  // project meanwhile and must not be overwritten.
                  .buildAndLoad(false, { ...station, network: draft.network })
                  .then((r) => r?.outcome === "LOADED" && setDraft(null))
              }
            >
              Download configuration
            </button>
            <button className="btn btn-sm" onClick={() => setDraft(null)}>
              Discard
            </button>
          </>
        )}
      </div>
      <div className="panel-body col">
        <div className="row wrap">
          <label className="field">
            PLC IP address
            <input
              className="input mono"
              value={iface?.ip ?? ""}
              onChange={(e) =>
                edit((n) => {
                  n.network.interface = {
                    ip: e.target.value,
                    mask: n.network.interface?.mask ?? "255.255.255.0",
                    gateway: n.network.interface?.gateway ?? null,
                  };
                })
              }
            />
          </label>
          <label className="field">
            Subnet mask
            <input
              className="input mono"
              value={iface?.mask ?? ""}
              disabled={!iface}
              onChange={(e) =>
                edit((n) => {
                  if (n.network.interface)
                    n.network.interface.mask = e.target.value;
                })
              }
            />
          </label>
          <label className="field">
            Gateway
            <input
              className="input mono"
              value={iface?.gateway ?? ""}
              disabled={!iface}
              placeholder="none"
              onChange={(e) =>
                edit((n) => {
                  if (n.network.interface)
                    n.network.interface.gateway = e.target.value || null;
                })
              }
            />
          </label>
        </div>
        {s.network.modbusTcp.map((scan, i) => (
          <div key={scan.id} className="source-card col">
            <strong>Modbus TCP client connection “{scan.id}”</strong>
            <div className="row wrap">
              <label className="field">
                Server IP
                <input
                  className="input mono"
                  value={scan.server}
                  onChange={(e) =>
                    edit((n) => {
                      const target = n.network.modbusTcp[i];
                      if (target) target.server = e.target.value;
                    })
                  }
                />
              </label>
              <label className="field">
                Unit ID
                <input
                  className="input mono"
                  style={{ width: 70 }}
                  value={scan.unitId}
                  onChange={(e) =>
                    edit((n) => {
                      const target = n.network.modbusTcp[i];
                      if (target)
                        target.unitId = Math.min(
                          255,
                          num(e.target.value, target.unitId),
                        );
                    })
                  }
                />
              </label>
              <label className="field">
                Function
                <select
                  className="input"
                  value={scan.function}
                  onChange={(e) =>
                    edit((n) => {
                      const target = n.network.modbusTcp[i];
                      if (target)
                        target.function = e.target
                          .value as typeof target.function;
                    })
                  }
                >
                  <option value="readHoldingRegisters">
                    03 Read holding registers
                  </option>
                  <option value="readInputRegisters">
                    04 Read input registers
                  </option>
                </select>
              </label>
              <label className="field">
                Start address (0-based)
                <input
                  className="input mono"
                  style={{ width: 90 }}
                  value={scan.startAddress}
                  onChange={(e) =>
                    edit((n) => {
                      const target = n.network.modbusTcp[i];
                      if (target)
                        target.startAddress = Math.min(
                          65535,
                          num(e.target.value, target.startAddress),
                        );
                    })
                  }
                />
              </label>
              <label className="field">
                Quantity
                <input
                  className="input mono"
                  style={{ width: 70 }}
                  value={scan.quantity}
                  onChange={(e) =>
                    edit((n) => {
                      const target = n.network.modbusTcp[i];
                      if (target)
                        target.quantity = Math.min(
                          125,
                          num(e.target.value, target.quantity),
                        );
                    })
                  }
                />
              </label>
            </div>
            <table className="table">
              <thead>
                <tr>
                  <th>Tag</th>
                  <th>Offset in the response</th>
                  <th>Data type</th>
                </tr>
              </thead>
              <tbody>
                {scan.values.map((v, j) => (
                  <tr key={v.tag}>
                    <td className="mono">{v.tag}</td>
                    <td>
                      <input
                        className="input mono"
                        style={{ width: 70, height: 22 }}
                        value={v.offset}
                        aria-label={`Offset of ${v.tag}`}
                        onChange={(e) =>
                          edit((n) => {
                            const value = n.network.modbusTcp[i]?.values[j];
                            if (value)
                              value.offset = num(e.target.value, value.offset);
                          })
                        }
                      />
                    </td>
                    <td>
                      {v.dataType.type === "int16"
                        ? "INT16"
                        : `FLOAT32 (${v.dataType.wordOrder === "highWordFirst" ? "high word first" : "low word first"})`}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            <span className="dim" style={{ fontSize: 11 }}>
              Register n of the response is protocol address startAddress + n
              (documentation often writes 40001 for address 0).
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
