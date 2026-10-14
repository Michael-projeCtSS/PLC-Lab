import type { TracePageDto } from "@plc-lab/ipc";
/** Merge ordered presentation history; never reconstruct an observation. */
export function mergeTracePage(
  previous: TracePageDto | null,
  page: TracePageDto,
): TracePageDto {
  const events = new Map(
    (previous?.sessionId === page.sessionId ? previous.entries : []).map(
      (e) => [e.sequence, e],
    ),
  );
  for (const event of page.entries) events.set(event.sequence, event);
  const entries = [...events.values()]
    .filter(
      (e) =>
        page.oldestSequence !== null &&
        BigInt(e.sequence) >= BigInt(page.oldestSequence),
    )
    .sort((a, b) => (BigInt(a.sequence) < BigInt(b.sequence) ? -1 : 1));
  return {
    ...page,
    entries: entries.slice(-page.capacity),
    gap: page.gap || (previous?.sessionId === page.sessionId && previous.gap),
  };
}
