//! Deterministic bounded observations. This crate never executes a PLC or process.
use plc_types::PlcTime;
use std::{
    collections::VecDeque,
    num::{NonZeroU64, NonZeroUsize},
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TraceSequenceId(NonZeroU64);
impl TraceSequenceId {
    pub fn new(value: u64) -> Option<Self> {
        NonZeroU64::new(value).map(Self)
    }
    pub fn get(self) -> u64 {
        self.0.get()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TracePoint {
    pub completed_scans: u64,
    pub simulation_time: PlcTime,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEntry<T> {
    pub sequence: TraceSequenceId,
    pub point: TracePoint,
    pub payload: T,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceMetadata {
    pub capacity: usize,
    pub oldest_sequence: Option<TraceSequenceId>,
    pub latest_sequence: Option<TraceSequenceId>,
    pub dropped_count: u64,
    pub saturated: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub struct TracePage<'a, T> {
    pub entries: Vec<&'a TraceEntry<T>>,
    pub metadata: TraceMetadata,
    pub gap: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FutureCursor;
impl std::fmt::Display for FutureCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("The trace cursor is newer than this history.")
    }
}
impl std::error::Error for FutureCursor {}

#[derive(Debug)]
pub struct TraceBuffer<T> {
    entries: VecDeque<TraceEntry<T>>,
    capacity: NonZeroUsize,
    next: Option<TraceSequenceId>,
    dropped_count: u64,
}
impl<T> TraceBuffer<T> {
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity.get()),
            capacity,
            next: TraceSequenceId::new(1),
            dropped_count: 0,
        }
    }
    /// Saturation or eviction never returns an execution error. No ID is reused.
    pub fn append(&mut self, point: TracePoint, payload: T) -> Option<TraceSequenceId> {
        let sequence = self.next?;
        self.next = sequence.get().checked_add(1).and_then(TraceSequenceId::new);
        if self.entries.len() == self.capacity.get() {
            self.entries.pop_front();
            // At most MAX-1 evictions: sequence starts at 1 and capacity is positive.
            self.dropped_count = self.dropped_count.saturating_add(1);
        }
        self.entries.push_back(TraceEntry {
            sequence,
            point,
            payload,
        });
        Some(sequence)
    }
    pub fn metadata(&self) -> TraceMetadata {
        TraceMetadata {
            capacity: self.capacity.get(),
            oldest_sequence: self.entries.front().map(|e| e.sequence),
            latest_sequence: self.entries.back().map(|e| e.sequence),
            dropped_count: self.dropped_count,
            saturated: self.next.is_none(),
        }
    }
    pub fn page(
        &self,
        after: Option<TraceSequenceId>,
        limit: NonZeroUsize,
    ) -> Result<TracePage<'_, T>, FutureCursor> {
        let metadata = self.metadata();
        if after.is_some_and(|cursor| {
            metadata
                .latest_sequence
                .is_none_or(|latest| cursor > latest)
        }) {
            return Err(FutureCursor);
        }
        // A cursor immediately before oldest loses no entries requested after it.
        let gap = after.is_some_and(|cursor| {
            metadata
                .oldest_sequence
                .is_some_and(|oldest| cursor.get() < oldest.get() - 1)
        });
        let entries = self
            .entries
            .iter()
            .filter(|e| after.is_none_or(|cursor| e.sequence > cursor))
            .take(limit.get())
            .collect();
        Ok(TracePage {
            entries,
            metadata,
            gap,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn capacity(n: usize) -> NonZeroUsize {
        NonZeroUsize::new(n).unwrap()
    }
    fn point(scan: u64, time: u64) -> TracePoint {
        TracePoint {
            completed_scans: scan,
            simulation_time: PlcTime::from_micros(time),
        }
    }
    #[test]
    fn sequence_is_nonzero_and_exact_at_u64_max() {
        assert_eq!(TraceSequenceId::new(0), None);
        assert_eq!(TraceSequenceId::new(u64::MAX).unwrap().get(), u64::MAX);
    }
    #[test]
    fn empty_history_metadata_and_query_are_explicit() {
        let b = TraceBuffer::<bool>::new(capacity(3));
        let p = b.page(None, capacity(2)).unwrap();
        assert!(p.entries.is_empty());
        assert!(!p.gap);
        assert_eq!(p.metadata.oldest_sequence, None);
        assert_eq!(p.metadata.latest_sequence, None);
        assert_eq!(p.metadata.dropped_count, 0);
        assert!(!p.metadata.saturated);
        assert_eq!(
            b.page(TraceSequenceId::new(1), capacity(1)),
            Err(FutureCursor)
        );
    }
    #[test]
    fn ordering_is_sequence_even_when_scan_and_time_reset_or_repeat() {
        let mut b = TraceBuffer::new(capacity(5));
        for p in [point(0, 0), point(0, 0), point(8, 800000), point(0, 0)] {
            b.append(p, "observation");
        }
        let page = b.page(None, capacity(5)).unwrap();
        assert_eq!(
            page.entries
                .iter()
                .map(|e| e.sequence.get())
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
        assert_eq!(page.entries[3].point, point(0, 0));
    }
    #[test]
    fn overflow_drops_oldest_without_renumbering_and_reports_exact_gap() {
        let mut b = TraceBuffer::new(capacity(3));
        for n in 1..=7 {
            b.append(point(n, n * 100000), n);
        }
        let p = b.page(TraceSequenceId::new(2), capacity(2)).unwrap();
        assert!(p.gap);
        assert_eq!(p.metadata.dropped_count, 4);
        assert_eq!(p.metadata.oldest_sequence.unwrap().get(), 5);
        assert_eq!(p.metadata.latest_sequence.unwrap().get(), 7);
        assert_eq!(
            p.entries
                .iter()
                .map(|e| (e.sequence.get(), e.payload))
                .collect::<Vec<_>>(),
            vec![(5, 5), (6, 6)]
        );
        assert!(!b.page(TraceSequenceId::new(4), capacity(3)).unwrap().gap);
        assert!(!b.page(None, capacity(3)).unwrap().gap);
        assert!(
            b.page(TraceSequenceId::new(7), capacity(2))
                .unwrap()
                .entries
                .is_empty()
        );
        assert_eq!(
            b.page(TraceSequenceId::new(8), capacity(1)),
            Err(FutureCursor)
        );
    }
    #[test]
    fn saturation_keeps_max_id_and_history_without_wrapping_or_panicking() {
        let mut b = TraceBuffer::new(capacity(2));
        b.next = TraceSequenceId::new(u64::MAX - 1);
        assert_eq!(
            b.append(point(1, 100000), "before").unwrap().get(),
            u64::MAX - 1
        );
        assert!(!b.metadata().saturated);
        assert_eq!(b.append(point(2, 200000), "last").unwrap().get(), u64::MAX);
        let metadata = b.metadata();
        assert!(metadata.saturated);
        assert_eq!(b.append(point(3, 300000), "unrecorded"), None);
        assert_eq!(b.metadata(), metadata);
        assert_eq!(
            b.page(None, capacity(2)).unwrap().entries[1].payload,
            "last"
        );
    }
    #[test]
    fn repeated_typed_actions_produce_equal_history() {
        let execute = || {
            let mut b = TraceBuffer::new(capacity(2));
            for n in 0..5 {
                b.append(point(n, n * 100000), (n, n % 2 == 0));
            }
            b
        };
        let a = execute();
        let b = execute();
        assert_eq!(a.page(None, capacity(2)), b.page(None, capacity(2)));
    }
}
