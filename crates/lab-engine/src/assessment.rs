//! Headless, isolated exercise assessment. Every scenario runs in a fresh session with
//! scripted actions at fixed world times; no pacing, wall clock or user interface is
//! involved. Evidence is projected from causal cycle observations.

use crate::{
    error::{BindingIssue, EngineError},
    plc::TagRole,
    session::{CycleObservation, CycleOutcome, LabEventKind, LabSession, SessionConfig},
    signals::{SignalId, SignalValue},
};
use grading_core::{
    CycleFacts, EvidenceEvent, EvidenceValue, GradeReport, GradingEvidence, PlcFaultEvidence,
};
use lab_model::{
    exercise::{
        EvidenceSignal, EvidenceSource, ExerciseDefinition, PlcModeRequest, RequiredTag, Scenario,
    },
    fault::FaultInjection,
    hardware::StationConfig,
    profile::PlatformProfile,
    scene::{SceneDefinition, SignalDirection, TagType},
};
use plc_types::{PlcType, diagnostics::Diagnostic, symbols::Scope};
use simulation_core::WorldTime;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssessmentError {
    Compile(Vec<Diagnostic>),
    Interface(Vec<String>),
    Binding(Vec<BindingIssue>),
    Engine(String),
}
impl From<EngineError> for AssessmentError {
    fn from(e: EngineError) -> Self {
        match e {
            EngineError::Compile(d) => Self::Compile(d),
            EngineError::Binding(b) => Self::Binding(b),
            other => Self::Engine(other.to_string()),
        }
    }
}

pub struct AssessmentSetup<'a> {
    pub scene: &'a SceneDefinition,
    pub station: &'a StationConfig,
    pub profile: &'a PlatformProfile,
    pub program: &'a str,
    /// Physical faults present in the installation during the run.
    pub faults: &'a [FaultInjection],
    pub config: SessionConfig,
}

fn tag_type(t: TagType) -> PlcType {
    match t {
        TagType::Bool => PlcType::Bool,
        TagType::Int => PlcType::Int,
        TagType::Real => PlcType::Real,
        TagType::Time => PlcType::Time,
    }
}

/// Check the program declares every required tag with the right type and direction.
pub fn interface_problems(
    source: &str,
    required: &[RequiredTag],
) -> Result<Vec<String>, Vec<Diagnostic>> {
    let compiled = crate::plc::compile(source)?;
    let program = compiled.program();
    let scope = Scope::Pou(program.entry());
    let mut problems = Vec::new();
    for r in required {
        match program.symbols().resolve(scope, r.tag.as_str()) {
            Err(_) => problems.push(format!(
                "Declare {} : {:?} as {}.",
                r.tag,
                r.tag_type,
                direction(r.direction)
            )),
            Ok(symbol) => {
                let role_ok = match r.direction {
                    SignalDirection::Input => {
                        symbol.category() == plc_types::symbols::VariableCategory::Input
                    }
                    SignalDirection::Output => {
                        symbol.category() == plc_types::symbols::VariableCategory::Output
                    }
                };
                if !role_ok || symbol.plc_type() != tag_type(r.tag_type) {
                    problems.push(format!(
                        "'{}' must be {} {:?}.",
                        symbol.name(),
                        direction(r.direction),
                        r.tag_type
                    ));
                }
            }
        }
    }
    Ok(problems)
}
fn direction(d: SignalDirection) -> &'static str {
    match d {
        SignalDirection::Input => "VAR_INPUT",
        SignalDirection::Output => "VAR_OUTPUT",
    }
}

