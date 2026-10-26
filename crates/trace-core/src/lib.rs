//! Deterministic, byte-bounded observation storage. This crate never executes a PLC,
//! a process model or a network; it only stores what the engine observed.
//!
//! Two stores exist:
//! - [`SignalTrace`]: per-record layered signal values. Records are grouped into chunks.
//!   The first record of a chunk is a keyframe (full layer state); every other record
//!   stores only the values that changed in each layer. Signal metadata (names, types,
//!   roles) is never stored here: callers keep one descriptor table and reference stable
//!   numeric signal IDs.
//! - [`EventLog`]: discrete, sequenced events bounded by count and estimated bytes.
//!
//! Both stores evict the oldest data first, never renumber retained entries, and report
//! exactly how much history was discarded. Eviction never influences execution.

use std::collections::VecDeque;
use std::num::NonZeroUsize;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Stable caller-owned signal identity.
pub type SignalKey = u32;

/// Values stored in a [`SignalTrace`]. `encoded_size` is the accounting cost per stored value.
pub trait TraceValue: Copy + PartialEq + std::fmt::Debug {
    fn encoded_size(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Sequence number of a stored record. Monotonic per store, never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecordSeq(pub u64);

/// One layer of signal values ordered by ascending key, with unique keys.
pub type Layer<V> = Vec<(SignalKey, V)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceError {
    /// Layer count differs from the store configuration.
    LayerCount,
    /// A layer is not strictly ordered by key.
    Unordered,
    /// Sequence space exhausted (unreachable in practice).
    Exhausted,
}
impl std::fmt::Display for TraceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::LayerCount => "The record layer count differs from the trace configuration.",
            Self::Unordered => "Trace layers must be strictly ordered by signal key.",
            Self::Exhausted => "The trace record sequence is exhausted.",
        })
    }
}
impl std::error::Error for TraceError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceConfig {
    pub layers: usize,
    pub byte_budget: usize,
    pub keyframe_interval: NonZeroUsize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredRecord<H, V> {
    seq: RecordSeq,
    header: H,
    /// Full layers for a keyframe; changed values only otherwise.
    layers: Vec<Layer<V>>,
}

#[derive(Debug, Clone)]
struct Chunk<H, V> {
    records: Vec<StoredRecord<H, V>>,
    bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceStats {
    pub retained_records: usize,
    pub oldest: Option<RecordSeq>,
    pub latest: Option<RecordSeq>,
    pub dropped_records: u64,
    pub used_bytes: usize,
    pub byte_budget: usize,
}

/// Byte-bounded layered signal history.
#[derive(Debug, Clone)]
pub struct SignalTrace<H, V> {
    config: TraceConfig,
    chunks: VecDeque<Chunk<H, V>>,
    /// Full state after the latest record, used to compute the next deltas.
    current: Vec<Layer<V>>,
    next: u64,
    dropped: u64,
    used: usize,
}

fn layer_bytes<V: TraceValue>(layer: &Layer<V>) -> usize {
    24 + layer
        .iter()
        .map(|(_, v)| 4 + v.encoded_size())
        .sum::<usize>()
}
fn ordered<V>(layer: &Layer<V>) -> bool {
    layer.windows(2).all(|w| w[0].0 < w[1].0)
}
fn same_keys<V>(a: &Layer<V>, b: &Layer<V>) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.0 == y.0)
}

