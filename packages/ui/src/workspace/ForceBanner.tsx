import { useInterface } from "../locale";
/**
 * The persistent warning while any force is active, with the list of forces, what each
 * one replaces, and Remove / Remove all. Forces belong to the backend's runtime; this view
 * only shows them and sends requests. It is never hidden while a force is active.
 */
import { useState } from "react";
import { api, type ForceListDto } from "@plc-lab/ipc";
import { formatValue } from "../format";
import { Badge } from "../ui";
import { useFrameQuery } from "./useFrameQuery";
import type { Lab } from "./useLab";

const TARGET: Record<string, string> = {
  INPUT_IMAGE: "PLC input image (the program reads this value)",
  OUTPUT_IMAGE: "PLC output image (the module receives this value)",
};

export function ForceBanner({ lab }: { lab: Lab }) {
  const { t } = useInterface();
  const ws = lab.ws;
  const count = lab.frame?.plc.forces ?? 0;
  const [open, setOpen] = useState(false);
  const list = useFrameQuery<ForceListDto | null>(
    lab,
    () => (ws && count > 0 ? api.forces(ws) : Promise.resolve(null)),
    [count],
  );
  if (!ws || count === 0) return null;
  return (
    <div
      className="notice warning"
      role="alert"
      aria-label={t("force.title")}
      style={{ margin: "4px 8px" }}
    >
      <div className="row wrap" style={{ gap: 8 }}>
        <strong>{t("force.active")}</strong>
        <span className="grow">
          {t(count === 1 ? "force.explainOne" : "force.explain", { count })}
        </span>
        <button
          className="btn btn-sm"
          aria-expanded={open}
          onClick={() => setOpen(!open)}
        >
          {open ? t("force.hide") : t("force.show")}
        </button>
        <button
          className="btn btn-sm btn-primary"
          disabled={lab.busy !== null}
          onClick={() =>
            void lab.act(
              "force",
              () => api.removeAllForces(ws),
              (f) => f,
            )
          }
        >
          {t("force.removeAll")}
        </button>
      </div>
      {open && list.data && (
        <table className="table" aria-label="Active forces">
          <thead>
            <tr>
              <th>Signal</th>
              <th>Replaces</th>
              <th>Forced value</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {list.data.forces.map((f) => (
              <tr key={f.signal}>
                <td>
                  {f.label} <span className="dim mono">{f.path}</span>
                </td>
                <td>{TARGET[f.target] ?? f.target}</td>
                <td>
                  <Badge kind="forced">F {formatValue(f.value)}</Badge>
                </td>
                <td>
                  <button
                    className="btn btn-sm btn-ghost"
                    disabled={lab.busy !== null}
                    onClick={() =>
                      void lab.act(
                        "force",
                        () => api.force(ws, f.signal, null),
                        (frame) => frame,
                      )
                    }
                  >
                    {t("force.remove")}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