enum Binding {
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

fn facts(observation: &CycleObservation, bindings: &[(String, Binding)]) -> CycleFacts {
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

/// Run one scenario in a fresh, isolated session and return causal evidence.
pub fn run_scenario(
    setup: &AssessmentSetup<'_>,
    scenario: &Scenario,
    evidence: &[EvidenceSignal],
) -> Result<GradingEvidence, AssessmentError> {
    let mut session = LabSession::new(
        setup.scene.clone(),
        setup.station.clone(),
        setup.profile.clone(),
        setup.config,
    )?;
    for fault in setup.faults {
        session.inject_fault(fault)?;
    }
    session.download(setup.program, 1, None)?;
    session.set_plc_switch(PlcModeRequest::Run)?;
    for action in &scenario.actions {
        session.schedule(
            WorldTime::from_micros(u64::from(action.at_ms) * 1_000),
            action.action.clone(),
        )?;
    }
    let bindings: Vec<(String, Binding)> = evidence
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
        .collect();
    let duration = u64::from(scenario.duration_ms) * 1_000;
    let mut result = GradingEvidence {
        scenario: scenario.id.to_string(),
        duration_us: duration,
        ..Default::default()
    };
    // Bounded: a faulted scan does not advance time, so allow a few extra steps.
    let max_steps = duration / setup.config.cycle_us.max(1) + 16;
    let mut steps = 0;
    while session.now().as_micros() < duration && steps < max_steps {
        let observation = session.step_cycle()?;
        if let (Some(d), None) = (&observation.fault, &result.plc_fault) {
            result.plc_fault = Some(PlcFaultEvidence {
                cycle: observation.header.cycle,
                code: d.code().to_owned(),
                message: d.message().to_owned(),
            });
        }
        result.cycles.push(facts(&observation, &bindings));
        steps += 1;
    }
    for (_, event) in session.events().iter() {
        if let LabEventKind::Process { component, kind } = &event.kind
            && let Some(kind) = map_event(*kind)
        {
            result.events.push(EvidenceEvent {
                time_us: event.time_us,
                component: component.clone(),
                kind,
            });
        }
    }
    Ok(result)
}

fn map_event(
    kind: process_simulation::ProcessEventKind,
) -> Option<lab_model::exercise::ProcessEventKind> {
    use lab_model::exercise::ProcessEventKind as E;
    use process_simulation::ProcessEventKind as P;
    match kind {
        P::OverflowStarted => Some(E::Overflow),
        P::DryRunStarted => Some(E::DryRun),
        P::HighHighOn => Some(E::HighHigh),
        P::LowLowOn => Some(E::LowLow),
        P::MotorOverloadTripped => Some(E::MotorOverload),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssessmentOutcome {
    Graded(GradeReport),
    CompileFailed(Vec<Diagnostic>),
    InterfaceMismatch(Vec<String>),
    BindingFailed(Vec<BindingIssue>),
}

/// Grade a practice exercise or the verification part of a troubleshooting exercise.
pub fn assess(
    exercise: &ExerciseDefinition,
    scene: &SceneDefinition,
    station: &StationConfig,
    profile: &PlatformProfile,
    program: &str,
    faults: &[FaultInjection],
    criteria_filter: Option<&[lab_model::Identifier]>,
) -> Result<AssessmentOutcome, AssessmentError> {
    match interface_problems(program, &exercise.required_interface) {
        Err(d) => return Ok(AssessmentOutcome::CompileFailed(d)),
        Ok(problems) if !problems.is_empty() => {
            return Ok(AssessmentOutcome::InterfaceMismatch(problems));
        }
        Ok(_) => {}
    }
    let criteria: Vec<_> = exercise
        .criteria
        .iter()
        .filter(|c| criteria_filter.is_none_or(|f| f.contains(&c.id)))
        .cloned()
        .collect();
    let setup = AssessmentSetup {
        scene,
        station,
        profile,
        program,
        faults,
        config: SessionConfig {
            seed: 7,
            ..SessionConfig::default()
        },
    };
    let mut evidence = BTreeMap::new();
    for scenario in exercise
        .scenarios
        .iter()
        .filter(|s| criteria.iter().any(|c| c.scenario == s.id))
    {
        match run_scenario(&setup, scenario, &exercise.evidence) {
            Ok(e) => {
                evidence.insert(scenario.id.to_string(), e);
            }
            Err(AssessmentError::Compile(d)) => return Ok(AssessmentOutcome::CompileFailed(d)),
            Err(AssessmentError::Binding(b)) => return Ok(AssessmentOutcome::BindingFailed(b)),
            Err(other) => return Err(other),
        }
    }
    Ok(AssessmentOutcome::Graded(grading_core::grade(
        &criteria, &evidence,
    )))
}
