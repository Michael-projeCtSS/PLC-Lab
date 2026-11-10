/**
 * The order in which one workspace's state-changing requests reach the backend
 * (ADR-0048). The desktop host runs each request on its own worker, and the workers then
 * wait for the one workspace lock, which grants no order among waiters: two requests in
 * flight at once may be applied in either order. A lane therefore sends a request only
 * after the previous one has been answered, and the host answers only after the use
 * case has finished applying its change. Requests are applied strictly in the order they
 * were made. An idle lane sends at once, in the same task as the gesture.
 *
 * A request is never skipped or overtaken, also when the backend is slow: sending the
 * next one early could not make it run sooner (it needs the same lock), but it could
 * make it run first. A request without an answer after `stallMs` is reported through
 * `onStall` with the number of requests waiting behind it, and its late answer through
 * `onLateAnswer`.
 */
export type LaneEvents = {
  /** No answer after the stall time; `waiting` requests are queued behind it. */
  onStall?: (waiting: number) => void;
  /** A request reported as stalled was answered after `ms` milliseconds. */
  onLateAnswer?: (ms: number) => void;
};

export type Lane = {
  run<T>(work: () => Promise<T>, events?: LaneEvents): Promise<T>;
};

export function createLane(
  stallMs: number,
  now: () => number = () => Date.now(),
): Lane {
  // Requests made and not yet answered, and the answer of the last one.
  let unanswered = 0;
  let tail: Promise<void> = Promise.resolve();
  return {
    run<T>(work: () => Promise<T>, events: LaneEvents = {}): Promise<T> {
      const start = (): Promise<T> => {
        const sent = now();
        let stalled = false;
        const timer = setTimeout(() => {
          stalled = true;
          events.onStall?.(unanswered - 1);
        }, stallMs);
        const request = new Promise<T>((resolve) => resolve(work()));
        const answered = () => {
          clearTimeout(timer);
          if (stalled) events.onLateAnswer?.(now() - sent);
        };
        request.then(answered, answered);
        return request;
      };
      const result = unanswered === 0 ? start() : tail.then(start);
      unanswered += 1;
      tail = result.then(
        () => {
          unanswered -= 1;
        },
        () => {
          unanswered -= 1;
        },
      );
      return result;
    },
  };
}