impl<H: Clone, V: TraceValue> SignalTrace<H, V> {
    pub fn new(config: TraceConfig) -> Self {
        Self {
            config,
            chunks: VecDeque::new(),
            current: Vec::new(),
            next: 1,
            dropped: 0,
            used: 0,
        }
    }
    pub fn config(&self) -> TraceConfig {
        self.config
    }
    /// Append one record. Each layer must be the complete, key-ordered state of that layer.
    pub fn append(&mut self, header: H, layers: Vec<Layer<V>>) -> Result<RecordSeq, TraceError> {
        if layers.len() != self.config.layers {
            return Err(TraceError::LayerCount);
        }
        if !layers.iter().all(ordered) {
            return Err(TraceError::Unordered);
        }
        let seq = RecordSeq(self.next);
        self.next = self.next.checked_add(1).ok_or(TraceError::Exhausted)?;
        let keys_changed = self.current.len() != layers.len()
            || self.current.iter().zip(&layers).any(|(a, b)| !same_keys(a, b));
        let start_chunk = keys_changed
            || self.chunks.back().is_none_or(|c| {
                c.records.len() >= self.config.keyframe_interval.get()
            });
        let stored = if start_chunk {
            layers.clone()
        } else {
            self.current
                .iter()
                .zip(&layers)
                .map(|(previous, next)| {
                    previous
                        .iter()
                        .zip(next)
                        .filter(|(p, n)| p.1 != n.1)
                        .map(|(_, n)| *n)
                        .collect()
                })
                .collect()
        };
        let bytes = 48 + std::mem::size_of::<H>() + stored.iter().map(layer_bytes).sum::<usize>();
        if start_chunk {
            self.chunks.push_back(Chunk {
                records: Vec::new(),
                bytes: 0,
            });
        }
        let chunk = self.chunks.back_mut().expect("chunk exists after push");
        chunk.records.push(StoredRecord {
            seq,
            header,
            layers: stored,
        });
        chunk.bytes += bytes;
        self.used += bytes;
        self.current = layers;
        // Evict whole chunks; the newest chunk is always retained.
        while self.used > self.config.byte_budget && self.chunks.len() > 1 {
            let old = self.chunks.pop_front().expect("more than one chunk");
            self.used -= old.bytes;
            self.dropped += old.records.len() as u64;
        }
        Ok(seq)
    }
    pub fn stats(&self) -> TraceStats {
        TraceStats {
            retained_records: self.chunks.iter().map(|c| c.records.len()).sum(),
            oldest: self
                .chunks
                .front()
                .and_then(|c| c.records.first())
                .map(|r| r.seq),
            latest: self
                .chunks
                .back()
                .and_then(|c| c.records.last())
                .map(|r| r.seq),
            dropped_records: self.dropped,
            used_bytes: self.used,
            byte_budget: self.config.byte_budget,
        }
    }
    /// Headers of retained records in order.
    pub fn headers(&self) -> impl DoubleEndedIterator<Item = (RecordSeq, &H)> + '_ {
        self.chunks
            .iter()
            .flat_map(|c| c.records.iter().map(|r| (r.seq, &r.header)))
    }
    fn locate(&self, seq: RecordSeq) -> Option<(usize, usize)> {
        let chunk = self
            .chunks
            .iter()
            .position(|c| c.records.last().is_some_and(|r| r.seq >= seq))?;
        let first = self.chunks[chunk].records.first()?.seq;
        if seq < first {
            return None;
        }
        let index = usize::try_from(seq.0 - first.0).ok()?;
        (index < self.chunks[chunk].records.len()).then_some((chunk, index))
    }
    /// Reconstruct the complete layers of one retained record.
    pub fn record(&self, seq: RecordSeq) -> Option<(&H, Vec<Layer<V>>)> {
        let (chunk, index) = self.locate(seq)?;
        let records = &self.chunks[chunk].records;
        let mut state = records[0].layers.clone();
        for record in &records[1..=index] {
            for (layer, delta) in state.iter_mut().zip(&record.layers) {
                apply(layer, delta);
            }
        }
        Some((&records[index].header, state))
    }
    /// Change points of the selected keys in one layer between two retained sequences.
    /// The first retained record in range always yields a point for every present key.
    pub fn series(
        &self,
        layer: usize,
        keys: &[SignalKey],
        from: RecordSeq,
        to: RecordSeq,
    ) -> Vec<Vec<(RecordSeq, V)>> {
        let mut out = vec![Vec::new(); keys.len()];
        if layer >= self.config.layers {
            return out;
        }
        for chunk in &self.chunks {
            let (Some(first), Some(last)) = (chunk.records.first(), chunk.records.last()) else {
                continue;
            };
            if last.seq < from || first.seq > to {
                continue;
            }
            let mut state = first.layers[layer].clone();
            for (i, record) in chunk.records.iter().enumerate() {
                if i > 0 {
                    apply(&mut state, &record.layers[layer]);
                }
                if record.seq < from || record.seq > to {
                    continue;
                }
                for (slot, key) in keys.iter().enumerate() {
                    let Ok(position) = state.binary_search_by_key(key, |e| e.0) else {
                        continue;
                    };
                    let value = state[position].1;
                    let series = &mut out[slot];
                    let changed = series.last().is_none_or(|(_, v)| *v != value);
                    let chunk_start = i == 0;
                    if changed || chunk_start && series.is_empty() {
                        series.push((record.seq, value));
                    }
                }
            }
        }
        out
    }
    pub fn clear(&mut self) {
        let dropped: u64 = self.chunks.iter().map(|c| c.records.len() as u64).sum();
        self.dropped += dropped;
        self.chunks.clear();
        self.current.clear();
        self.used = 0;
    }
}

