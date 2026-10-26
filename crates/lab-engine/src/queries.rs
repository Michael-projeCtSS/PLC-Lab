//! Read-only projections for presentation: cause and effect, variables, trends and
//! history. All values come from authoritative engine state or the trace; nothing is
//! re-evaluated, and every result is bounded.

use crate::{
    plc::{InputSource, TagRole},
    session::{CycleHeader, CycleObservation, LAYER_COMMITTED, LAYER_INPUTS, LAYER_PROCESS, LabSession, Layer},
    signals::{SignalId, SignalKind, SignalValue},
};
use hardware_model::Quality;
use trace_core::RecordSeq;

#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    pub signal: SignalId,
    pub value: SignalValue,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    pub signal: SignalId,
    pub before: Option<SignalValue>,
    pub after: SignalValue,
}

/// "PLC saw -> PLC decided -> process result -> next scan input" for one cycle.
#[derive(Debug, Clone, PartialEq)]
pub struct CauseEffect {
    pub header: CycleHeader,
    pub saw: Vec<Effect>,
    pub decided: Vec<Effect>,
    pub process: Vec<Change>,
    pub next: Vec<Effect>,
    pub fault: Option<plc_types::diagnostics::Diagnostic>,
}

const SECTION_LIMIT: usize = 16;

fn value_in(layer: &Layer, signal: SignalId) -> Option<SignalValue> {
    layer.binary_search_by_key(&signal, |(s, _)| *s).ok().map(|i| layer[i].1)
}

fn effects(layer: &Layer, reference: Option<&Layer>, keep: impl Fn(SignalId) -> bool) -> Vec<Effect> {
    let mut all: Vec<Effect> = layer
        .iter()
        .filter(|(s, _)| keep(*s))
        .map(|(signal, value)| Effect {
            signal: *signal,
            value: *value,
            changed: reference.is_some_and(|r| value_in(r, *signal) != Some(*value)),
        })
        .collect();
    // Changed first, then signal order; bounded for large programs.
    all.sort_by_key(|e| (!e.changed, e.signal));
    all.truncate(SECTION_LIMIT);
    all.sort_by_key(|e| e.signal);
    all
}

