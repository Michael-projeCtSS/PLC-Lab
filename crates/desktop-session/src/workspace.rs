//! The open workspace: one project bound to one live lab session, its simulation control
//! state, and (for troubleshooting exercises) the student's investigation record.
//!
//! Simulation control (Run/Pause/Step/Stop) is separate from the PLC operating mode
//! (RUN/STOP/FAULT). Stopping the simulation restores the plant's initial state; the PLC
//! program, hardware configuration and mode switch are kept (see ADR-0018).

use crate::{
    convert,
    error::{AppError, AppResult},
};
use app_contract::*;
use hardware_model::Measurement;
use lab_engine::{
    CycleOutcome, EngineError, LabSession, OperatorAction, SessionConfig, SignalId, TrendLayer,
};
use lab_model::{
    exercise::{ExerciseDefinition, PlcModeRequest},
    fault::FaultInjection,
    hardware::{ChannelRef, StationConfig},
    network::Ipv4,
    profile::PlatformProfile,
    scene::{Coupling, SceneDefinition},
};
use network_core::PingResult;
use process_simulation::Quantity;
use project_model::{ExerciseState, Project, ProjectKind};
use std::collections::BTreeSet;
use trace_core::EventSeq;

const MAX_WATCH: usize = 64;
const MAX_EVIDENCE: usize = 128;
/// Modbus TCP messages shown in the network view (newest kept).
const MAX_TRAFFIC: usize = 60;
/// Quantities that would reveal a hidden fault directly; the student must infer them.
const HIDDEN_QUANTITIES: [&str; 1] = ["stuck"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimState {
    Stopped,
    Paused,
    Running { run: u64 },
}

/// Exclusive right of one host pacing loop to advance one simulation run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunLease {
    pub(crate) workspace: u64,
    pub(crate) run: u64,
}
impl RunLease {
    pub fn wake(&self, cycle: u64) -> ExecutionWakeDto {
        ExecutionWakeDto {
            contract_version: CONTRACT_VERSION,
            workspace_id: ExactU64(self.workspace),
            run_id: ExactU64(self.run),
            cycle: ExactU64(cycle),
        }
    }
}

