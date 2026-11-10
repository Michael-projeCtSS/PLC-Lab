import { afterEach, describe, expect, it, vi } from "vitest";
import { createLane } from "./requestLane";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}
/** Let every pending promise reaction run. */
const flush = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

afterEach(() => vi.useRealTimers());

describe("request lane", () => {
  it("sends at once when idle, in the same task", () => {
    const lane = createLane(1000);
    const sent: string[] = [];
    void lane.run(async () => sent.push("press"));
    expect(sent).toEqual(["press"]);
  });

  it("sends each request only after the previous one settled", async () => {
    const lane = createLane(1000);
    const sent: string[] = [];
    const press = deferred<string>();
    const results = [
      lane.run(() => {
        sent.push("press");
        return press.promise;
      }),
      lane.run(async () => {
        sent.push("release");
        return "released";
      }),
      lane.run(async () => {
        sent.push("step");
        return "stepped";
      }),
    ];
    await flush();
    expect(sent).toEqual(["press"]);
    press.resolve("pressed");
    expect(await Promise.all(results)).toEqual([
      "pressed",
      "released",
      "stepped",
    ]);
    expect(sent).toEqual(["press", "release", "step"]);
  });

  it("a failed request does not stop the requests after it", async () => {
    const lane = createLane(1000);
    const failed = lane.run(() => Promise.reject(new Error("refused")));
    const thrown = lane.run(() => {
      throw new Error("thrown");
    });
    const next = lane.run(async () => "sent");
    await expect(failed).rejects.toThrow("refused");
    await expect(thrown).rejects.toThrow("thrown");
    expect(await next).toBe("sent");
  });

  it("a request that never answers stops holding the lane after the limit", async () => {
    vi.useFakeTimers();
    const lane = createLane(5000);
    const stalls: string[] = [];
    const hung = lane.run(
      () => new Promise<never>(() => undefined),
      () => stalls.push("build"),
    );
    const sent: string[] = [];
    const release = lane.run(async () => sent.push("release"));
    await vi.advanceTimersByTimeAsync(4999);
    expect(sent).toEqual([]);
    expect(stalls).toEqual([]);
    await vi.advanceTimersByTimeAsync(1);
    expect(stalls).toEqual(["build"]);
    await release;
    expect(sent).toEqual(["release"]);
    void hung;
  });

  it("a request that answers in time is not reported as stalled", async () => {
    vi.useFakeTimers();
    const lane = createLane(5000);
    const onStall = vi.fn();
    await lane.run(async () => "fast", onStall);
    await vi.advanceTimersByTimeAsync(10_000);
    expect(onStall).not.toHaveBeenCalled();
  });

  it("is idle again once everything settled", async () => {
    const lane = createLane(1000);
    await lane.run(async () => "first");
    await flush();
    const sent: string[] = [];
    void lane.run(async () => sent.push("second"));
    expect(sent).toEqual(["second"]);
  });
});