fn apply<V: Copy>(layer: &mut Layer<V>, delta: &Layer<V>) {
    for (key, value) in delta {
        if let Ok(position) = layer.binary_search_by_key(key, |e| e.0) {
            layer[position].1 = *value;
        }
    }
}

/// Events stored in an [`EventLog`].
pub trait TraceEvent: Clone {
    fn encoded_size(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventSeq(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventPage<'a, E> {
    pub entries: Vec<(EventSeq, &'a E)>,
    /// True when the cursor precedes retained history: events were discarded in between.
    pub gap: bool,
    pub oldest: Option<EventSeq>,
    pub latest: Option<EventSeq>,
    pub dropped: u64,
}

/// Bounded discrete event log.
#[derive(Debug, Clone)]
pub struct EventLog<E> {
    entries: VecDeque<(EventSeq, E, usize)>,
    max_entries: NonZeroUsize,
    byte_budget: usize,
    used: usize,
    next: u64,
    dropped: u64,
}

impl<E: TraceEvent> EventLog<E> {
    pub fn new(max_entries: NonZeroUsize, byte_budget: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
            byte_budget,
            used: 0,
            next: 1,
            dropped: 0,
        }
    }
    pub fn append(&mut self, event: E) -> EventSeq {
        let seq = EventSeq(self.next);
        self.next = self.next.saturating_add(1);
        let bytes = 16 + event.encoded_size();
        self.entries.push_back((seq, event, bytes));
        self.used += bytes;
        while self.entries.len() > 1
            && (self.entries.len() > self.max_entries.get() || self.used > self.byte_budget)
        {
            let (_, _, b) = self.entries.pop_front().expect("nonempty");
            self.used -= b;
            self.dropped += 1;
        }
        seq
    }
    pub fn latest(&self) -> Option<EventSeq> {
        self.entries.back().map(|e| e.0)
    }
    pub fn oldest(&self) -> Option<EventSeq> {
        self.entries.front().map(|e| e.0)
    }
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (EventSeq, &E)> + '_ {
        self.entries.iter().map(|(s, e, _)| (*s, e))
    }
    /// Entries strictly after `after`, limited by count and estimated bytes.
    pub fn page(&self, after: Option<EventSeq>, max_entries: usize, max_bytes: usize) -> EventPage<'_, E> {
        let gap = after.is_some_and(|cursor| {
            self.oldest()
                .is_some_and(|oldest| cursor.0.saturating_add(1) < oldest.0)
        });
        let mut entries = Vec::new();
        let mut bytes = 0;
        for (seq, event, size) in &self.entries {
            if after.is_some_and(|c| *seq <= c) {
                continue;
            }
            if entries.len() >= max_entries || (!entries.is_empty() && bytes + size > max_bytes) {
                break;
            }
            bytes += size;
            entries.push((*seq, event));
        }
        EventPage {
            entries,
            gap,
            oldest: self.oldest(),
            latest: self.latest(),
            dropped: self.dropped,
        }
    }
    /// The most recent entries (newest last), limited by count.
    pub fn tail(&self, count: usize) -> Vec<(EventSeq, &E)> {
        let skip = self.entries.len().saturating_sub(count);
        self.entries.iter().skip(skip).map(|(s, e, _)| (*s, e)).collect()
    }
    pub fn clear(&mut self) {
        self.dropped += self.entries.len() as u64;
        self.entries.clear();
        self.used = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl TraceValue for i32 {}
    impl TraceEvent for &'static str {}

    fn config(budget: usize, interval: usize) -> TraceConfig {
        TraceConfig {
            layers: 2,
            byte_budget: budget,
            keyframe_interval: NonZeroUsize::new(interval).unwrap(),
        }
    }

    #[test]
    fn records_reconstruct_exactly_from_keyframes_and_deltas() {
        let mut t: SignalTrace<u64, i32> = SignalTrace::new(config(1 << 20, 4));
        let mut expected = Vec::new();
        for n in 0..10i32 {
            let layers = vec![
                vec![(1, n / 3), (2, 7), (5, n % 2)],
                vec![(3, n), (9, -n / 4)],
            ];
            expected.push(layers.clone());
            t.append(u64::try_from(n).unwrap(), layers).unwrap();
        }
        for (i, layers) in expected.iter().enumerate() {
            let (header, actual) = t.record(RecordSeq(i as u64 + 1)).unwrap();
            assert_eq!(*header, i as u64);
            assert_eq!(&actual, layers);
        }
        assert!(t.record(RecordSeq(11)).is_none());
    }

    #[test]
    fn unchanged_values_cost_no_delta_bytes() {
        let mut small: SignalTrace<(), i32> = SignalTrace::new(config(1 << 30, 64));
        let wide: Layer<i32> = (0..1000).map(|k| (k, 0)).collect();
        small.append((), vec![wide.clone(), Vec::new()]).unwrap();
        let after_key = small.stats().used_bytes;
        for _ in 0..50 {
            small.append((), vec![wide.clone(), Vec::new()]).unwrap();
        }
        let per_record = (small.stats().used_bytes - after_key) / 50;
        assert!(per_record < 200, "steady state costs {per_record} bytes");
    }

    #[test]
    fn byte_budget_evicts_whole_oldest_chunks_and_reports_drops() {
        let mut t: SignalTrace<(), i32> = SignalTrace::new(config(4_000, 8));
        let wide: Layer<i32> = (0..50).map(|k| (k, 0)).collect();
        for n in 0..200 {
            let layer: Layer<i32> = wide.iter().map(|(k, _)| (*k, n)).collect();
            t.append((), vec![layer, Vec::new()]).unwrap();
            assert!(t.stats().used_bytes <= 4_000 + 2_000);
        }
        let s = t.stats();
        assert!(s.dropped_records > 0);
        assert_eq!(s.dropped_records + s.retained_records as u64, 200);
        assert_eq!(s.latest, Some(RecordSeq(200)));
        assert!(t.record(RecordSeq(1)).is_none());
        let (_, last) = t.record(RecordSeq(200)).unwrap();
        assert!(last[0].iter().all(|(_, v)| *v == 199));
    }

    #[test]
    fn key_set_changes_start_a_new_keyframe() {
        let mut t: SignalTrace<(), i32> = SignalTrace::new(config(1 << 20, 100));
        t.append((), vec![vec![(1, 1)], vec![]]).unwrap();
        t.append((), vec![vec![(1, 1), (2, 5)], vec![]]).unwrap();
        t.append((), vec![vec![(1, 2), (2, 5)], vec![]]).unwrap();
        assert_eq!(t.record(RecordSeq(3)).unwrap().1[0], vec![(1, 2), (2, 5)]);
        assert_eq!(t.record(RecordSeq(1)).unwrap().1[0], vec![(1, 1)]);
    }

    #[test]
    fn series_reports_change_points_only() {
        let mut t: SignalTrace<(), i32> = SignalTrace::new(config(1 << 20, 3));
        for v in [0, 0, 1, 1, 1, 0, 0, 2] {
            t.append((), vec![vec![(4, v)], vec![]]).unwrap();
        }
        let s = t.series(0, &[4, 99], RecordSeq(1), RecordSeq(8));
        assert_eq!(
            s[0],
            vec![
                (RecordSeq(1), 0),
                (RecordSeq(3), 1),
                (RecordSeq(6), 0),
                (RecordSeq(8), 2)
            ]
        );
        assert!(s[1].is_empty());
        let window = t.series(0, &[4], RecordSeq(4), RecordSeq(5));
        assert_eq!(window[0], vec![(RecordSeq(4), 1)]);
    }

    #[test]
    fn invalid_layers_are_rejected_without_consuming_sequence() {
        let mut t: SignalTrace<(), i32> = SignalTrace::new(config(1000, 2));
        assert_eq!(t.append((), vec![vec![]]), Err(TraceError::LayerCount));
        assert_eq!(
            t.append((), vec![vec![(2, 0), (1, 0)], vec![]]),
            Err(TraceError::Unordered)
        );
        assert_eq!(t.append((), vec![vec![], vec![]]).unwrap(), RecordSeq(1));
    }

    #[test]
    fn event_log_bounds_by_count_and_bytes_and_reports_gaps() {
        let mut log = EventLog::new(NonZeroUsize::new(3).unwrap(), 1 << 20);
        for e in ["a", "b", "c", "d", "e"] {
            log.append(e);
        }
        assert_eq!(log.len(), 3);
        assert_eq!(log.dropped(), 2);
        let page = log.page(Some(EventSeq(1)), 10, 1 << 20);
        assert!(page.gap);
        assert_eq!(
            page.entries.iter().map(|(_, e)| **e).collect::<Vec<_>>(),
            ["c", "d", "e"]
        );
        assert!(!log.page(Some(EventSeq(2)), 10, 1 << 20).gap);
        let limited = log.page(None, 10, 1);
        assert_eq!(limited.entries.len(), 1, "at least one entry is always returned");
        assert_eq!(log.tail(2).iter().map(|(_, e)| **e).collect::<Vec<_>>(), ["d", "e"]);
    }
}
