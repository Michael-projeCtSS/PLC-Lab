/**
 * PLC start operations (ADR-0038): warm restart, cold restart and memory reset. Each one
 * initializes simulated program data, so each asks for confirmation; the backend performs
 * it with the program and configuration already in the CPU. The view only asks and shows
 * the CPU's start state.
 */
import { api, type LiveFrameDto } from "@plc-lab/ipc";
import { useState } from "react";
import { useInterface } from "../locale";
import type { TranslationKey } from "../locale/en";
import { Modal } from "../ui";
import type { Lab } from "./useLab";

type Operation = "warm" | "cold" | "reset";
const LABEL: Record<Operation, TranslationKey> = {
  warm: "control.warmRestart",
  cold: "control.coldRestart",
  reset: "control.reset",
};
const HELP: Record<Operation, TranslationKey> = {
  warm: "help.warmRestart",
  cold: "help.coldRestart",
  reset: "help.reset",
};
const QUESTION: Record<Operation, TranslationKey> = {
  warm: "confirm.warmRestart",
  cold: "confirm.coldRestart",
  reset: "confirm.reset",
};

export function RestartControls({
  lab,
  advanced,
}: {
  lab: Lab;
  advanced: boolean;
}) {
  const { t } = useInterface();
  const [asking, setAsking] = useState<Operation | null>(null);
  const { frame, ws } = lab;
  if (!frame || !ws) return null;
  const faulted = frame.plc.mode === "FAULT";
  const running = frame.plc.mode === "RUN";
  // Basic mode offers only the memory reset a faulted CPU needs.
  if (!advanced && !faulted) return null;
  const operations: Operation[] = advanced
    ? ["warm", "cold", "reset"]
    : ["reset"];
  const perform = (operation: Operation) => {
    setAsking(null);
    void lab.act(
      "restart",
      (): Promise<LiveFrameDto> =>
        operation === "reset"
          ? api.plcReset(ws)
          : api.plcRestart(ws, operation === "warm" ? "WARM" : "COLD"),
      (f) => f,
    );
  };
  const start = frame.plc.lastStart;
  return (
    <>
      {operations.map((operation) => {
        const needsRun = operation !== "reset" && !running;
        return (
          <button
            key={operation}
            className={`btn btn-sm${operation === "reset" && faulted ? " btn-danger" : ""}`}
            disabled={lab.busy !== null || needsRun}
            title={needsRun ? t("plc.restartNeedsRun") : t(HELP[operation])}
            onClick={() => setAsking(operation)}
          >
            {t(LABEL[operation])}
          </button>
        );
      })}
      {advanced && (
        <span
          className="mono muted"
          role="status"
          aria-label={t("plc.startState")}
        >
          {start
            ? t("plc.lastStart", {
                kind: t(start === "WARM" ? "plc.start.WARM" : "plc.start.COLD"),
              })
            : t("plc.notStarted")}
          {" · "}
          {t("plc.retained", { count: frame.plc.retainedVariables })}
          {frame.plc.firstScanPending && <> · {t("plc.firstScanPending")}</>}
        </span>
      )}
      {asking && (
        <Modal title={t(QUESTION[asking])} onClose={() => setAsking(null)}>
          <p>{t(HELP[asking])}</p>
          <div className="row" style={{ justifyContent: "flex-end" }}>
            <button className="btn" onClick={() => setAsking(null)}>
              {t("common.cancel")}
            </button>
            <button className="btn btn-primary" onClick={() => perform(asking)}>
              {t(LABEL[asking])}
            </button>
          </div>
        </Modal>
      )}
    </>
  );
}
