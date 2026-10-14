import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { getTracePage } from "@plc-lab/ipc";
import type { TracePageDto } from "@plc-lab/ipc";
import { mergeTracePage } from "./trace-history";
import { coalescedRefresh } from "./coalesced-refresh";

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
  const history = useRef<TracePageDto | null>(null),
    generation = useRef(0n),
    context = useRef({ sessionId, current }),
    reader = useRef<ReturnType<typeof coalescedRefresh> | null>(null);
  useLayoutEffect(() => {
    generation.current += 1n;
    context.current = { sessionId, current };
    if (history.current?.sessionId !== sessionId) {
      history.current = null;
      setView({ page: null, pending: false, error: null });
    }
    if (!current) setView((v) => ({ ...v, pending: false }));
  }, [sessionId, current]);
  useEffect(() => {
    let active = true;
    const refresh = coalescedRefresh(async () => {
      const { sessionId, current } = context.current,
        token = generation.current;
      if (!current || sessionId === null) return;
      const valid = () => active && generation.current === token;
      setView((v) => ({ ...v, pending: true, error: null }));
      try {
        for (let pageNumber = 0; pageNumber < 16; pageNumber++) {
          const known = history.current,
            cursor = known?.entries.at(-1)?.sequence ?? null;
          const page = await getTracePage(sessionId, cursor, 256);
          if (!valid()) return;
          if (page.sessionId !== sessionId)
            throw new Error("Trace response belongs to another session.");
          history.current = mergeTracePage(known, page);
          const more =
            history.current.entries.at(-1)?.sequence !== page.latestSequence;
          setView({ page: history.current, pending: more, error: null });
          if (!more) return;
        }
        // Run may append while these bounded pages are read. Continue through
        // the same coalescer instead of mistaking new events for invalid data.
        refresh.trigger();
      } catch (error) {
        if (valid())
          setView((v) => ({
            ...v,
            pending: false,
            error:
              error instanceof Error ? error.message : "Trace query failed.",
          }));
      }
    });
    reader.current = refresh;
    return () => {
      active = false;
      generation.current += 1n;
      refresh.dispose();
      reader.current = null;
    };
  }, []);
  useEffect(() => {
    reader.current?.trigger();
  }, [sessionId, observationKey, current]);
  return {
    ...view,
    page: view.page?.sessionId === sessionId ? view.page : null,
  };
}
