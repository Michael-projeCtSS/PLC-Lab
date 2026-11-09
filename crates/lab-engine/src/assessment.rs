//! Headless, isolated exercise assessment. Every scenario runs in a fresh session with
//! scripted actions at fixed world times; no pacing, wall clock or user interface is
//! involved. Evidence is projected from causal cycle observations.

use crate::{
    error::{BindingIssue, EngineError},
    session::{LabEventKind, LabSession, SessionConfig},
};
use grading_core::{
    EvidenceEvent, GradeReport, GradingEvidence, PlcFaultEvidence, SequenceEvidence, SequenceFault,
};
use lab_model::{
    exercise::{
        EvidenceSignal, ExerciseDefinition, PlcModeRequest, RequiredPou, RequiredPouKind,
        RequiredTag, Scenario,
    },
    fault::FaultInjection,
    hardware::StationConfig,
    profile::PlatformProfile,
    scene::{SceneDefinition, SignalDirection, TagType},
    sequence::SequenceDefinition,
};
use plc_types::{PlcType, diagnostics::Diagnostic, symbols::Scope};
use simulation_core::WorldTime;
use st_frontend::CompiledProgram;
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
    /// The project library documents compiled with `program` (ADR-0045).
    pub library: &'a [String],
    /// Physical faults present in the installation during the run.
    pub faults: &'a [FaultInjection],
    pub config: SessionConfig,
    /// The sequence the program implements, observed during every run (ADR-0043).
    pub sequence: Option<&'a SequenceDefinition>,
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
pub fn interface_problems(compiled: &CompiledProgram, required: &[RequiredTag]) -> Vec<String> {
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
    problems
}
/// Check the program defines every user function and function block the exercise asks
/// for (ADR-0044).
pub fn pou_problems(compiled: &CompiledProgram, required: &[RequiredPou]) -> Vec<String> {
    let mut problems = Vec::new();
    for r in required {
        let (keyword, kind) = match r.kind {
            RequiredPouKind::Function => ("FUNCTION", "FUNCTION"),
            RequiredPouKind::FunctionBlock => ("FUNCTION_BLOCK", "FUNCTION_BLOCK"),
        };
        match compiled
            .pous()
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(r.name.as_str()))
        {
            None => problems.push(format!("Write {keyword} {}.", r.name)),
            Some(p) if p.kind != kind => {
                problems.push(format!("'{}' must be a {keyword}.", p.name))
            }
            Some(_) => {}
        }
    }
    problems
}

fn direction(d: SignalDirection) -> &'static str {
    match d {
        SignalDirection::Input => "VAR_INPUT",
        SignalDirection::Output => "VAR_OUTPUT",
    }
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
    session.download_documents(setup.program, setup.library, 1, None)?;
    session.set_plc_switch(PlcModeRequest::Run)?;
    for action in &scenario.actions {
        session.schedule(
            WorldTime::from_micros(u64::from(action.at_ms) * 1_000),
            action.action.clone(),
        )?;
    }
    if let Some(sequence) = setup.sequence {
        session.configure_sequence(sequence.clone(), evidence.to_vec())?;
    }
    let bindings = crate::evidence::bind(&session, evidence);
    let duration = u64::from(scenario.duration_ms) * 1_000;
    let mut result = GradingEvidence {
        scenario: scenario.id.to_string(),
        duration_us: duration,
        ..Default::default()
    };
    // Bounded: a faulted scan does not advance time, so allow a few extra steps.
    let max_steps = duration / setup.config.cycle_us.max(1) + 16;
    let mut steps = 0;
    let mut reached: Vec<(String, u64)> = Vec::new();
    while session.now().as_micros() < duration && steps < max_steps {
        let observation = session.step_cycle()?;
        if let (Some(d), None) = (&observation.fault, &result.plc_fault) {
            result.plc_fault = Some(PlcFaultEvidence {
                cycle: observation.header.cycle,
                code: d.code().to_owned(),
                message: d.message().to_owned(),
            });
        }
        result
            .cycles
            .push(crate::evidence::facts(&observation, &bindings));
        if let Some(monitor) = session.sequence()
            && let Some(step) = &monitor.status().active
            && !reached.iter().any(|(id, _)| id == step.as_str())
        {
            reached.push((step.to_string(), observation.header.end_us));
        }
        steps += 1;
    }
    if session.sequence().is_some() {
        let mut faults = Vec::new();
        for (_, event) in session.events().iter() {
            if let LabEventKind::Sequence { event } = &event.kind
                && event.kind.is_fault()
            {
                faults.push(SequenceFault {
                    time_us: event.time_us,
                    code: event.kind.code().to_owned(),
                    step: event.step.as_ref().map(ToString::to_string),
                });
            }
        }
        result.sequence = Some(SequenceEvidence { faults, reached });
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
    assess_documents(
        exercise,
        scene,
        station,
        profile,
        program,
        &[],
        faults,
        criteria_filter,
    )
}

/// [`assess`] for a main program with project library documents (ADR-0045).
#[allow(clippy::too_many_arguments)]
pub fn assess_documents(
    exercise: &ExerciseDefinition,
    scene: &SceneDefinition,
    station: &StationConfig,
    profile: &PlatformProfile,
    program: &str,
    library: &[String],
    faults: &[FaultInjection],
    criteria_filter: Option<&[lab_model::Identifier]>,
) -> Result<AssessmentOutcome, AssessmentError> {
    let compiled = match crate::plc::compile_documents(program, library) {
        Ok(compiled) => compiled,
        Err(d) => return Ok(AssessmentOutcome::CompileFailed(d)),
    };
    let mut problems = interface_problems(&compiled, &exercise.required_interface);
    problems.extend(pou_problems(&compiled, &exercise.required_pous));
    if !problems.is_empty() {
        return Ok(AssessmentOutcome::InterfaceMismatch(problems));
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
        library,
        faults,
        config: SessionConfig {
            seed: 7,
            ..SessionConfig::default()
        },
        sequence: exercise.sequence.as_ref(),
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
