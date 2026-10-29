/**
 * Watch tables: named tables of canonical signals (PLC tags, block outputs, process
 * values, I/O channels) with the runtime's value, type, quality, source, scan input,
 * committed value, next input and force state. Rows are saved in the project by
 * canonical signal path; every value comes from the backend's watch query.
 */
import { useState } from "react";
import { api, type WatchTableDto } from "@plc-lab/ipc";
import { formatValue, seconds, valueClass } from "../format";
import { Badge, Tabs } from "../ui";
import { useFrameQuery } from "./useFrameQuery";
import type { Selection } from "./ProjectTree";
import type { Lab } from "./useLab";
import {
  MAX_ROWS,
  MAX_TABLES,
  addTable,
  formatWatchValue,
  setRows,
  updateTable,
  type WatchFormat,
} from "./watchEdit";

type Column =
  | "symbol"
  | "path"
  | "value"
  | "type"
  | "quality"
  | "source"
  | "scan"
  | "committed"
  | "next"
  | "forced"
  | "time";
const COLUMNS: { id: Column; label: string }[] = [
  { id: "symbol", label: "Symbol" },
  { id: "path", label: "Canonical ID" },
  { id: "value", label: "Value" },
  { id: "type", label: "Type" },
  { id: "quality", label: "Quality" },
  { id: "source", label: "Source" },
  { id: "scan", label: "Scan input" },
  { id: "committed", label: "Committed" },
  { id: "next", label: "Next input" },
  { id: "forced", label: "Forced" },
  { id: "time", label: "World time" },
];
const DEFAULT_COLUMNS: Column[] = [
  "symbol",
  "value",
  "type",
  "quality",
  "forced",
];
const FORMATS: WatchFormat[] = ["auto", "decimal", "hex", "binary"];
const KIND_LABEL: Record<string, string> = {
  PLC_INPUT: "PLC input",
  PLC_OUTPUT: "PLC output",
  PLC_LOCAL: "PLC",
  BLOCK_OUTPUT: "Block output",
  PHYSICAL: "Process",
  CHANNEL: "I/O channel",
};
const STORAGE = "plclab.watch.columns";

/** Visible columns are a per-viewer preference, so browser storage is enough. */
function loadColumns(): Column[] {
  try {
    const raw = localStorage.getItem(STORAGE);
    const list: unknown = raw ? JSON.parse(raw) : null;
    if (Array.isArray(list)) {
      const valid = list.filter((c): c is Column =>
        COLUMNS.some((k) => k.id === c),
      );
      if (valid.length) return valid;
    }
  } catch {
    // Storage unavailable or unreadable: use the defaults.
  }
  return DEFAULT_COLUMNS;
}
function storeColumns(columns: Column[]) {
  try {
    localStorage.setItem(STORAGE, JSON.stringify(columns));
  } catch {
    // Not stored: the choice still applies to this view.
  }
}

