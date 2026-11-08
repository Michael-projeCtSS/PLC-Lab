//! Sequence projections (ADR-0043): the documented definition for the exercise view and
//! the monitor's live state for the frame. The view only displays these; it evaluates
//! nothing.

use app_contract::*;
use grading_core::EvidenceValue;
use lab_engine::{
    LabSession,
    sequence_core::{ConditionStatus, RunStart, SequenceEvent, SequenceState, StepState},
};
use lab_model::{
    exercise::{Comparison, EvidenceLayer, EvidenceSignal, EvidenceSource, Literal},
    sequence::{InterlockReaction, SequenceDefinition, StepRole},
};

/// The newest monitor events a frame carries.
pub const FRAME_HISTORY: usize = 40;

pub fn definition(s: &SequenceDefinition) -> SequenceDefinitionDto {
    SequenceDefinitionDto {
        id: s.id.to_string(),
        name: s.name.clone(),
        steps: s
            .steps
            .iter()
            .map(|step| SequenceStepDto {
                id: step.id.to_string(),
                number: step.number,
                name: step.name.clone(),
                role: match step.role {
                    StepRole::Initial => "initial",
                    StepRole::Normal => "normal",
                    StepRole::Final => "final",
                }
                .into(),
                timeout_ms: step.timeout_ms,
            })
            .collect(),
        transitions: s
            .transitions
            .iter()
            .map(|t| SequenceTransitionDto {
                id: t.id.to_string(),
                from: t.from.to_string(),
                to: t.to.to_string(),
                name: t.name.clone(),
                permissives: t
                    .permissives
                    .iter()
                    .map(|p| SequencePermissiveDto {
                        id: p.id.to_string(),
                        name: p.name.clone(),
                    })
                    .collect(),
            })
            .collect(),
        interlocks: s
            .interlocks
            .iter()
            .map(|i| SequenceInterlockDto {
                id: i.id.to_string(),
                name: i.name.clone(),
                reaction: match i.reaction {
                    InterlockReaction::BlockStart => "blockStart",
                    InterlockReaction::HoldProgress => "holdProgress",
                    InterlockReaction::AbortSequence => "abortSequence",
                }
                .into(),
                steps: i.steps.iter().map(ToString::to_string).collect(),
                reaction_ms: i.reaction_ms,
            })
            .collect(),
    }
}

fn layer(l: EvidenceLayer) -> &'static str {
    match l {
        EvidenceLayer::Inputs => "inputs",
        EvidenceLayer::Outputs => "outputs",
        EvidenceLayer::Process => "process",
        EvidenceLayer::Next => "next",
    }
}

fn op(c: Comparison) -> &'static str {
    match c {
        Comparison::Eq => "eq",
        Comparison::Ne => "ne",
        Comparison::Gt => "gt",
        Comparison::Ge => "ge",
        Comparison::Lt => "lt",
        Comparison::Le => "le",
    }
}

fn value(v: EvidenceValue) -> SequenceValueDto {
    match v {
        EvidenceValue::Bool(value) => SequenceValueDto::Bool { value },
        EvidenceValue::Number(value) => SequenceValueDto::Number { value },
    }
}

fn literal(l: Literal) -> SequenceValueDto {
    match l {
        Literal::Bool(value) => SequenceValueDto::Bool { value },
        Literal::Number(value) => SequenceValueDto::Number { value },
    }
}

/// What an evidence name is bound to: the PLC tag or the component quantity.
fn source(evidence: &[EvidenceSignal], name: &str) -> String {
    match evidence
        .iter()
        .find(|e| e.name.as_str() == name)
        .map(|e| &e.source)
    {
        Some(EvidenceSource::PlcTag { tag }) => tag.to_string(),
        Some(EvidenceSource::Component {
            component,
            quantity,
        }) => format!("{component}.{quantity}"),
        None => name.to_owned(),
    }
}

fn condition(c: &ConditionStatus, evidence: &[EvidenceSignal]) -> SequenceConditionDto {
    SequenceConditionDto {
        satisfied: c.satisfied,
        rows: c
            .rows
            .iter()
            .map(|r| SequenceConditionRowDto {
                signal: r.signal.to_string(),
                source: source(evidence, r.signal.as_str()),
                layer: layer(r.layer).into(),
                op: op(r.op).into(),
                expected: literal(r.expected),
                actual: r.actual.map(value),
                satisfied: r.satisfied,
                negated: r.negated,
            })
            .collect(),
    }
}

