//! CPU diagnostic buffer: time-stamped incoming/outgoing diagnostic events, bounded.

use lab_model::hardware::{ChannelRef, ModuleType, SlotRef};
use simulation_core::WorldTime;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticSource {
    Cpu,
    Module(SlotRef),
    Channel(ChannelRef),
    /// A configured network connection, identified by its index in the station config.
    Connection(u16),
    Interface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticKind {
    ModuleMissing,
    ModuleMismatch {
        configured: Option<ModuleType>,
        installed: ModuleType,
    },
    UnconfiguredModule,
    NoFieldPower,
    WireBreak,
    Overflow,
    Underflow,
    ConnectionTimeout,
    ConnectionRefused,
    NoRoute,
    ModbusException(u8),
    IpAddressConflict,
    LinkDown,
    PlcFault,
    ForceActive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticEvent {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticEntry {
    pub time: WorldTime,
    pub source: DiagnosticSource,
    pub kind: DiagnosticKind,
    pub event: DiagnosticEvent,
}

/// Fixed-capacity ring buffer, newest last, like a CPU diagnostic buffer.
#[derive(Debug, Clone)]
pub struct DiagnosticBuffer {
    entries: VecDeque<DiagnosticEntry>,
    capacity: usize,
    total: u64,
}

impl DiagnosticBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            capacity: capacity.max(1),
            total: 0,
        }
    }
    pub fn push(&mut self, entry: DiagnosticEntry) {
        if self.entries.len() == self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
        self.total += 1;
    }
    pub fn entries(&self) -> impl DoubleEndedIterator<Item = &DiagnosticEntry> {
        self.entries.iter()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Total entries ever recorded, including overwritten ones.
    pub fn total(&self) -> u64 {
        self.total
    }
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