pub fn cause_effect(session: &LabSession, current: &CycleObservation, previous: Option<&CycleObservation>) -> CauseEffect {
    let kind = |s: SignalId| session.signals().get(s).map(|d| d.kind);
    let interesting_physical = |s: SignalId| {
        session.signals().get(s).is_some_and(|d| d.kind == SignalKind::Physical && !d.path.ends_with(".capacityMl") && !d.path.ends_with(".color"))
    };
    let mut process: Vec<Change> = current
        .process_after
        .iter()
        .filter(|(s, _)| interesting_physical(*s))
        .filter_map(|(signal, after)| {
            let before = previous.and_then(|p| value_in(&p.process_after, *signal));
            (before != Some(*after)).then_some(Change { signal: *signal, before, after: *after })
        })
        .collect();
    process.truncate(SECTION_LIMIT);
    let next = effects(&current.staged_next, Some(&current.scan_inputs), |_| true);
    CauseEffect {
        header: current.header,
        saw: effects(&current.scan_inputs, previous.map(|p| &p.scan_inputs), |_| true),
        decided: effects(&current.committed, previous.map(|p| &p.committed), |s| {
            matches!(kind(s), Some(SignalKind::PlcOutput | SignalKind::PlcLocal | SignalKind::BlockOutput))
        }),
        process,
        next,
        fault: current.fault.clone(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableRow {
    pub signal: SignalId,
    pub name: String,
    pub kind: SignalKind,
    pub scan_value: Option<SignalValue>,
    pub committed: Option<SignalValue>,
    pub staged: Option<SignalValue>,
    pub source: Option<String>,
    pub quality: Option<Quality>,
    pub forced: Option<SignalValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendLayer {
    Inputs,
    Committed,
    Process,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrendSeries {
    pub signal: SignalId,
    pub layer: TrendLayer,
    pub points: Vec<(u64, SignalValue)>,
    pub decimated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryRow {
    pub header: CycleHeader,
}

impl LabSession {
    pub fn latest_cause_effect(&self) -> Option<CauseEffect> {
        self.last.as_ref().map(|c| cause_effect(self, c, self.previous.as_ref()))
    }

    /// Reconstruct one retained cycle observation from the trace.
    pub fn observation(&self, cycle: u64) -> Option<CycleObservation> {
        let (header, layers) = self.trace().record(self.seq_of(cycle))?;
        let lift = |l: &Vec<(u32, SignalValue)>| l.iter().map(|(k, v)| (SignalId(*k), *v)).collect::<Layer>();
        let fault = self
            .events()
            .iter()
            .rev()
            .find_map(|(_, e)| match &e.kind {
                crate::session::LabEventKind::PlcFault { diagnostic } if e.cycle == cycle => Some(diagnostic.clone()),
                _ => None,
            });
        Some(CycleObservation {
            header: *header,
            scan_inputs: lift(&layers[0]),
            committed: lift(&layers[1]),
            applied: lift(&layers[2]),
            process_after: lift(&layers[3]),
            staged_next: lift(&layers[4]),
            fault,
        })
    }

    pub fn cause_effect_at(&self, cycle: u64) -> Option<CauseEffect> {
        let current = self.observation(cycle)?;
        let previous = cycle.checked_sub(1).and_then(|c| self.observation(c));
        Some(cause_effect(self, &current, previous.as_ref()))
    }

    /// PLC variables with all images, I/O source and quality. Filtered and paged.
    pub fn variables(&self, filter: Option<&str>, offset: usize, limit: usize) -> (usize, Vec<VariableRow>) {
        let snapshot = self.plc.snapshot();
        let needle = filter.map(str::to_ascii_lowercase);
        let matches = |name: &str| needle.as_ref().is_none_or(|n| name.to_ascii_lowercase().contains(n));
        let mut rows = Vec::new();
        for tag in self.tags() {
            if !matches(&tag.name) {
                continue;
            }
            let get = |map: Option<&std::collections::BTreeMap<_, plc_types::PlcValue>>| {
                map.and_then(|m| m.get(&tag.symbol)).map(|v| SignalValue::from_plc(*v))
            };
            let (scan_value, staged, committed) = match tag.role {
                TagRole::Input => (
                    get(snapshot.as_ref().map(|s| s.input_image())),
                    get(snapshot.as_ref().map(|s| s.staged_inputs())),
                    None,
                ),
                _ => (None, None, get(snapshot.as_ref().map(|s| s.committed()))),
            };
            let (source, quality) = match (tag.source, tag.sink) {
                (Some(InputSource::Channel(c)), _) | (None, Some(c)) => (
                    Some(self.channel_address(c)),
                    self.station().reading(c).map(|r| r.quality).or(Some(Quality::Good)),
                ),
                (Some(InputSource::Modbus { scan, .. } | InputSource::ModbusStatus { scan }), _) => {
                    let status = self.network().scan_status(scan);
                    let view = self.network().scan_views().into_iter().nth(scan);
                    (
                        view.map(|v| format!("Modbus TCP {} unit {}", v.server, v.unit_id)),
                        Some(if status == Some(network_core::ScanStatus::Ok) { Quality::Good } else { Quality::ModuleFault }),
                    )
                }
                _ => (None, None),
            };
            rows.push(VariableRow {
                signal: tag.signal,
                name: tag.name.clone(),
                kind: match tag.role {
                    TagRole::Input => SignalKind::PlcInput,
                    TagRole::Output => SignalKind::PlcOutput,
                    TagRole::Local => SignalKind::PlcLocal,
                },
                scan_value,
                committed,
                staged,
                source,
                quality,
                forced: self.forces().get(&tag.signal).copied(),
            });
        }
        for block in self.blocks() {
            let state = snapshot.as_ref().and_then(|s| s.function_blocks().get(&block.instance).copied());
            for (name, signal) in &block.outputs {
                let label = format!("{}.{}", block.name, name);
                if !matches(&label) {
                    continue;
                }
                rows.push(VariableRow {
                    signal: *signal,
                    name: label,
                    kind: SignalKind::BlockOutput,
                    scan_value: None,
                    committed: state.and_then(|s| s.outputs().into_iter().find(|(n, _)| n == name)).map(|(_, v)| SignalValue::from_plc(v)),
                    staged: None,
                    source: None,
                    quality: None,
                    forced: None,
                });
            }
        }
        let total = rows.len();
        (total, rows.into_iter().skip(offset).take(limit).collect())
    }

    pub fn channel_address(&self, channel: lab_model::hardware::ChannelRef) -> String {
        let config = self.station().configuration();
        let Some(module) = config.module_at(channel.slot_ref()) else { return channel.to_string() };
        hardware_model::address::format(
            self.profile().addressing,
            module.module,
            module.start_address,
            channel,
            self.profile().product_name(module.module),
        )
    }

    /// Current values of selected signals (watch list), from authoritative state.
    pub fn watch(&self, signals: &[SignalId]) -> Vec<(SignalId, Option<SignalValue>)> {
        let latest = self.last.as_ref();
        signals
            .iter()
            .map(|s| {
                let value = latest.and_then(|o| {
                    value_in(&o.staged_next, *s)
                        .or_else(|| value_in(&o.committed, *s))
                        .or_else(|| value_in(&o.process_after, *s))
                });
                (*s, value.or_else(|| self.current_value(*s)))
            })
            .collect()
    }

    fn current_value(&self, signal: SignalId) -> Option<SignalValue> {
        if let Some(v) = self.channel_values.get(&signal) {
            return Some(*v);
        }
        if let Some((index, name, _)) = self.physical.iter().find(|(_, _, s)| *s == signal) {
            return self.scene.quantity(*index, name).map(|q| match q {
                process_simulation::Quantity::Bool(b) => SignalValue::Bool(b),
                process_simulation::Quantity::Integer(i) => SignalValue::Count(i),
            });
        }
        let snapshot = self.plc.snapshot()?;
        let tag = self.tags().iter().find(|t| t.signal == signal)?;
        let map = if tag.role == TagRole::Input { snapshot.staged_inputs() } else { snapshot.committed() };
        map.get(&tag.symbol).map(|v| SignalValue::from_plc(*v))
    }

    /// Change-point series for up to 16 signals in a time window, decimated to `max_points`.
    pub fn trend(&self, signals: &[SignalId], from_us: u64, to_us: u64, max_points: usize) -> Vec<TrendSeries> {
        let trace = self.trace();
        let stats = trace.stats();
        let (Some(oldest), Some(latest)) = (stats.oldest, stats.latest) else { return Vec::new() };
        let from = trace.find_first(|h| h.end_us >= from_us).unwrap_or(latest);
        let to = trace
            .find_first(|h| h.start_us > to_us)
            .map(|s| RecordSeq(s.0.saturating_sub(1)))
            .unwrap_or(latest);
        if to < from || to < oldest {
            return Vec::new();
        }
        let max_points = max_points.clamp(16, 5_000);
        signals
            .iter()
            .take(16)
            .filter_map(|signal| {
                let kind = self.signals().get(*signal)?.kind;
                let (layer, index, at_end) = match kind {
                    SignalKind::PlcInput => (TrendLayer::Inputs, LAYER_INPUTS, false),
                    SignalKind::PlcOutput | SignalKind::PlcLocal | SignalKind::BlockOutput => (TrendLayer::Committed, LAYER_COMMITTED, false),
                    SignalKind::Physical | SignalKind::Channel => (TrendLayer::Process, LAYER_PROCESS, true),
                };
                let series = trace.series(index, &[signal.0], from, to).pop().unwrap_or_default();
                let mut points: Vec<(u64, SignalValue)> = series
                    .into_iter()
                    .filter_map(|(seq, value)| {
                        let h = trace.header(seq)?;
                        Some((if at_end { h.end_us } else { h.start_us }, value))
                    })
                    .collect();
                let decimated = points.len() > max_points;
                if decimated {
                    points = decimate(&points, from_us, to_us, max_points);
                }
                Some(TrendSeries { signal: *signal, layer, points, decimated })
            })
            .collect()
    }

    /// Cycle headers in a window, newest last, bounded.
    pub fn history(&self, before_cycle: Option<u64>, limit: usize) -> Vec<HistoryRow> {
        let limit = limit.min(500);
        let mut rows: Vec<HistoryRow> = self
            .trace()
            .headers()
            .rev()
            .filter(|(_, h)| before_cycle.is_none_or(|c| h.cycle < c))
            .take(limit)
            .map(|(_, h)| HistoryRow { header: *h })
            .collect();
        rows.reverse();
        rows
    }
}

/// Keep, per time bucket, the first, minimum, maximum and last point (in time order).
fn decimate(points: &[(u64, SignalValue)], from: u64, to: u64, max_points: usize) -> Vec<(u64, SignalValue)> {
    let buckets = (max_points / 4).max(1) as u64;
    let span = to.saturating_sub(from).max(1);
    let mut out: Vec<(u64, SignalValue)> = Vec::with_capacity(max_points);
    let mut start = 0;
    while start < points.len() {
        let bucket = (points[start].0.saturating_sub(from)) * buckets / span;
        let mut end = start;
        while end < points.len() && (points[end].0.saturating_sub(from)) * buckets / span == bucket {
            end += 1;
        }
        let slice = &points[start..end];
        let mut keep = vec![0, slice.len() - 1];
        let min = (0..slice.len()).min_by(|a, b| slice[*a].1.as_f64().total_cmp(&slice[*b].1.as_f64())).unwrap_or(0);
        let max = (0..slice.len()).max_by(|a, b| slice[*a].1.as_f64().total_cmp(&slice[*b].1.as_f64())).unwrap_or(0);
        keep.extend([min, max]);
        keep.sort_unstable();
        keep.dedup();
        out.extend(keep.into_iter().map(|i| slice[i]));
        start = end;
    }
    out
}
