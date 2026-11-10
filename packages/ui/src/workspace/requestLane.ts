/**
 * The order in which one workspace's state-changing requests reach the backend
 * (ADR-0048). The desktop host runs concurrent requests on separate workers, so two
 * requests sent at once may be applied in either order. A lane sends each request only
 * after the previous one settled, so operator gestures and commands are applied in the
 * order they were made. An idle lane sends at once, in the same task as the gesture.
 *
 * A request that has not settled after `stallMs` stops holding the lane: a backend that
 * never answers cannot block later requests forever. `onStall` reports it, and the
 * request still settles for its caller whenever the answer comes.
 */
export type Lane = {
  run<T>(work: () => Promise<T>, onStall?: () => void): Promise<T>;
};

export function createLane(stallMs: number): Lane {
  // Requests made and not yet released from the lane, and the release of the last one.
  let waiting = 0;
  let tail: Promise<void> = Promise.resolve();
  return {
    run<T>(work: () => Promise<T>, onStall?: () => void): Promise<T> {
      let free!: () => void;
      const released = new Promise<void>((resolve) => (free = resolve));
      const start = (): Promise<T> => {
        const timer = setTimeout(() => {
          free();
          onStall?.();
        }, stallMs);
        const request = new Promise<T>((resolve) => resolve(work()));
        const settle = () => {
          clearTimeout(timer);
          free();
        };
        request.then(settle, settle);
        return request;
      };
      const result = waiting === 0 ? start() : tail.then(start);
      waiting += 1;
      tail = released.then(() => {
        waiting -= 1;
      });
      return result;
    },
  };
}
