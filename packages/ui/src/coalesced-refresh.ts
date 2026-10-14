/** One in-flight read chain plus one dirty bit. Signals never queue promises. */
export function coalescedRefresh(read: () => Promise<void>) {
  let running = false,
    dirty = false,
    disposed = false;
  return {
    trigger() {
      if (disposed) return;
      dirty = true;
      if (running) return;
      running = true;
      void (async () => {
        try {
          while (dirty && !disposed) {
            dirty = false;
            await read();
          }
        } finally {
          running = false;
        }
      })();
    },
    dispose() {
      disposed = true;
      dirty = false;
    },
  };
}
