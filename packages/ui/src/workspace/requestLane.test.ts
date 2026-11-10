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

  it("sends each request only after the previous one was answered", async () => {
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

  it("a slow request is never overtaken: the next one waits for its late answer", async () => {
    vi.useFakeTimers();
    const lane = createLane(5000);
    const applied: string[] = [];
    const press = deferred<void>();
    const pressed = lane.run(async () => {
      await press.promise;
      applied.push("press");
    });
    const released = lane.run(async () => {
      applied.push("release");
    });
    // Long past the stall time the release is still not sent.
    await vi.advanceTimersByTimeAsync(60_000);
    expect(applied).toEqual([]);
    press.resolve();
    await pressed;
    await released;
    expect(applied).toEqual(["press", "release"]);
  });

  it("reports a stall with the requests waiting behind it, then the late answer", async () => {
    vi.useFakeTimers();
    let clock = 0;
    const lane = createLane(5000, () => clock);
    const reports: string[] = [];
    const build = deferred<void>();
    const building = lane.run(() => build.promise, {
      onStall: (waiting) => reports.push(`stalled, ${waiting} waiting`),
      onLateAnswer: (ms) => reports.push(`answered after ${ms} ms`),
    });
    void lane.run(async () => "release");
    void lane.run(async () => "press");
    await vi.advanceTimersByTimeAsync(4999);
    expect(reports).toEqual([]);
    await vi.advanceTimersByTimeAsync(1);
    expect(reports).toEqual(["stalled, 2 waiting"]);
    clock = 7000;
    build.resolve();
    await building;
    expect(reports).toEqual(["stalled, 2 waiting", "answered after 7000 ms"]);
  });

  it("a request answered in time is not reported", async () => {
    vi.useFakeTimers();
    const lane = createLane(5000);
    const onStall = vi.fn();
    const onLateAnswer = vi.fn();
    await lane.run(async () => "fast", { onStall, onLateAnswer });
    await vi.advanceTimersByTimeAsync(10_000);
    expect(onStall).not.toHaveBeenCalled();
    expect(onLateAnswer).not.toHaveBeenCalled();
  });

  it("is idle again once everything was answered", async () => {
    const lane = createLane(1000);
    await lane.run(async () => "first");
    await flush();
    const sent: string[] = [];
    void lane.run(async () => sent.push("second"));
    expect(sent).toEqual(["second"]);
  });
});
