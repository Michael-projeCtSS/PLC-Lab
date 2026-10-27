import { useEffect, useRef } from "react";

/**
 * Run `read` whenever `deps` change, with at most one read in flight. Changes that
 * arrive during a read collapse into a single follow-up read with the latest `read`.
 */
export function useCoalescedRead(
  read: () => Promise<void>,
  deps: readonly unknown[],
) {
  const latest = useRef(read);
  const state = useRef({ running: false, dirty: false, alive: true });
  useEffect(() => {
    latest.current = read;
  });
  useEffect(() => {
    const s = state.current;
    s.alive = true;
    return () => {
      s.alive = false;
    };
  }, []);
  useEffect(() => {
    const s = state.current;
    s.dirty = true;
    if (s.running) return;
    s.running = true;
    void (async () => {
      try {
        while (s.dirty && s.alive) {
          s.dirty = false;
          await latest.current();
        }
      } finally {
        s.running = false;
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
}
