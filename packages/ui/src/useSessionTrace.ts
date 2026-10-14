import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { getTracePage } from "@plc-lab/ipc";
import type { TracePageDto } from "@plc-lab/ipc";
import { mergeTracePage } from "./trace-history";

export function useSessionTrace(
  sessionId: string | null,
  observationKey: string,
  current: boolean,
) {
  const [view, setView] = useState<{
    page: TracePageDto | null;
    pending: boolean;
    error: string | null;
  }>({ page: null, pending: false, error: null });
  const history = useRef<TracePageDto | null>(null);
  const generation = useRef(0n);
  useLayoutEffect(() => {
    generation.current += 1n;
    return () => {
      generation.current += 1n;
    };
  }, [sessionId, observationKey, current]);
  useEffect(() => {
    let active = true;
    const token = generation.current;
    const valid = () => active && generation.current === token;
    if (sessionId === null) {
      history.current = null;
      setView({ page: null, pending: false, error: null });
      return;
    }
    if (history.current?.sessionId !== sessionId) {
      history.current = null;
      setView({ page: null, pending: false, error: null });
    }
    if (!current) {
      setView((v) => ({ ...v, pending: false }));
      return;
    }
    setView((v) => ({ ...v, pending: true, error: null }));
    void (async () => {
      try {
        // Request-driven catch-up is bounded by session capacity, never a polling loop.
        for (let pageNumber = 0; pageNumber < 16; pageNumber++) {
          const known = history.current;
          const cursor = known?.entries.at(-1)?.sequence ?? null;
          const page = await getTracePage(sessionId, cursor, 256);
          if (!valid()) return;
          if (page.sessionId !== sessionId)
            throw new Error("Trace response belongs to another session.");
          history.current = mergeTracePage(known, page);
          const last = history.current.entries.at(-1)?.sequence ?? null;
          const more = last !== page.latestSequence;
          setView({ page: history.current, pending: more, error: null });
          if (!more) return;
        }
        throw new Error(
          "The trace response exceeds the bounded session history.",
        );
      } catch (e: unknown) {
        if (valid())
          setView((v) => ({
            ...v,
            pending: false,
            error: e instanceof Error ? e.message : "Trace query failed.",
          }));
      }
    })();
    return () => {
      active = false;
    };
  }, [sessionId, observationKey, current]);
  // A new session must never render an old session's page even before effects run.
  return {
    ...view,
    page: view.page?.sessionId === sessionId ? view.page : null,
  };
}
