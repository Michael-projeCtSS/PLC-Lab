/** Commissioning checklist: evaluated by the engine from the simulated installation and the
 * I/O tests performed in this session. The view only displays it. */
import { useState } from "react";
import { api, type CheckItemDto, type CommissioningDto } from "@plc-lab/ipc";
import { Badge, ErrorNotice } from "../ui";
import { useEvidence, useFrameQuery } from "./useFrameQuery";
import type { Lab } from "./useLab";

type StateStyle = { badge: string; label: string; help: string };
const NOT_APPLICABLE: StateStyle = {
  badge: "normal",
  label: "n/a",
  help: "Nothing to check in this machine.",
};
const STATE: Record<string, StateStyle | undefined> = {
  OK: { badge: "active", label: "OK", help: "Checked and as documented." },
  ATTENTION: {
    badge: "warning",
    label: "Check",
    help: "Deviates from the drawing or needs a closer look.",
  },
  FAULT: {
    badge: "fault",
    label: "Fault",
    help: "Prevents correct operation.",
  },
  UNTESTED: {
    badge: "stopped",
    label: "To test",
    help: "Needs a test you have not performed yet.",
  },
  NOT_APPLICABLE,
};

function StateBadge({ state }: { state: string }) {
  const s = STATE[state] ?? NOT_APPLICABLE;
  return (
    <Badge kind={s.badge} title={s.help}>
      {s.label}
    </Badge>
  );
}

function Item({ item }: { item: CheckItemDto }) {
  return (
    <li className="check-item" aria-label={`${item.title}: ${item.state}`}>
      <div className="row" style={{ gap: 8 }}>
        <StateBadge state={item.state} />
        <strong>{item.title}</strong>
      </div>
      <ul className="check-findings">
        {item.findings.map((f, i) => (
          <li key={i}>{f}</li>
        ))}
      </ul>
      {item.how && item.state !== "OK" && <p className="muted">{item.how}</p>}
    </li>
  );
}

export function CommissioningTab({ lab }: { lab: Lab }) {
  const { ws } = lab;
  const list = useFrameQuery<CommissioningDto>(lab, () =>
    ws ? api.commissioning(ws) : Promise.reject(new Error("No workspace")),
  );
  useEvidence(lab, "commissioning checklist");
  // Categories with nothing to check start collapsed; a click toggles a category.
  const [toggled, setToggled] = useState<ReadonlySet<string>>(new Set());
  const toggle = (id: string) =>
    setToggled((prev) => {
      const next = new Set(prev);
      if (!next.delete(id)) next.add(id);
      return next;
    });
  const categories = list.data?.categories ?? [];
  const counts = categories
    .flatMap((c) => c.items)
    .reduce<Record<string, number>>((acc, i) => {
      acc[i.state] = (acc[i.state] ?? 0) + 1;
      return acc;
    }, {});
  return (
    <div
      style={{ position: "absolute", inset: 0, overflow: "auto", padding: 16 }}
      className="col"
      aria-label="Commissioning checklist"
    >
      <ErrorNotice error={list.error} />
      <div className="row wrap" style={{ gap: 8 }}>
        <p className="muted" style={{ margin: 0 }}>
          Work from Power to Process. Operate every button and sensor and switch
          every output: checks marked "To test" turn into results as the
          simulation observes them. Missing feedback is reported after an
          actuator has been energized for {list.data?.feedbackWaitS ?? 3} s.
        </p>
        <span className="spacer" />
        {(["FAULT", "ATTENTION", "UNTESTED", "OK"] as const).map((s) =>
          counts[s] ? (
            <span key={s} className="row" style={{ gap: 4 }}>
              <StateBadge state={s} /> {counts[s]}
            </span>
          ) : null,
        )}
      </div>
      <div className="check-grid">
        {categories.map((c) => {
          const expanded = (c.state !== "NOT_APPLICABLE") !== toggled.has(c.id);
          return (
            <section key={c.id} className="panel" aria-label={c.title}>
              <button
                className="panel-head check-head"
                aria-expanded={expanded}
                onClick={() => toggle(c.id)}
              >
                <h4>{c.title.toUpperCase()}</h4>
                <span className="spacer" />
                <StateBadge state={c.state} />
              </button>
              {expanded && (
                <ul className="check-items panel-body">
                  {c.items.map((i) => (
                    <Item key={i.id} item={i} />
                  ))}
                </ul>
              )}
            </section>
          );
        })}
      </div>
    </div>
  );
}
