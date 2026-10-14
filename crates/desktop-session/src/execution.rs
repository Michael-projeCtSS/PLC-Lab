//! Deterministic application control. Host scheduling lives outside this crate.
use super::*;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    #[default]
    Stopped,
    Running,
    Paused,
}
impl Mode {
    pub(super) fn dto(self) -> ExecutionMode {
        match self {
            Self::Stopped => ExecutionMode::Stopped,
            Self::Running => ExecutionMode::Running,
            Self::Paused => ExecutionMode::Paused,
        }
    }
}

/// Unforgeable through the IPC contract: only start_execution can issue a lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunLease {
    session_id: RuntimeSessionId,
    epoch: u64,
}
impl RunLease {
    pub fn wake(self) -> ExecutionWake {
        ExecutionWake {
            contract_version: CONTRACT_VERSION,
            session_id: self.session_id,
            run_id: ExecutionRunId::new(self.epoch).expect("Issued epoch is nonzero."),
        }
    }
}
pub struct ExecutionStart {
    pub snapshot: RuntimeSnapshotDto,
    /// Only the caller that changed mode receives driver ownership.
    pub driver: Option<RunLease>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickOutcome {
    Stale,
    Observed { running: bool },
}

impl LoadedSession {
    pub(super) fn require_stopped(&self) -> Result<(), IpcError> {
        if self.mode == Mode::Stopped {
            Ok(())
        } else {
            Err(application_error(
                "APP-EXECUTION-ACTIVE",
                "Stop execution before editing the loaded program or process topology.",
            ))
        }
    }
    fn require_ready(&self) -> Result<(), IpcError> {
        if self.runtime.state() != RuntimeState::Ready {
            Err(application_error(
                "APP-EXECUTION-FAULTED",
                "Reset the faulted runtime before execution control.",
            ))
        } else {
            Ok(())
        }
    }
    pub(super) fn halt(&mut self) {
        self.mode = Mode::Stopped;
        self.active_lease = None;
    }
    /// The only PLC/process cycle path, shared by manual and leased execution.
    pub(super) fn execute_one_cycle(&mut self) -> Result<RuntimeSnapshotDto, IpcError> {
        let result = match self.runtime.single_scan() {
            Ok(_) => {
                if let Some(process) = &mut self.process
                    && let Err(error) =
                        process.advance(&mut self.runtime, self.config.cycle_period())
                {
                    self.halt();
                    self.process_failed = true;
                    return Err(error);
                }
                self.record(EventKind::ScanCompleted, None)
            }
            Err(plc_runtime::ScanError::Fault(_)) => {
                self.halt();
                self.record(EventKind::ScanFaulted, None)
            }
            Err(plc_runtime::ScanError::Internal(_)) => {
                self.halt();
                self.snapshot()
            }
            Err(plc_runtime::ScanError::Faulted) => self.snapshot(),
        };
        if result.is_err() {
            self.halt();
        }
        result
    }
}

impl RuntimeSessionService {
    pub fn start_execution(
        &self,
        request: RuntimeSessionRequest,
    ) -> Result<ExecutionStart, IpcError> {
        let mut driver = None;
        let snapshot = self.with_session(request, |s| {
            s.require_ready()?;
            if s.mode == Mode::Running {
                return s.snapshot();
            }
            let epoch = s.epoch.checked_add(1).ok_or_else(|| {
                application_error(
                    "APP-EXECUTION-EPOCH-EXHAUSTED",
                    "The continuous execution identity range is exhausted.",
                )
            })?;
            let kind = if s.mode == Mode::Paused {
                EventKind::ExecutionResumed
            } else {
                EventKind::ExecutionStarted
            };
            let lease = RunLease {
                session_id: s.id,
                epoch,
            };
            s.epoch = epoch;
            s.mode = Mode::Running;
            s.active_lease = Some(lease);
            let result = s.record(kind, None);
            if result.is_ok() {
                driver = Some(lease);
            } else {
                s.halt();
            }
            result
        })?;
        Ok(ExecutionStart { snapshot, driver })
    }
    pub fn pause_execution(
        &self,
        request: RuntimeSessionRequest,
    ) -> Result<RuntimeSnapshotDto, IpcError> {
        self.with_session(request, |s| {
            s.require_ready()?;
            if s.mode != Mode::Running {
                return s.snapshot();
            }
            s.mode = Mode::Paused;
            s.active_lease = None;
            s.record(EventKind::ExecutionPaused, None)
        })
    }
    pub fn stop_execution(
        &self,
        request: RuntimeSessionRequest,
    ) -> Result<RuntimeSnapshotDto, IpcError> {
        self.with_session(request, |s| {
            s.require_ready()?;
            if s.mode == Mode::Stopped {
                return s.snapshot();
            }
            s.halt();
            s.record(EventKind::ExecutionStopped, None)
        })
    }
    /// Host-only operation. No Tauri command accepts a lease or exposes this call.
    pub fn continuous_tick(&self, lease: RunLease) -> Result<TickOutcome, IpcError> {
        let mut slot = self.lock()?;
        let Some(s) = slot.loaded.as_mut().filter(|s| {
            s.id == lease.session_id && s.mode == Mode::Running && s.active_lease == Some(lease)
        }) else {
            return Ok(TickOutcome::Stale);
        };
        if s.process_failed {
            s.halt();
            return Err(application_error(
                "APP-PROCESS-INTERNAL",
                "The process session is unavailable after an application failure.",
            ));
        }
        s.execute_one_cycle()?;
        Ok(TickOutcome::Observed {
            running: s.mode == Mode::Running,
        })
    }
    /// Fail closed if a host worker cannot finish. Cannot stop a newer lease.
    pub fn driver_failed(&self, lease: RunLease, error: IpcError) -> Result<(), IpcError> {
        let mut slot = self.lock()?;
        if let Some(s) = slot
            .loaded
            .as_mut()
            .filter(|s| s.active_lease == Some(lease))
        {
            s.halt();
            s.driver_error = Some(error);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroUsize;
    const TANK: &str = include_str!("../../../scenes/tank-filling/example.st");
    fn load(s: &RuntimeSessionService, source: &str) -> RuntimeSessionRequest {
        let LoadProgramResponse::Loaded { snapshot } = s
            .load(LoadStProgramRequest {
                contract_version: CONTRACT_VERSION,
                source: source.into(),
            })
            .unwrap()
        else {
            panic!("Valid ST required")
        };
        RuntimeSessionRequest {
            contract_version: CONTRACT_VERSION,
            session_id: snapshot.session_id,
        }
    }
    fn plant(s: &RuntimeSessionService, r: &RuntimeSessionRequest) {
        s.attach_process(AttachProcessSceneRequest {
            contract_version: CONTRACT_VERSION,
            session_id: r.session_id,
            scene_kind: ProcessSceneKind::TankFilling,
        })
        .unwrap();
    }
    fn page(s: &RuntimeSessionService, r: &RuntimeSessionRequest) -> TracePageDto {
        s.trace_page(TracePageRequest {
            contract_version: CONTRACT_VERSION,
            session_id: r.session_id,
            after_sequence: None,
            limit: 256,
        })
        .unwrap()
    }
    fn semantic(mut s: RuntimeSnapshotDto) -> RuntimeSnapshotDto {
        // Application controller metadata is not PLC/process semantics.
        s.execution_mode = ExecutionMode::Stopped;
        s.execution_run_id = None;
        s
    }
    #[test]
    fn manual_and_authorized_cycles_have_identical_memory_ton_clock_process_and_trace() {
        let combined = TANK.replace("InletValve :=", "VAR Delay:TON; Count:INT; END_VAR Count := Count + 1; Delay(IN := TRUE, PT := T#300ms); InletValve :=");
        let source = combined.as_str();
        for source in [
            source,
            "PROGRAM Main VAR_INPUT Start:BOOL; END_VAR VAR_OUTPUT Done:BOOL; END_VAR VAR Delay:TON; Count:INT; END_VAR Count := Count + 1; Delay(IN := Start, PT := T#300ms, Q => Done); END_PROGRAM",
        ] {
            let manual = RuntimeSessionService::default();
            let continuous = RuntimeSessionService::default();
            let a = load(&manual, source);
            let b = load(&continuous, source);
            if source.contains("LevelPercent") {
                plant(&manual, &a);
                plant(&continuous, &b);
            }
            for (s, r) in [(&manual, &a), (&continuous, &b)] {
                let snapshot = s.snapshot(r.clone()).unwrap();
                if let Some(input) = snapshot.variables.iter().find(|v| v.name == "Start") {
                    s.set_input(SetRuntimeInputRequest {
                        contract_version: CONTRACT_VERSION,
                        session_id: r.session_id,
                        variable_id: input.variable_id,
                        value: RuntimeValue::Bool { value: true },
                    })
                    .unwrap();
                }
            }
            let started = continuous.start_execution(b.clone()).unwrap();
            assert_eq!(started.snapshot.completed_scans.get(), 0);
            let lease = started.driver.unwrap();
            assert!(
                continuous
                    .start_execution(b.clone())
                    .unwrap()
                    .driver
                    .is_none()
            );
            for _ in 0..20 {
                let expected = manual.single_scan(a.clone()).unwrap();
                assert_eq!(
                    continuous.continuous_tick(lease).unwrap(),
                    TickOutcome::Observed { running: true }
                );
                assert_eq!(semantic(continuous.snapshot(b.clone()).unwrap()), expected);
                let left = manual.inner.lock().unwrap();
                let right = continuous.inner.lock().unwrap();
                assert_eq!(
                    left.loaded.as_ref().unwrap().runtime.snapshot(),
                    right.loaded.as_ref().unwrap().runtime.snapshot()
                );
            }
            let projection = |s: &RuntimeSessionService, r: &RuntimeSessionRequest| {
                page(s, r)
                    .entries
                    .into_iter()
                    .filter(|e| e.kind == TraceEventKind::ScanCompleted)
                    .map(|mut e| {
                        e.sequence = TraceSequenceId::new(1).unwrap();
                        e.execution_mode = ExecutionMode::Stopped;
                        e.execution_run_id = None;
                        e
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(projection(&manual, &a), projection(&continuous, &b));
        }
    }
    #[test]
    fn control_trace_pause_stop_resume_and_stale_leases_preserve_state() {
        let s = RuntimeSessionService::default();
        let r = load(
            &s,
            "PROGRAM Main VAR Count:INT; Delay:TON; END_VAR Count := Count + 1; Delay(IN := TRUE, PT := T#1s); END_PROGRAM",
        );
        let a = s.start_execution(r.clone()).unwrap().driver.unwrap();
        s.continuous_tick(a).unwrap();
        s.continuous_tick(a).unwrap();
        let before = s.snapshot(r.clone()).unwrap();
        let paused = s.pause_execution(r.clone()).unwrap();
        assert_eq!(semantic(paused.clone()), semantic(before));
        assert_eq!(s.continuous_tick(a).unwrap(), TickOutcome::Stale);
        assert_eq!(s.pause_execution(r.clone()).unwrap(), paused);
        let b = s.start_execution(r.clone()).unwrap().driver.unwrap();
        assert_ne!(a, b);
        assert_eq!(s.continuous_tick(a).unwrap(), TickOutcome::Stale);
        s.continuous_tick(b).unwrap();
        let before = s.snapshot(r.clone()).unwrap();
        let stopped = s.stop_execution(r.clone()).unwrap();
        assert_eq!(semantic(before), semantic(stopped.clone()));
        assert_eq!(s.continuous_tick(b).unwrap(), TickOutcome::Stale);
        assert_eq!(s.stop_execution(r.clone()).unwrap(), stopped);
        let p = page(&s, &r);
        assert_eq!(
            p.entries.iter().map(|e| e.kind).collect::<Vec<_>>(),
            [
                TraceEventKind::SessionLoaded,
                TraceEventKind::ExecutionStarted,
                TraceEventKind::ScanCompleted,
                TraceEventKind::ScanCompleted,
                TraceEventKind::ExecutionPaused,
                TraceEventKind::ExecutionResumed,
                TraceEventKind::ScanCompleted,
                TraceEventKind::ExecutionStopped
            ]
        );
        assert_eq!(
            p.entries
                .iter()
                .map(|e| (
                    e.completed_scans.get(),
                    e.simulation_time_micros.as_micros()
                ))
                .collect::<Vec<_>>(),
            [
                (0, 0),
                (0, 0),
                (1, 100000),
                (2, 200000),
                (2, 200000),
                (2, 200000),
                (3, 300000),
                (3, 300000)
            ]
        );
        assert_eq!(s.single_scan(r.clone()).unwrap().completed_scans.get(), 4);
        s.unload(r.clone()).unwrap();
        let next = load(&s, TANK);
        assert_ne!(r.session_id, next.session_id);
        assert_eq!(s.continuous_tick(b).unwrap(), TickOutcome::Stale);
    }
    #[test]
    fn stopped_guards_and_load_commit_race_preserve_active_session_and_intents() {
        let s = RuntimeSessionService::default();
        let r = load(&s, TANK);
        plant(&s, &r);
        let intent = s.reserve_load_intent().unwrap();
        let PreparedLoad::Ready(candidate) = RuntimeSessionService::prepare_load(TANK).unwrap()
        else {
            panic!()
        };
        let a = s.start_execution(r.clone()).unwrap().driver.unwrap();
        let before = s.snapshot(r.clone()).unwrap();
        let trace = page(&s, &r);
        assert_eq!(
            s.commit_prepared_load(intent, candidate).unwrap_err().code,
            "APP-EXECUTION-ACTIVE"
        );
        assert_eq!(s.snapshot(r.clone()).unwrap(), before);
        assert_eq!(page(&s, &r), trace);
        assert_eq!(
            s.continuous_tick(a).unwrap(),
            TickOutcome::Observed { running: true }
        );
        let before = s.snapshot(r.clone()).unwrap();
        let trace = page(&s, &r);
        for paused in [false, true] {
            if paused {
                s.pause_execution(r.clone()).unwrap();
            }
            for result in [
                s.single_scan(r.clone()),
                s.reset(r.clone()),
                s.detach_process(r.clone()),
                s.reset_process(r.clone()),
                s.attach_process(AttachProcessSceneRequest {
                    contract_version: CONTRACT_VERSION,
                    session_id: r.session_id,
                    scene_kind: ProcessSceneKind::TankFilling,
                }),
            ] {
                assert_eq!(result.unwrap_err().code, "APP-EXECUTION-ACTIVE");
            }
            assert_eq!(
                s.unload(r.clone()).unwrap_err().code,
                "APP-EXECUTION-ACTIVE"
            );
            assert_eq!(
                s.load(LoadStProgramRequest {
                    contract_version: CONTRACT_VERSION,
                    source: TANK.into()
                })
                .unwrap_err()
                .code,
                "APP-EXECUTION-ACTIVE"
            );
        }
        assert_eq!(semantic(before), semantic(s.snapshot(r.clone()).unwrap()));
        assert_eq!(page(&s, &r).entries.len(), trace.entries.len() + 1);
        assert_eq!(s.inner.lock().unwrap().latest_load_intent, intent.0);
        assert_eq!(s.continuous_tick(a).unwrap(), TickOutcome::Stale);
    }
    #[test]
    fn automatic_fault_rolls_back_once_stops_before_observation_and_requires_reset() {
        let s = RuntimeSessionService::default();
        let r = load(
            &s,
            "PROGRAM Main VAR_INPUT Divisor:INT; END_VAR VAR Result:INT; Delay:TON; END_VAR Delay(IN := TRUE, PT := T#1s); Result := 10 / Divisor; END_PROGRAM",
        );
        let a = s.start_execution(r.clone()).unwrap().driver.unwrap();
        let before = s.snapshot(r.clone()).unwrap();
        assert_eq!(
            s.continuous_tick(a).unwrap(),
            TickOutcome::Observed { running: false }
        );
        let fault = s.snapshot(r.clone()).unwrap();
        assert_eq!(fault.state, SessionRuntimeState::Faulted);
        assert_eq!(fault.execution_mode, ExecutionMode::Stopped);
        assert_eq!(fault.completed_scans, before.completed_scans);
        assert_eq!(fault.simulation_time_micros, before.simulation_time_micros);
        assert_eq!(fault.variables, before.variables);
        assert_eq!(fault.function_blocks, before.function_blocks);
        assert_eq!(fault.fault.as_ref().unwrap().category, "runtimeFault");
        assert_eq!(s.continuous_tick(a).unwrap(), TickOutcome::Stale);
        assert_eq!(
            s.start_execution(r.clone()).err().unwrap().code,
            "APP-EXECUTION-FAULTED"
        );
        assert_eq!(
            page(&s, &r)
                .entries
                .iter()
                .filter(|e| e.kind == TraceEventKind::ScanFaulted
                    && e.execution_mode == ExecutionMode::Stopped)
                .count(),
            1
        );
        assert_eq!(s.reset(r).unwrap().state, SessionRuntimeState::Ready);
    }
    #[test]
    fn control_versions_and_stale_targets_reject_without_state_or_history_changes() {
        let s = RuntimeSessionService::default();
        let r = load(&s, TANK);
        let before = s.snapshot(r.clone()).unwrap();
        let history = page(&s, &r);
        for (request, code) in [
            (
                RuntimeSessionRequest {
                    contract_version: 6,
                    ..r.clone()
                },
                "APP-IPC-001",
            ),
            (
                RuntimeSessionRequest {
                    session_id: RuntimeSessionId::new(99).unwrap(),
                    ..r.clone()
                },
                "APP-RUNTIME-STALE-SESSION",
            ),
        ] {
            assert_eq!(s.start_execution(request.clone()).err().unwrap().code, code);
            assert_eq!(s.pause_execution(request.clone()).unwrap_err().code, code);
            assert_eq!(s.stop_execution(request).unwrap_err().code, code);
        }
        assert_eq!(s.snapshot(r.clone()).unwrap(), before);
        assert_eq!(page(&s, &r), history);
    }
    #[test]
    fn inputs_remain_ordered_and_process_owned_inputs_reject_in_all_modes() {
        let s = RuntimeSessionService::default();
        let r = load(&s, &TANK.replace("VAR_INPUT", "VAR_INPUT User:BOOL;"));
        plant(&s, &r);
        let vars = s.snapshot(r.clone()).unwrap().variables;
        let user = vars.iter().find(|v| v.name == "User").unwrap().variable_id;
        let sensor = vars
            .iter()
            .find(|v| v.name == "LevelHigh")
            .unwrap()
            .variable_id;
        for mode in [Mode::Stopped, Mode::Running, Mode::Paused] {
            if mode == Mode::Running {
                s.start_execution(r.clone()).unwrap();
            }
            if mode == Mode::Paused {
                s.pause_execution(r.clone()).unwrap();
            }
            let updated = s
                .set_input(SetRuntimeInputRequest {
                    contract_version: CONTRACT_VERSION,
                    session_id: r.session_id,
                    variable_id: user,
                    value: RuntimeValue::Bool { value: true },
                })
                .unwrap();
            assert_eq!(updated.execution_mode, mode.dto());
            assert_eq!(
                s.set_input(SetRuntimeInputRequest {
                    contract_version: CONTRACT_VERSION,
                    session_id: r.session_id,
                    variable_id: sensor,
                    value: RuntimeValue::Bool { value: true }
                })
                .unwrap_err()
                .code,
                "PLC-PROCESS-004"
            );
        }
        assert_eq!(
            page(&s, &r)
                .entries
                .iter()
                .filter(|e| e.kind == TraceEventKind::InputStaged)
                .count(),
            3
        );
    }
    #[test]
    fn epoch_exhaustion_is_application_error_preserving_paused_state_and_history() {
        let s = RuntimeSessionService::default();
        let r = load(&s, TANK);
        plant(&s, &r);
        s.start_execution(r.clone()).unwrap();
        s.pause_execution(r.clone()).unwrap();
        s.inner.lock().unwrap().loaded.as_mut().unwrap().epoch = u64::MAX;
        let before = s.snapshot(r.clone()).unwrap();
        let history = page(&s, &r);
        assert_eq!(
            s.start_execution(r.clone()).err().unwrap().code,
            "APP-EXECUTION-EPOCH-EXHAUSTED"
        );
        assert_eq!(s.snapshot(r.clone()).unwrap(), before);
        assert_eq!(page(&s, &r), history);
    }
    #[test]
    fn host_failure_stops_only_its_lease_and_surfaces_application_error_without_fake_fault() {
        let s = RuntimeSessionService::default();
        let r = load(&s, TANK);
        plant(&s, &r);
        let a = s.start_execution(r.clone()).unwrap().driver.unwrap();
        s.continuous_tick(a).unwrap();
        s.pause_execution(r.clone()).unwrap();
        let b = s.start_execution(r.clone()).unwrap().driver.unwrap();
        let error = application_error("APP-EXECUTION-WORKER", "Injected worker failure");
        s.driver_failed(a, error.clone()).unwrap();
        assert_eq!(
            s.snapshot(r.clone()).unwrap().execution_mode,
            ExecutionMode::Running
        );
        let before = s
            .inner
            .lock()
            .unwrap()
            .loaded
            .as_ref()
            .unwrap()
            .runtime
            .snapshot();
        let history = page(&s, &r);
        s.driver_failed(b, error.clone()).unwrap();
        assert_eq!(s.snapshot(r.clone()).unwrap_err(), error);
        assert_eq!(s.continuous_tick(b).unwrap(), TickOutcome::Stale);
        let slot = s.inner.lock().unwrap();
        let session = slot.loaded.as_ref().unwrap();
        assert_eq!(session.mode, Mode::Stopped);
        assert_eq!(session.runtime.snapshot(), before);
        assert_eq!(
            session.trace.metadata().latest_sequence.unwrap().get(),
            history.latest_sequence.unwrap().get()
        );
        drop(slot);
        s.unload(r).unwrap();
    }
    #[test]
    fn user_staging_during_run_and_pause_changes_only_the_next_cycle() {
        let s = RuntimeSessionService::default();
        let r = load(
            &s,
            "PROGRAM Main VAR_INPUT Start:BOOL; END_VAR VAR_OUTPUT Motor:BOOL; END_VAR Motor := Start; END_PROGRAM",
        );
        let lease = s.start_execution(r.clone()).unwrap().driver.unwrap();
        let stage = |value| {
            s.set_input(SetRuntimeInputRequest {
                contract_version: CONTRACT_VERSION,
                session_id: r.session_id,
                variable_id: RuntimeVariableId::new(1).unwrap(),
                value: RuntimeValue::Bool { value },
            })
            .unwrap()
        };
        let output = |snapshot: RuntimeSnapshotDto| {
            snapshot
                .variables
                .into_iter()
                .find(|v| v.name == "Motor")
                .unwrap()
                .value
        };
        assert_eq!(output(stage(true)), RuntimeValue::Bool { value: false });
        s.continuous_tick(lease).unwrap();
        assert_eq!(
            output(s.snapshot(r.clone()).unwrap()),
            RuntimeValue::Bool { value: true }
        );
        s.pause_execution(r.clone()).unwrap();
        assert_eq!(output(stage(false)), RuntimeValue::Bool { value: true });
        assert_eq!(s.continuous_tick(lease).unwrap(), TickOutcome::Stale);
        let next = s.start_execution(r.clone()).unwrap().driver.unwrap();
        s.continuous_tick(next).unwrap();
        assert_eq!(
            output(s.snapshot(r.clone()).unwrap()),
            RuntimeValue::Bool { value: false }
        );
        let events = page(&s, &r).entries;
        let staged = events
            .iter()
            .filter(|e| e.kind == TraceEventKind::InputStaged)
            .map(|e| e.completed_scans.get())
            .collect::<Vec<_>>();
        assert_eq!(staged, [0, 1]);
    }
    #[test]
    fn eviction_and_sequence_saturation_never_control_execution() {
        for saturated in [false, true] {
            let s = RuntimeSessionService::default();
            let r = load(&s, TANK);
            plant(&s, &r);
            s.inner.lock().unwrap().loaded.as_mut().unwrap().trace = if saturated {
                trace_core::TraceBuffer::near_exhaustion_for_test(NonZeroUsize::new(3).unwrap())
            } else {
                trace_core::TraceBuffer::new(NonZeroUsize::new(3).unwrap())
            };
            let a = s.start_execution(r.clone()).unwrap().driver.unwrap();
            for _ in 0..20 {
                assert_eq!(
                    s.continuous_tick(a).unwrap(),
                    TickOutcome::Observed { running: true }
                );
            }
            let observed = s.snapshot(r.clone()).unwrap();
            assert_eq!(observed.completed_scans.get(), 20);
            assert_eq!(observed.simulation_time_micros.as_micros(), 2_000_000);
            let ProcessSnapshotDto::TankFilling(tank) = observed.process.unwrap();
            assert_eq!(tank.level_permille, 800);
            let p = page(&s, &r);
            assert_eq!(p.saturated, saturated);
            assert_eq!(p.entries.len(), if saturated { 2 } else { 3 });
            assert_eq!(
                p.dropped_count.get(),
                if saturated { u64::MAX - 2 } else { 18 }
            );
        }
    }
}
