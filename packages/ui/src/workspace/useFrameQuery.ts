import { useEffect, useState } from "react";
import { api } from "@plc-lab/ipc";
import { useCoalescedRead } from "../useCoalescedRead";
import type { Lab } from "./useLab";

/** Re-read a backend projection whenever the authoritative frame changes (coalesced). */
export function useFrameQuery<T>(
  lab: Lab,
  read: () => Promise<T>,
  extra: readonly unknown[] = [],
) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<unknown>(null);
  useCoalescedRead(async () => {
    try {
      setData(await read());
      setError(null);
    } catch (e) {
      setError(e);
    }
  }, [lab.frame, ...extra]);
  return { data, error };
}

/** Record that the student opened an evidence source (troubleshooting exercises only). */
export function useEvidence(lab: Lab, evidence: string) {
  const ws = lab.ws;
  const troubleshooting = lab.workspace?.exercise?.kind === "troubleshooting";
  useEffect(() => {
    if (ws && troubleshooting)
      void api.noteEvidence(ws, evidence).catch(() => undefined);
  }, [ws, troubleshooting, evidence]);
}