fn event(e: &SequenceEvent, evidence: &[EvidenceSignal]) -> SequenceEventDto {
    let id = |x: &Option<lab_model::Identifier>| x.as_ref().map(ToString::to_string);
    SequenceEventDto {
        seq: ExactU64(e.seq),
        time_us: ExactU64(e.time_us),
        cycle: ExactU64(e.cycle),
        code: e.kind.code().into(),
        fault: e.kind.is_fault(),
        step: id(&e.step),
        from: id(&e.from),
        transition: id(&e.transition),
        interlock: id(&e.interlock),
        permissives: e.permissives.iter().map(ToString::to_string).collect(),
        interlocks: e.interlocks.iter().map(ToString::to_string).collect(),
        step_value: e.step_value,
        start: match e.kind {
            lab_engine::sequence_core::SequenceEventKind::RunStarted { start } => Some(
                match start {
                    RunStart::Warm => "WARM",
                    RunStart::Cold => "COLD",
                    RunStart::Attached => "ATTACHED",
                }
                .into(),
            ),
            _ => None,
        },
        values: e
            .values
            .iter()
            .map(|v| SequenceObservedDto {
                signal: v.signal.to_string(),
                source: source(evidence, v.signal.as_str()),
                layer: layer(v.layer).into(),
                value: v.value.map(value),
            })
            .collect(),
    }
}

/// The monitor's current state for the frame, when the session observes a sequence.
pub fn status(session: &LabSession) -> Option<SequenceStatusDto> {
    let monitor = session.sequence()?;
    let evidence = session.sequence_evidence();
    let st = monitor.status();
    let def = monitor.definition();
    let history: Vec<SequenceEventDto> = monitor
        .history()
        .rev()
        .take(FRAME_HISTORY)
        .map(|e| event(e, evidence))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    Some(SequenceStatusDto {
        sequence_id: def.id.to_string(),
        state: match st.state {
            SequenceState::Unbound => "UNBOUND",
            SequenceState::Stopped => "STOPPED",
            SequenceState::Idle => "IDLE",
            SequenceState::Running => "RUNNING",
            SequenceState::Held => "HELD",
            SequenceState::Completed => "COMPLETED",
            SequenceState::Faulted => "FAULTED",
        }
        .into(),
        active_step: st.active.as_ref().map(ToString::to_string),
        previous_step: st.previous.as_ref().map(ToString::to_string),
        step_value: st.step_value,
        elapsed_us: ExactU64(st.elapsed_us),
        timeout_us: st.timeout_us.map(ExactU64),
        fault: st.fault.map(|f| f.code().into()),
        fault_step: st.fault_step.as_ref().map(ToString::to_string),
        steps: def
            .steps
            .iter()
            .zip(&st.steps)
            .map(|(step, state)| SequenceStepStateDto {
                id: step.id.to_string(),
                state: match state {
                    StepState::Inactive => "INACTIVE",
                    StepState::Active => "ACTIVE",
                    StepState::Held => "HELD",
                    StepState::Completed => "COMPLETED",
                    StepState::Faulted => "FAULTED",
                }
                .into(),
            })
            .collect(),
        transitions: st
            .transitions
            .iter()
            .map(|t| SequenceTransitionStateDto {
                id: t.id.to_string(),
                from_active: t.from_active,
                condition: condition(&t.condition, evidence),
                permissives: t
                    .permissives
                    .iter()
                    .map(|p| SequencePermissiveStateDto {
                        id: p.id.to_string(),
                        condition: condition(&p.condition, evidence),
                    })
                    .collect(),
                blocked_by: t.blocked_by.iter().map(ToString::to_string).collect(),
            })
            .collect(),
        interlocks: st
            .interlocks
            .iter()
            .map(|i| SequenceInterlockStateDto {
                id: i.id.to_string(),
                applies: i.applies,
                active: i.active,
                condition: condition(&i.condition, evidence),
            })
            .collect(),
        history,
        total_events: ExactU64(monitor.total_events()),
    })
}