/// A run the host must pace: wait `period_us` of wall time before each tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunStart {
    pub lease: RunLease,
    pub period_us: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TickOutcome {
    /// The lease no longer matches the active run; the pacing loop must exit.
    Stale,
    /// One cycle was executed. The loop continues only while `running`.
    Advanced {
        running: bool,
        next_period_us: u64,
        wake: ExecutionWakeDto,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Investigation {
    pub diagnoses: Vec<String>,
    pub evidence: Vec<String>,
    pub repairs: Vec<String>,
}

pub struct Workspace {
    pub id: u64,
    pub project: Project,
    pub scene: SceneDefinition,
    pub profile: PlatformProfile,
    pub exercise: Option<ExerciseDefinition>,
    pub session: LabSession,
    pub sim: SimState,
    pub speed_percent: u32,
    pub latest_revision: u64,
    pub investigation: Investigation,
    pub signals_revision: u32,
    pub failure: Option<String>,
}

fn session_config(project: &Project) -> SessionConfig {
    SessionConfig {
        cycle_us: u64::from(project.simulation.cycle_ms) * 1000,
        ..SessionConfig::default()
    }
}

impl Workspace {
    pub fn open(
        id: u64,
        project: Project,
        scene: SceneDefinition,
        profile: PlatformProfile,
        exercise: Option<ExerciseDefinition>,
    ) -> AppResult<Self> {
        let mut session = LabSession::new(
            scene.clone(),
            project.station.clone(),
            profile.clone(),
            session_config(&project),
        )?;
        let mut investigation = Investigation::default();
        if let Some(state) = &project.exercise_state {
            investigation = Investigation {
                diagnoses: state.diagnoses.clone(),
                evidence: state.evidence.clone(),
                repairs: state.repairs.clone(),
            };
        }
        if let Some(t) = exercise.as_ref().and_then(|e| e.troubleshooting.as_ref()) {
            for fault in &t.faults {
                session.inject_fault(fault)?;
            }
            for repair in &investigation.repairs {
                if let Some(option) = t.repairs.iter().find(|r| r.id.as_str() == repair) {
                    session.repair(&option.label, &option.clears)?;
                }
            }
            // The installation was running before the fault was reported.
            let main = project
                .main_program()
                .map(|p| p.source.clone())
                .unwrap_or_default();
            let starter = exercise
                .as_ref()
                .map(|e| e.starter_program.as_str())
                .unwrap_or_default();
            let running = match &t.loaded_program {
                Some(loaded) if main == starter => loaded.clone(),
                _ => main,
            };
            session.download(&running, 0, None)?;
            session.set_plc_switch(PlcModeRequest::Run)?;
        }
        Ok(Self {
            id,
            speed_percent: project.simulation.speed_percent,
            project,
            scene,
            profile,
            exercise,
            session,
            sim: SimState::Stopped,
            latest_revision: 0,
            investigation,
            signals_revision: 1,
            failure: None,
        })
    }

    pub fn is_fault_finding(&self) -> bool {
        self.exercise
            .as_ref()
            .is_some_and(|e| e.kind.is_fault_finding())
    }

    pub fn ensure_healthy(&self) -> AppResult<()> {
        match &self.failure {
            Some(m) => Err(AppError::WorkspaceFailed(m.clone())),
            None => Ok(()),
        }
    }

    /// Run an engine operation; an internal engine error fails the workspace closed.
    fn engine<T>(
        &mut self,
        op: impl FnOnce(&mut LabSession) -> Result<T, EngineError>,
    ) -> AppResult<T> {
        self.ensure_healthy()?;
        op(&mut self.session).map_err(|e| {
            if e.is_internal() {
                self.failure = Some(e.to_string());
                self.sim = SimState::Stopped;
                AppError::WorkspaceFailed(e.to_string())
            } else {
                AppError::Engine(e)
            }
        })
    }

    pub fn period_us(&self) -> u64 {
        self.session.config().cycle_us * 100 / u64::from(self.speed_percent.clamp(10, 1000))
    }

    // ---- program and hardware ------------------------------------------------------

    pub fn build_and_load(
        &mut self,
        source: &str,
        revision: u64,
        station: Option<StationConfig>,
    ) -> AppResult<BuildLoadResultDto> {
        self.ensure_healthy()?;
        if revision < self.latest_revision {
            return Ok(self.build_result(
                "SUPERSEDED",
                Vec::new(),
                Vec::new(),
                Some("A newer program version was already loaded.".into()),
            ));
        }
        self.latest_revision = revision;
        match self.session.download(source, revision, station.clone()) {
            Ok(report) => {
                self.signals_revision += 1;
                if let Some(main) = self.project.main_program_mut() {
                    main.source = source.to_owned();
                }
                if let Some(station) = station {
                    self.project.station = station;
                }
                let issues = report.warnings.iter().map(binding_issue).collect();
                Ok(self.build_result(
                    "LOADED",
                    Vec::new(),
                    issues,
                    Some(format!(
                        "Program {} loaded into the PLC.",
                        report.program_name
                    )),
                ))
            }
            Err(EngineError::Compile(d)) => Ok(self.build_result(
                "COMPILE_FAILED",
                d.iter().map(convert::diagnostic).collect(),
                Vec::new(),
                None,
            )),
            Err(EngineError::Binding(b)) => Ok(self.build_result(
                "BINDING_FAILED",
                Vec::new(),
                b.iter().map(binding_issue).collect(),
                None,
            )),
            Err(EngineError::Hardware(m)) => {
                Ok(self.build_result("HARDWARE_REJECTED", Vec::new(), Vec::new(), Some(m)))
            }
            Err(e) => self.engine(|_| Err(e)),
        }
    }

    fn build_result(
        &self,
        outcome: &str,
        diagnostics: Vec<DiagnosticDto>,
        binding_issues: Vec<BindingIssueDto>,
        message: Option<String>,
    ) -> BuildLoadResultDto {
        BuildLoadResultDto {
            outcome: outcome.into(),
            diagnostics,
            binding_issues,
            message,
            signals: convert::signals(&self.session),
            frame: self.frame(&[]),
        }
    }

    pub fn set_plc_mode(&mut self, mode: &str) -> AppResult<()> {
        let request = match mode {
            "RUN" => PlcModeRequest::Run,
            "STOP" => PlcModeRequest::Stop,
            other => {
                return Err(AppError::InvalidRequest(format!(
                    "Unknown PLC mode '{other}'."
                )));
            }
        };
        self.engine(|s| s.set_plc_switch(request)).map(|_| ())
    }

    pub fn reset_plc(&mut self) -> AppResult<()> {
        self.engine(LabSession::reset_plc).map(|_| ())
    }

    // ---- simulation control --------------------------------------------------------

    pub fn simulation(&mut self, command: &str, next_run: &mut u64) -> AppResult<Option<RunStart>> {
        self.ensure_healthy()?;
        match (command, self.sim) {
            ("RUN", SimState::Running { .. }) => Ok(None),
            ("RUN", _) => {
                *next_run += 1;
                self.sim = SimState::Running { run: *next_run };
                Ok(Some(RunStart {
                    lease: RunLease {
                        workspace: self.id,
                        run: *next_run,
                    },
                    period_us: self.period_us(),
                }))
            }
            ("PAUSE", SimState::Running { .. }) => {
                self.sim = SimState::Paused;
                Ok(None)
            }
            ("PAUSE", _) => Ok(None),
            ("STEP", SimState::Running { .. }) => Err(AppError::SimulationRunning),
            ("STEP", _) => {
                self.step()?;
                self.sim = SimState::Paused;
                Ok(None)
            }
            ("STOP", _) => {
                self.sim = SimState::Stopped;
                self.engine(LabSession::reset_world)?;
                Ok(None)
            }
            (other, _) => Err(AppError::InvalidRequest(format!(
                "Unknown simulation command '{other}'."
            ))),
        }
    }

    fn step(&mut self) -> AppResult<CycleOutcome> {
        self.engine(|s| s.step_cycle().map(|o| o.header.outcome))
    }

    pub fn set_speed(&mut self, percent: u32) -> AppResult<()> {
        if !(10..=1000).contains(&percent) {
            return Err(AppError::InvalidRequest(
                "Speed must be between 10 % and 1000 %.".into(),
            ));
        }
        self.speed_percent = percent;
        self.project.simulation.speed_percent = percent;
        Ok(())
    }

    /// Advance one cycle for the host pacing loop. A PLC fault pauses the simulation so
    /// the faulting cycle stays on screen.
    pub fn tick(&mut self, lease: RunLease) -> AppResult<TickOutcome> {
        if lease.workspace != self.id || self.sim != (SimState::Running { run: lease.run }) {
            return Ok(TickOutcome::Stale);
        }
        let cycle = self.session.cycle();
        let outcome = self.step()?;
        if outcome == CycleOutcome::Faulted {
            self.sim = SimState::Paused;
        }
        Ok(TickOutcome::Advanced {
            running: matches!(self.sim, SimState::Running { .. }),
            next_period_us: self.period_us(),
            wake: lease.wake(cycle),
        })
    }

    pub fn driver_failed(&mut self, lease: RunLease, message: String) {
        if lease.workspace == self.id && self.sim == (SimState::Running { run: lease.run }) {
            self.sim = SimState::Paused;
            self.failure.get_or_insert(message);
        }
    }

    // ---- operator and engineering actions -------------------------------------------

    pub fn operate(&mut self, component: &str, action: &OperateActionDto) -> AppResult<()> {
        let component = component.to_owned();
        let action = match action {
            OperateActionDto::Press => OperatorAction::Press { component },
            OperateActionDto::Release => OperatorAction::Release { component },
            OperateActionDto::Selector { position } => OperatorAction::SetSelector {
                component,
                position: *position,
            },
            OperateActionDto::Presence { present } => OperatorAction::SetPresence {
                component,
                present: *present,
            },
        };
        self.engine(|s| s.operate(action))
    }

    pub fn force(&mut self, signal: u32, value: Option<SignalValueDto>) -> AppResult<()> {
        let value = value
            .map(|v| {
                convert::value_from_dto(v).ok_or_else(|| {
                    AppError::InvalidRequest("A forced REAL value must be finite.".into())
                })
            })
            .transpose()?;
        self.engine(|s| s.force(SignalId(signal), value))
    }

    pub fn note_evidence(&mut self, evidence: &str) -> AppResult<()> {
        let evidence = evidence.trim();
        if evidence.is_empty() || evidence.len() > 80 {
            return Err(AppError::InvalidRequest(
                "Evidence names have 1 to 80 characters.".into(),
            ));
        }
        if self.is_fault_finding()
            && !self.investigation.evidence.iter().any(|e| e == evidence)
            && self.investigation.evidence.len() < MAX_EVIDENCE
        {
            self.investigation.evidence.push(evidence.to_owned());
            self.persist_investigation();
        }
        Ok(())
    }

    pub fn measure(&mut self, channel: &ChannelRefDto) -> AppResult<MeasurementDto> {
        let c = ChannelRef {
            rack: channel.rack,
            slot: channel.slot,
            channel: channel.channel,
        };
        let address = self.session.channel_address(c);
        self.note_evidence(&format!("measure {c}"))?;
        Ok(
            match self.session.station().measure(c, self.session.scene()) {
                Measurement::Millivolts(mv) => MeasurementDto {
                    kind: "VOLTAGE".into(),
                    value: mv,
                    text: format!("{:.2} V DC at {c} ({address})", mv as f64 / 1000.0),
                },
                Measurement::Microamps(ua) => MeasurementDto {
                    kind: "CURRENT".into(),
                    value: ua,
                    text: format!(
                        "{:.2} mA loop current at {c} ({address})",
                        ua as f64 / 1000.0
                    ),
                },
                Measurement::NoModule => MeasurementDto {
                    kind: "NO_MODULE".into(),
                    value: 0,
                    text: format!("No module terminal at {c}."),
                },
            },
        )
    }

    pub fn ping(&mut self, ip: &str) -> AppResult<PingDto> {
        let target = Ipv4::parse(ip)
            .ok_or_else(|| AppError::InvalidRequest(format!("'{ip}' is not an IPv4 address.")))?;
        self.note_evidence(&format!("ping {target}"))?;
        Ok(match self.session.network().ping(target) {
            PingResult::Reply { round_trip_us } => PingDto {
                result: "REPLY".into(),
                round_trip_us: Some(ExactU64(round_trip_us)),
                text: format!(
                    "Reply from {target}: time={:.1} ms",
                    round_trip_us as f64 / 1000.0
                ),
            },
            PingResult::Timeout => PingDto {
                result: "TIMEOUT".into(),
                round_trip_us: None,
                text: format!("Request to {target} timed out."),
            },
            PingResult::NoRoute => PingDto {
                result: "NO_ROUTE".into(),
                round_trip_us: None,
                text: format!("{target} is outside the PLC subnet and no gateway is configured."),
            },
            PingResult::DuplicateReplies => PingDto {
                result: "DUPLICATE".into(),
                round_trip_us: None,
                text: format!("More than one device answered for {target}: IP address conflict."),
            },
            PingResult::NoInterface => PingDto {
                result: "NO_INTERFACE".into(),
                round_trip_us: None,
                text: "The PLC has no configured Ethernet interface.".into(),
            },
        })
    }

    /// Whether a diagnosis on record names a root cause.
    pub fn diagnosed(&self) -> bool {
        self.exercise
            .as_ref()
            .and_then(|e| e.troubleshooting.as_ref())
            .is_some_and(|t| {
                self.investigation
                    .diagnoses
                    .iter()
                    .any(|d| t.root_causes.iter().any(|r| r.as_str() == d))
            })
    }

    pub fn diagnose(&mut self, option: &str) -> AppResult<DiagnosisResultDto> {
        let t = self
            .exercise
            .as_ref()
            .and_then(|e| e.troubleshooting.as_ref())
            .ok_or_else(|| {
                AppError::InvalidRequest("This workspace is not a troubleshooting exercise.".into())
            })?;
        if !t.diagnosis_options.iter().any(|o| o.id.as_str() == option) {
            return Err(AppError::NotFound {
                what: "diagnosis option",
                id: option.to_owned(),
            });
        }
        let correct = t.root_causes.iter().any(|r| r.as_str() == option);
        let repairs = if correct {
            crate::library::repair_options(t)
        } else {
            Vec::new()
        };
        if self.investigation.diagnoses.len() < 64 {
            self.investigation.diagnoses.push(option.to_owned());
        }
        self.persist_investigation();
        Ok(DiagnosisResultDto {
            correct,
            message: if correct {
                "Your diagnosis is consistent with the evidence. Repair the fault, then verify the system.".into()
            } else {
                "This cause does not explain all observed symptoms. Compare the PLC values with the field measurements again.".into()
            },
            attempts: u32::try_from(self.investigation.diagnoses.len()).unwrap_or(u32::MAX),
            repairs,
        })
    }

    pub fn repair(&mut self, repair: &str) -> AppResult<String> {
        // The repair options reveal the cause, so they are available only after it was named.
        if !self.diagnosed() {
            return Err(AppError::InvalidRequest(
                "Submit a diagnosis that explains the evidence before repairing.".into(),
            ));
        }
        let option = self
            .exercise
            .as_ref()
            .and_then(|e| e.troubleshooting.as_ref())
            .and_then(|t| t.repairs.iter().find(|r| r.id.as_str() == repair))
            .cloned()
            .ok_or_else(|| AppError::NotFound {
                what: "repair",
                id: repair.to_owned(),
            })?;
        self.engine(|s| s.repair(&option.label, &option.clears))?;
        if !self.investigation.repairs.iter().any(|r| r == repair) {
            self.investigation.repairs.push(repair.to_owned());
        }
        self.persist_investigation();
        Ok(format!("{} — done.", option.label))
    }

    fn persist_investigation(&mut self) {
        let state = self
            .project
            .exercise_state
            .get_or_insert_with(ExerciseState::default);
        state.diagnoses = self.investigation.diagnoses.clone();
        state.evidence = self.investigation.evidence.clone();
        state.repairs = self.investigation.repairs.clone();
    }

    /// Hidden faults still active, as injections usable by an isolated assessment.
    pub fn remaining_faults(&self) -> Vec<FaultInjection> {
        let active: BTreeSet<String> = self
            .session
            .active_fault_ids()
            .map(ToString::to_string)
            .collect();
        self.exercise
            .as_ref()
            .and_then(|e| e.troubleshooting.as_ref())
            .map(|t| {
                t.faults
                    .iter()
                    .filter(|f| active.contains(f.id.as_str()))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn exercise_id(&self) -> Option<String> {
        match &self.project.kind {
            ProjectKind::Exercise { exercise, .. } => Some(exercise.to_string()),
            ProjectKind::FreeLab => None,
        }
    }

    // ---- projections ----------------------------------------------------------------

    pub fn frame(&self, watch: &[u32]) -> LiveFrameDto {
        let s = &self.session;
        let scene = s.scene();
        let mut components = Vec::new();
        let mut alarms = Vec::new();
        for index in 0..scene.component_count() {
            let id = scene
                .component_id(index)
                .map(ToString::to_string)
                .unwrap_or_default();
            let quantities = scene.quantities(index);
            for (name, q) in &quantities {
                if let (Quantity::Bool(true), Some((kind, text))) = (q, alarm_text(name)) {
                    alarms.push(AlarmDto {
                        component: id.clone(),
                        kind: kind.into(),
                        text: text.into(),
                    });
                }
            }
            components.push(ComponentStateDto {
                id,
                quantities: quantities
                    .into_iter()
                    .filter(|(n, _)| !HIDDEN_QUANTITIES.contains(n))
                    .map(|(name, q)| QuantityDto {
                        name: name.into(),
                        value: match q {
                            Quantity::Bool(value) => SignalValueDto::Bool { value },
                            Quantity::Integer(value) => SignalValueDto::Count { value },
                        },
                    })
                    .collect(),
            });
        }
        let watch: Vec<SignalId> = watch.iter().take(MAX_WATCH).map(|s| SignalId(*s)).collect();
        let stats = s.trace_stats();
        LiveFrameDto {
            workspace_id: ExactU64(self.id),
            world_time_us: ExactU64(s.now().as_micros()),
            cycle: ExactU64(s.cycle()),
            simulation: SimulationStatusDto {
                state: match self.sim {
                    SimState::Stopped => "STOPPED",
                    SimState::Paused => "PAUSED",
                    SimState::Running { .. } => "RUNNING",
                }
                .into(),
                speed_percent: self.speed_percent,
                run_id: match self.sim {
                    SimState::Running { run } => Some(ExactU64(run)),
                    _ => None,
                },
                cycle_us: ExactU64(s.config().cycle_us),
                failure: self.failure.clone(),
            },
            plc: PlcStatusDto {
                mode: convert::mode(s.plc_mode()).into(),
                switch: match s.plc_switch() {
                    PlcModeRequest::Run => "RUN",
                    PlcModeRequest::Stop => "STOP",
                }
                .into(),
                program_name: s.program().map(|(name, _, _)| name.to_owned()),
                program_revision: s.program().map(|(_, revision, _)| ExactU64(revision)),
                segment: s.plc_segment(),
                scans: ExactU64(s.plc_scans()),
                task_time_us: ExactU64(s.plc_task_time_us()),
                fault: s.plc_fault().map(convert::diagnostic),
                forces: u32::try_from(s.forces().len()).unwrap_or(u32::MAX),
                leds: convert::cpu_leds(s),
            },
            components,
            watch: s
                .watch(&watch)
                .into_iter()
                .map(|(id, v)| WatchValueDto {
                    signal: id.0,
                    value: v.map(convert::value),
                })
                .collect(),
            cause_effect: s.latest_cause_effect().as_ref().map(convert::cause_effect),
            alarms,
            active_diagnostics: u32::try_from(s.station().active_diagnostics().count())
                .unwrap_or(u32::MAX),
            latest_event: s.events().latest().map(|e| ExactU64(e.0)),
            trace: TraceStatsDto {
                retained_cycles: ExactU64(stats.retained_records as u64),
                dropped_cycles: ExactU64(stats.dropped_records),
                used_bytes: ExactU64(stats.used_bytes as u64),
                budget_bytes: ExactU64(stats.byte_budget as u64),
            },
            signals_revision: self.signals_revision,
        }
    }

    pub fn scene_dto(&self) -> SceneDto {
        let scene = self.session.scene();
        SceneDto {
            id: self.scene.id.to_string(),
            title: self.scene.title.clone(),
            summary: self.scene.summary.clone(),
            components: self
                .scene
                .components
                .iter()
                .enumerate()
                .map(|(index, c)| {
                    let kind = serde_json::to_value(&c.kind).unwrap_or_default();
                    let config = kind
                        .as_object()
                        .map(|o| {
                            o.iter()
                                .filter(|(k, _)| {
                                    k.as_str() != "type"
                                        && k.as_str() != "positions"
                                        && k.as_str() != "color"
                                })
                                .map(|(k, v)| ConfigItemDto {
                                    key: k.clone(),
                                    value: v.as_str().map_or_else(|| v.to_string(), str::to_owned),
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    ComponentDto {
                        id: c.id.to_string(),
                        label: c.label.clone(),
                        kind: c.kind.type_name().into(),
                        color: kind
                            .get("color")
                            .and_then(|v| v.as_str())
                            .map(str::to_owned),
                        positions: kind
                            .get("positions")
                            .and_then(|v| v.as_array())
                            .map(|a| {
                                a.iter()
                                    .filter_map(|p| p.as_str().map(str::to_owned))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        operable: scene.is_operable(index),
                        config,
                    }
                })
                .collect(),
            layout: self
                .scene
                .layout
                .iter()
                .map(|l| LayoutDto {
                    component: l.component.to_string(),
                    x: l.x,
                    y: l.y,
                    width: l.width,
                    height: l.height,
                })
                .collect(),
            couplings: self
                .scene
                .couplings
                .iter()
                .map(|c| match c {
                    Coupling::ValveFillsTank { valve, tank } => ("valveFillsTank", valve, tank),
                    Coupling::ValveDrainsTank { valve, tank } => ("valveDrainsTank", tank, valve),
                    Coupling::PumpDrainsTank { pump, tank } => ("pumpDrainsTank", tank, pump),
                })
                .map(|(kind, from, to)| CouplingDto {
                    kind: kind.into(),
                    from: from.to_string(),
                    to: to.to_string(),
                })
                .collect(),
            interface: self
                .scene
                .interface
                .iter()
                .map(|i| InterfaceSignalDto {
                    tag: i.tag.to_string(),
                    data_type: format!("{:?}", i.tag_type).to_uppercase(),
                    direction: crate::library::direction(i.direction).into(),
                    description: i.description.clone(),
                })
                .collect(),
            has_network: self.scene.network.is_some(),
        }
    }

    pub fn variables(&self, filter: Option<&str>, offset: u32, limit: u16) -> VariablesDto {
        let (total, rows) =
            self.session
                .variables(filter, offset as usize, usize::from(limit.clamp(1, 500)));
        VariablesDto {
            total: u32::try_from(total).unwrap_or(u32::MAX),
            rows: rows
                .into_iter()
                .map(|r| VariableRowDto {
                    signal: r.signal.0,
                    name: r.name,
                    kind: convert::signal_kind(r.kind).into(),
                    scan_value: r.scan_value.map(convert::value),
                    committed: r.committed.map(convert::value),
                    staged: r.staged.map(convert::value),
                    source: r.source,
                    quality: r.quality.map(|q| convert::quality(q).into()),
                    forced: r.forced.map(convert::value),
                })
                .collect(),
        }
    }

    pub fn trend(
        &self,
        signals: &[u32],
        from_us: u64,
        to_us: u64,
        max_points: u16,
    ) -> AppResult<TrendDto> {
        if to_us < from_us {
            return Err(AppError::InvalidRequest(
                "The trend window ends before it starts.".into(),
            ));
        }
        let ids: Vec<SignalId> = signals.iter().take(16).map(|s| SignalId(*s)).collect();
        let window = self.session.retained_window();
        Ok(TrendDto {
            series: self
                .session
                .trend(&ids, from_us, to_us, usize::from(max_points))
                .into_iter()
                .map(|t| TrendSeriesDto {
                    signal: t.signal.0,
                    layer: match t.layer {
                        TrendLayer::Inputs => "INPUTS",
                        TrendLayer::Committed => "COMMITTED",
                        TrendLayer::Process => "PROCESS",
                    }
                    .into(),
                    points: t
                        .points
                        .into_iter()
                        .map(|(t, v)| TrendPointDto {
                            t: ExactU64(t),
                            value: convert::value(v),
                        })
                        .collect(),
                    decimated: t.decimated,
                })
                .collect(),
            oldest_us: window.map(|w| ExactU64(w.0)),
            latest_us: window.map(|w| ExactU64(w.1)),
        })
    }

    pub fn events(&self, after: Option<u64>, limit: u16) -> EventsPageDto {
        let log = self.session.events();
        let page = log.page(
            after.map(EventSeq),
            usize::from(limit.clamp(1, 500)),
            512 * 1024,
        );
        EventsPageDto {
            events: page
                .entries
                .iter()
                .map(|(seq, e)| self.event_dto(*seq, e))
                .collect(),
            gap: page.gap,
            dropped: ExactU64(page.dropped),
            latest: page.latest.map(|s| ExactU64(s.0)),
        }
    }

    fn event_dto(&self, seq: EventSeq, e: &lab_engine::LabEvent) -> EventDto {
        let (kind, severity, text, source) = convert::describe(&self.session, e);
        EventDto {
            seq: ExactU64(seq.0),
            time_us: ExactU64(e.time_us),
            cycle: ExactU64(e.cycle),
            kind,
            severity: severity.into(),
            text,
            source,
        }
    }

    pub fn commissioning(&self) -> CommissioningDto {
        let state = |s: lab_engine::CheckState| {
            match s {
                lab_engine::CheckState::Ok => "OK",
                lab_engine::CheckState::Attention => "ATTENTION",
                lab_engine::CheckState::Fault => "FAULT",
                lab_engine::CheckState::Untested => "UNTESTED",
                lab_engine::CheckState::NotApplicable => "NOT_APPLICABLE",
            }
            .to_owned()
        };
        CommissioningDto {
            categories: self
                .session
                .commissioning()
                .categories
                .into_iter()
                .map(|c| CheckCategoryDto {
                    id: c.id.into(),
                    title: c.title.into(),
                    state: state(c.state()),
                    items: c
                        .items
                        .into_iter()
                        .map(|i| CheckItemDto {
                            id: i.id.into(),
                            title: i.title.into(),
                            state: state(i.state),
                            findings: i.findings,
                            how: i.how.into(),
                        })
                        .collect(),
                })
                .collect(),
            feedback_wait_s: u32::try_from(lab_engine::FEEDBACK_WAIT_US / 1_000_000)
                .unwrap_or(u32::MAX),
        }
    }

    pub fn hardware(&self) -> AppResult<HardwareDto> {
        let s = &self.session;
        let profile = s.profile();
        let station = s.station();
        let cpu = station.configuration().cpu;
        let mut diagnostics: Vec<DiagnosticEntryDto> = station
            .diagnostics()
            .entries()
            .rev()
            .take(200)
            .map(|e| convert::diagnostic_entry(s, e))
            .collect();
        diagnostics.reverse();
        Ok(HardwareDto {
            station: convert::station_to_dto(station.configuration())
                .map_err(|e| AppError::InvalidRequest(e.message))?,
            cpu: cpu.id().into(),
            cpu_name: convert::module_name(cpu, profile),
            cpu_product: profile.product_name(cpu).map(str::to_owned),
            cpu_leds: convert::cpu_leds(s),
            modules: station
                .module_views()
                .into_iter()
                .map(|m| ModuleViewDto {
                    module_id: station
                        .configuration()
                        .module_at(m.position)
                        .map(|c| c.id.to_string()),
                    position: SlotRefDto {
                        rack: m.position.rack,
                        slot: m.position.slot,
                    },
                    configured: m.configured.map(|t| t.id().into()),
                    installed: m.installed.map(|t| t.id().into()),
                    configured_name: m.configured.map(|t| convert::module_name(t, profile)),
                    installed_name: m.installed.map(|t| convert::module_name(t, profile)),
                    product: m.product,
                    status: convert::module_status(m.status).into(),
                    status_led: convert::led(m.status_led).into(),
                    channels: m
                        .channels
                        .into_iter()
                        .map(|c| ChannelViewDto {
                            channel_id: station.configuration().channel_id(
                                lab_model::hardware::ChannelRef {
                                    rack: m.position.rack,
                                    slot: m.position.slot,
                                    channel: c.channel,
                                },
                            ),
                            channel: c.channel,
                            mode: convert::channel_mode(c.mode).into(),
                            tag: c.tag,
                            address: c.address,
                            value: c.reading.map(|r| convert::value(channel_value(r.value))),
                            quality: c.reading.map(|r| convert::quality(r.quality).into()),
                            output: c.output,
                            led: c.led,
                        })
                        .collect(),
                })
                .collect(),
            diagnostics,
            catalog: lab_model::hardware::ModuleType::ALL
                .iter()
                .filter(|t| t.is_io())
                .map(|t| ModuleTypeDto {
                    id: t.id().into(),
                    name: t.generic_name().into(),
                    product: profile.product_name(*t).map(str::to_owned),
                    channels: t.channel_count(),
                    class: convert::module_class(*t).into(),
                })
                .collect(),
            findings: station
                .compare(s.scene())
                .into_iter()
                .map(convert::hardware_finding)
                .collect(),
            signals: s
                .signal_map()
                .into_iter()
                .map(|r| SignalMapRowDto {
                    tag: r.tag,
                    channel: ChannelRefDto {
                        rack: r.channel.rack,
                        slot: r.channel.slot,
                        channel: r.channel.channel,
                    },
                    channel_id: r.channel_id,
                    address: r.address,
                    value: r.reading.map(|v| convert::value(channel_value(v.value))),
                    quality: r.reading.map(|v| convert::quality(v.quality).into()),
                    output: r.output,
                    drawn_device: r.drawn_device,
                    drawn_port: r.drawn_port,
                    device_state: r.device_state,
                })
                .collect(),
        })
    }

    pub fn network(&self) -> NetworkDto {
        let s = &self.session;
        let net = s.network();
        let interface = s
            .station()
            .configuration()
            .network
            .interface
            .as_ref()
            .map(|i| InterfaceDto {
                ip: i.ip.to_string(),
                mask: i.mask.to_string(),
                gateway: i.gateway.map(|g| g.to_string()),
            });
        let config = &s.station().configuration().network;
        let network_events: Vec<_> = s
            .events()
            .tail(400)
            .into_iter()
            .filter(|(_, e)| matches!(e.kind, lab_engine::LabEventKind::Network { .. }))
            .collect();
        let traffic: Vec<TrafficDto> = network_events
            .iter()
            .filter_map(|(seq, e)| match &e.kind {
                lab_engine::LabEventKind::Network { event } => {
                    Some(convert::traffic(seq.0, event, config))
                }
                _ => None,
            })
            .collect();
        let last = |scan: &str, kind: &str| {
            traffic
                .iter()
                .rev()
                .find(|t| t.connection == scan && t.kind == kind)
                .map(|t| t.time_us)
        };
        NetworkDto {
            plc_interface: interface,
            plc_link_up: net.plc_link_up(),
            scans: net
                .scan_views()
                .into_iter()
                .enumerate()
                .map(|(index, v)| ScanViewDto {
                    id: v.id.to_string(),
                    server: v.server.to_string(),
                    unit_id: v.unit_id,
                    status: convert::scan_status(v.status),
                    connected: v.connected,
                    requests: ExactU64(v.requests),
                    responses: ExactU64(v.responses),
                    timeouts: ExactU64(v.timeouts),
                    exceptions: ExactU64(v.exceptions),
                    last_round_trip_us: v.last_round_trip_us.map(ExactU64),
                    registers: v.registers,
                    request: config
                        .modbus_tcp
                        .get(index)
                        .map(convert::modbus_request)
                        .unwrap_or_default(),
                    values: config.modbus_tcp.get(index).map_or_else(Vec::new, |scan| {
                        scan.values
                            .iter()
                            .enumerate()
                            .map(|(m, mapping)| convert::scan_value(mapping, net.value(index, m)))
                            .collect()
                    }),
                    last_request_us: last(&v.id.to_string(), "request"),
                    last_response_us: last(&v.id.to_string(), "response"),
                })
                .collect(),
            devices: net
                .device_views()
                .into_iter()
                .map(|d| DeviceViewDto {
                    id: d.id.to_string(),
                    label: d.label,
                    ip: d.ip.to_string(),
                    mask: d.mask.to_string(),
                    link_up: d.link_up,
                    online: d.online,
                    modbus_server: d.modbus_server,
                    requests_served: ExactU64(d.requests_served),
                    last_activity_us: d.last_activity.map(|t| ExactU64(t.as_micros())),
                    same_subnet: config
                        .interface
                        .as_ref()
                        .map(|i| i.ip.same_subnet(d.ip, i.mask)),
                })
                .collect(),
            traffic: {
                let skip = traffic.len().saturating_sub(MAX_TRAFFIC);
                traffic.into_iter().skip(skip).collect()
            },
        }
    }

    pub fn workspace_dto(&self, project_id: &str, exercise: Option<ExerciseDto>) -> WorkspaceDto {
        WorkspaceDto {
            workspace_id: ExactU64(self.id),
            project_id: project_id.to_owned(),
            scene: self.scene_dto(),
            platform: convert::platform(&self.profile),
            signals: convert::signals(&self.session),
            exercise,
            frame: self.frame(&[]),
        }
    }
}

fn binding_issue(i: &lab_engine::BindingIssue) -> BindingIssueDto {
    BindingIssueDto {
        tag: i.tag.clone(),
        location: i.location.clone(),
        message: i.message.clone(),
        blocking: i.blocking,
    }
}

fn channel_value(v: hardware_model::ChannelValue) -> lab_engine::SignalValue {
    match v {
        hardware_model::ChannelValue::Bool(b) => lab_engine::SignalValue::Bool(b),
        hardware_model::ChannelValue::Int(i) => lab_engine::SignalValue::Int(i),
        hardware_model::ChannelValue::Real(r) => lab_engine::SignalValue::Real(r),
    }
}

fn alarm_text(quantity: &str) -> Option<(&'static str, &'static str)> {
    Some(match quantity {
        "overflowing" => ("OVERFLOW", "Tank overflow"),
        "highHigh" => ("HIGH_HIGH", "Level high-high"),
        "lowLow" => ("LOW_LOW", "Level low-low"),
        "dry" => ("DRY_RUN", "Pump running dry"),
        "tripped" => ("OVERLOAD", "Motor overload tripped"),
        _ => return None,
    })
}