export function WatchTables({
  lab,
  select = () => undefined,
}: {
  lab: Lab;
  select?: (s: Selection) => void;
}) {
  const ws = lab.ws;
  const [columns, setColumns] = useState<Column[]>(loadColumns);
  const [search, setSearch] = useState("");
  const [name, setName] = useState<string | null>(null);
  const tables = lab.watchTables;
  const table = tables.find((t) => t.id === lab.activeTable) ?? null;
  const byPath = new Map(lab.signals.map((s) => [s.path, s]));
  const ids = (table?.rows ?? [])
    .map((r) => byPath.get(r.signal)?.id)
    .filter((id): id is number => id !== undefined);
  const live = useFrameQuery(
    lab,
    () => (ws && ids.length ? api.watch(ws, ids) : Promise.resolve(null)),
    [ids.join(",")],
  );
  const entries = new Map((live.data?.rows ?? []).map((e) => [e.path, e]));
  const save = (next: WatchTableDto) =>
    lab.saveWatchTables(updateTable(tables, next), next.id);
  const needle = search.trim().toLowerCase();
  const matches = needle
    ? lab.signals
        .filter(
          (s) =>
            s.active &&
            !table?.rows.some((r) => r.signal === s.path) &&
            `${s.label} ${s.path}`.toLowerCase().includes(needle),
        )
        .slice(0, 30)
    : [];
  const visible = COLUMNS.filter((c) => columns.includes(c.id));
  const toggle = (id: Column) => {
    const next = columns.includes(id)
      ? columns.filter((c) => c !== id)
      : [...columns, id];
    setColumns(next);
    storeColumns(next);
  };
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="row wrap" style={{ padding: "4px 8px", gap: 8 }}>
        {tables.length > 0 && (
          <Tabs<string>
            label="Watch tables"
            active={table?.id ?? ""}
            onSelect={(id) => {
              lab.selectTable(id);
              setName(null);
            }}
            tabs={tables.map((t) => ({ id: t.id, label: t.name }))}
          />
        )}
        <button
          className="btn btn-sm"
          disabled={tables.length >= MAX_TABLES}
          onClick={() => {
            const next = addTable(tables);
            lab.saveWatchTables(next, next.at(-1)?.id ?? null);
            setName(null);
          }}
        >
          New table
        </button>
        {table && (
          <>
            <label className="row muted" style={{ fontSize: 12, gap: 4 }}>
              Name
              <input
                className="input"
                value={name ?? table.name}
                maxLength={40}
                onChange={(e) => setName(e.target.value)}
              />
            </label>
            <button
              className="btn btn-sm"
              disabled={!name?.trim() || name.trim() === table.name}
              onClick={() => {
                save({ ...table, name: (name ?? table.name).trim() });
                setName(null);
              }}
            >
              Rename
            </button>
            <button
              className="btn btn-sm btn-ghost"
              onClick={() => {
                const rest = tables.filter((t) => t.id !== table.id);
                lab.saveWatchTables(rest, rest[0]?.id ?? null);
              }}
            >
              Delete table
            </button>
          </>
        )}
        <span className="spacer" />
        <details>
          <summary className="muted" style={{ fontSize: 12 }}>
            Columns
          </summary>
          <div className="col" style={{ padding: 4 }} role="group">
            {COLUMNS.map((c) => (
              <label key={c.id} className="row" style={{ fontSize: 12 }}>
                <input
                  type="checkbox"
                  checked={columns.includes(c.id)}
                  onChange={() => toggle(c.id)}
                />
                {c.label}
              </label>
            ))}
          </div>
        </details>
      </div>
      {table && (
        <div className="col" style={{ padding: "0 8px 4px", gap: 4 }}>
          <input
            className="input"
            type="search"
            placeholder="Find a PLC tag, block output, process value or I/O channel…"
            aria-label="Find a signal"
            value={search}
            disabled={table.rows.length >= MAX_ROWS}
            onChange={(e) => setSearch(e.target.value)}
            style={{ maxWidth: 420 }}
          />
          {matches.length > 0 && (
            <ul className="list" aria-label="Matching signals">
              {matches.map((s) => (
                <li key={s.path}>
                  <button
                    className="btn btn-sm btn-ghost"
                    onClick={() => {
                      save(
                        setRows(table, [
                          ...table.rows.map((r) => r.signal),
                          s.path,
                        ]),
                      );
                      setSearch("");
                    }}
                  >
                    Add {s.label}{" "}
                    <span className="dim">
                      ({KIND_LABEL[s.kind] ?? s.kind})
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
      <div style={{ flex: 1, overflow: "auto" }}>
        {!table ? (
          <div className="empty">
            Create a watch table to follow PLC tags, block outputs, process
            values and I/O channels together. Rows are saved with the project.
          </div>
        ) : table.rows.length === 0 ? (
          <div className="empty">
            Find a signal above to add it. The active table also feeds the
            waveforms.
          </div>
        ) : (
          <table className="table" aria-label={`Watch table ${table.name}`}>
            <thead>
              <tr>
                {visible.map((c) => (
                  <th key={c.id}>{c.label}</th>
                ))}
                <th>Format</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {table.rows.map((row) => {
                const e = entries.get(row.signal);
                const known = byPath.get(row.signal);
                const cell = (c: Column) => {
                  switch (c) {
                    case "symbol":
                      return known ? (
                        <>
                          {e?.label ?? known.label}{" "}
                          <span className="dim">
                            {KIND_LABEL[known.kind] ?? known.kind}
                          </span>
                        </>
                      ) : (
                        <span className="dim">
                          {row.signal}: not in the loaded program or station
                        </span>
                      );
                    case "path":
                      return <span className="mono">{row.signal}</span>;
                    case "value":
                      return (
                        <span className={valueClass(e?.value)}>
                          {formatWatchValue(
                            e?.value,
                            row.format,
                            e?.unit ?? null,
                          )}
                        </span>
                      );
                    case "type":
                      return e?.valueType ?? "—";
                    case "quality":
                      return !e?.quality ? (
                        "—"
                      ) : e.quality === "GOOD" ? (
                        <span className="dim">good</span>
                      ) : (
                        <Badge kind="bad">
                          {e.quality.replaceAll("_", " ").toLowerCase()}
                        </Badge>
                      );
                    case "source":
                      return e?.source ?? "—";
                    case "scan":
                      return formatValue(e?.scanValue);
                    case "committed":
                      return formatValue(e?.committed);
                    case "next":
                      return formatValue(e?.staged);
                    case "forced":
                      return e?.forced ? (
                        <Badge kind="forced">F {formatValue(e.forced)}</Badge>
                      ) : (
                        ""
                      );
                    case "time":
                      return live.data ? seconds(live.data.worldTimeUs) : "—";
                  }
                };
                return (
                  <tr key={row.signal}>
                    {visible.map((c) => (
                      <td key={c.id}>{cell(c.id)}</td>
                    ))}
                    <td>
                      <select
                        className="input"
                        style={{ height: 22 }}
                        aria-label={`Format of ${known?.label ?? row.signal}`}
                        value={row.format}
                        onChange={(ev) =>
                          save({
                            ...table,
                            rows: table.rows.map((r) =>
                              r.signal === row.signal
                                ? {
                                    ...r,
                                    format: ev.target.value as WatchFormat,
                                  }
                                : r,
                            ),
                          })
                        }
                      >
                        {FORMATS.map((f) => (
                          <option key={f} value={f}>
                            {f}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td>
                      {known && (
                        <button
                          className="btn btn-sm btn-ghost"
                          onClick={() =>
                            select({ kind: "signal", id: known.id })
                          }
                        >
                          Details
                        </button>
                      )}
                      <button
                        className="btn btn-sm btn-ghost"
                        onClick={() =>
                          save(
                            setRows(
                              table,
                              table.rows
                                .map((r) => r.signal)
                                .filter((p) => p !== row.signal),
                            ),
                          )
                        }
                      >
                        Remove
                      </button>
                    </td>
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
