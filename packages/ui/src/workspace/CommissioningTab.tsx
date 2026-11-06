/** Commissioning checklist: evaluated by the engine from the simulated installation and the
 * I/O tests performed in this session. The view only displays it. */
import { useState } from "react";
import { api, type CheckItemDto, type CommissioningDto } from "@plc-lab/ipc";
import { useInterface } from "../locale";
import type { TranslationKey } from "../locale/en";
import { backendText, translateCode } from "../locale/messages";
import { Badge, ErrorNotice } from "../ui";
import { useEvidence, useFrameQuery } from "./useFrameQuery";
import type { Lab } from "./useLab";

type StateStyle = {
  badge: string;
  label: TranslationKey;
  help: TranslationKey;
};
const NOT_APPLICABLE: StateStyle = {
  badge: "normal",
  label: "check.state.NOT_APPLICABLE",
  help: "check.help.NOT_APPLICABLE",
};
const STATE: Record<string, StateStyle | undefined> = {
  OK: { badge: "active", label: "check.state.OK", help: "check.help.OK" },
  ATTENTION: {
    badge: "warning",
    label: "check.state.ATTENTION",
    help: "check.help.ATTENTION",
  },
  FAULT: {
    badge: "fault",
    label: "check.state.FAULT",
    help: "check.help.FAULT",
  },
  UNTESTED: {
    badge: "stopped",
    label: "check.state.UNTESTED",
    help: "check.help.UNTESTED",
  },
  NOT_APPLICABLE,
};

function StateBadge({ state }: { state: string }) {
  const { t } = useInterface();
  const s = STATE[state] ?? NOT_APPLICABLE;
  return (
    <Badge kind={s.badge} title={t(s.help)}>
      {t(s.label)}
    </Badge>
  );
}

function Item({ item }: { item: CheckItemDto }) {
  const { t, preferences } = useInterface();
  const title = translateCode(t, `check.item.${item.id}`, item.title);
  const findings = item.findingMessages.length
    ? item.findingMessages.map((m) => backendText(preferences.language, m))
    : item.findings;
  const how = backendText(preferences.language, item.howMessage, item.how);
  return (
    <li className="check-item" aria-label={`${title}: ${item.state}`}>
      <div className="row" style={{ gap: 8 }}>
        <StateBadge state={item.state} />
        <strong>{title}</strong>
      </div>
      <ul className="check-findings">
        {findings.map((f, i) => (
          <li key={i}>{f}</li>
        ))}
      </ul>
      {how && item.state !== "OK" && <p className="muted">{how}</p>}
    </li>
  );
}

export function CommissioningTab({ lab }: { lab: Lab }) {
  const { t } = useInterface();
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
      aria-label={t("check.title")}
    >
      <ErrorNotice error={list.error} />
      <div className="row wrap" style={{ gap: 8 }}>
        <p className="muted" style={{ margin: 0 }}>
          {t("check.intro", { seconds: list.data?.feedbackWaitS ?? 3 })}
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
          const title = translateCode(t, `check.category.${c.id}`, c.title);
          return (
            <section key={c.id} className="panel" aria-label={title}>
              <button
                className="panel-head check-head"
                aria-expanded={expanded}
                onClick={() => toggle(c.id)}
              >
                <h4>{title.toUpperCase()}</h4>
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
