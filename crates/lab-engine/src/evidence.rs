//! Exercise evidence: named signals bound to the session's PLC tags and process
//! quantities, projected from one cycle observation into the facts grading and the
//! sequence monitor read (the same facts, so both judge the same snapshot).

use crate::{
    plc::TagRole,
    session::{CycleObservation, CycleOutcome, LabSession},
    signals::{SignalId, SignalValue},
};
use grading_core::{CycleFacts, EvidenceValue};
use lab_model::exercise::{EvidenceSignal, EvidenceSource};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Binding {
    Input(SignalId),
    Output(SignalId),
    Process(SignalId),
    Missing,
}

fn evidence_value(v: SignalValue) -> EvidenceValue {
    match v {
        SignalValue::Bool(b) => EvidenceValue::Bool(b),
        SignalValue::Int(i) => EvidenceValue::Number(f64::from(i)),
        SignalValue::Real(r) => EvidenceValue::Number(f64::from(r)),
        SignalValue::Time(t) => EvidenceValue::Number(t as f64),
        SignalValue::Count(c) => EvidenceValue::Number(c as f64),
    }
}

fn lookup(layer: &[(SignalId, SignalValue)], signal: SignalId) -> Option<EvidenceValue> {
    layer
        .binary_search_by_key(&signal, |(s, _)| *s)
        .ok()
        .map(|i| evidence_value(layer[i].1))
}

pub(crate) fn facts(observation: &CycleObservation, bindings: &[(String, Binding)]) -> CycleFacts {
    let mut f = CycleFacts {
        cycle: observation.header.cycle,
        start_us: observation.header.start_us,
        end_us: observation.header.end_us,
        outcome: Some(match observation.header.outcome {
            CycleOutcome::Scanned => grading_core::CycleOutcome::Scanned,
            CycleOutcome::Faulted => grading_core::CycleOutcome::Faulted,
            CycleOutcome::NotScanned(_) => grading_core::CycleOutcome::NotScanned,
        }),
        ..Default::default()
    };
    for (name, binding) in bindings {
        match binding {
            Binding::Input(s) => {
                if let Some(v) = lookup(&observation.scan_inputs, *s) {
                    f.inputs.insert(name.clone(), v);
                }
                if let Some(v) = lookup(&observation.staged_next, *s) {
                    f.next.insert(name.clone(), v);
                }
            }
            Binding::Output(s) => {
                if let Some(v) = lookup(&observation.committed, *s) {
                    f.outputs.insert(name.clone(), v);
                }
            }
            Binding::Process(s) => {
                if let Some(v) = lookup(&observation.process_after, *s) {
                    f.process.insert(name.clone(), v);
                }
            }
            Binding::Missing => {}
        }
    }
    f
}

/// Resolve evidence names against the session's current tags and process signals.
pub(crate) fn bind(session: &LabSession, evidence: &[EvidenceSignal]) -> Vec<(String, Binding)> {
    evidence
        .iter()
        .map(|e| {
            let binding = match &e.source {
                EvidenceSource::PlcTag { tag } => session
                    .tags()
                    .iter()
                    .find(|t| t.name.eq_ignore_ascii_case(tag.as_str()))
                    .map_or(Binding::Missing, |t| {
                        if t.role == TagRole::Input {
                            Binding::Input(t.signal)
                        } else {
                            Binding::Output(t.signal)
                        }
                    }),
                EvidenceSource::Component {
                    component,
                    quantity,
                } => session
                    .signals()
                    .by_path(&format!("process.{component}.{quantity}"))
                    .map_or(Binding::Missing, Binding::Process),
            };
            (e.name.to_string(), binding)
        })
        .collect()
}
