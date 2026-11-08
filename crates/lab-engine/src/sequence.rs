//! Sequence supervision (ADR-0043): the session's sequence monitor, bound to the current
//! program through exercise evidence. The monitor observes each cycle after the process
//! advanced and before alarms are evaluated; it never writes PLC memory or the plant.

use crate::{
    error::EngineError,
    evidence::{Binding, bind, facts},
    session::{CycleObservation, LabEvent, LabEventKind, LabSession},
};
use lab_model::{exercise::EvidenceSignal, sequence::SequenceDefinition};
use plc_runtime::StartKind;
use sequence_core::{RunStart, SequenceMonitor};

pub(crate) struct Supervision {
    pub monitor: SequenceMonitor,
    pub evidence: Vec<EvidenceSignal>,
    pub bindings: Vec<(String, Binding)>,
}

impl LabSession {
    /// Observe the program against a sequence definition from now on. The definition
    /// must be validated with its exercise (`lab_model::validate_exercise`).
    pub fn configure_sequence(
        &mut self,
        definition: SequenceDefinition,
        evidence: Vec<EvidenceSignal>,
    ) -> Result<(), EngineError> {
        let monitor = SequenceMonitor::new(definition).map_err(EngineError::Scene)?;
        let bindings = bind(self, &evidence);
        self.sequence = Some(Supervision {
            monitor,
            evidence,
            bindings,
        });
        self.configure_alarms()?;
        self.evaluate_alarms();
        Ok(())
    }

    /// The sequence monitor, when the session observes a sequence.
    pub fn sequence(&self) -> Option<&SequenceMonitor> {
        self.sequence.as_ref().map(|s| &s.monitor)
    }

    /// The definition and evidence to carry into a fresh world.
    pub(crate) fn sequence_setup(&self) -> Option<(SequenceDefinition, Vec<EvidenceSignal>)> {
        self.sequence
            .as_ref()
            .map(|s| (s.monitor.definition().clone(), s.evidence.clone()))
    }

    /// A new program has new tags: resolve the evidence again.
    pub(crate) fn rebind_sequence(&mut self) {
        if let Some(evidence) = self.sequence.as_ref().map(|s| s.evidence.clone()) {
            let bindings = bind(self, &evidence);
            if let Some(s) = self.sequence.as_mut() {
                s.bindings = bindings;
            }
        }
    }

    /// Observe one cycle; `started` is the start kind when this cycle executed the first
    /// scan after a PLC start.
    pub(crate) fn observe_sequence(
        &mut self,
        observation: &CycleObservation,
        started: Option<StartKind>,
    ) {
        let Some(supervision) = self.sequence.as_mut() else {
            return;
        };
        let start = started.map(|kind| match kind {
            StartKind::Warm => RunStart::Warm,
            StartKind::Cold => RunStart::Cold,
        });
        let facts = facts(observation, &supervision.bindings);
        for event in supervision.monitor.observe(&facts, start) {
            self.events.append(LabEvent {
                time_us: event.time_us,
                cycle: self.cycle,
                kind: LabEventKind::Sequence { event },
            });
        }
    }
}
