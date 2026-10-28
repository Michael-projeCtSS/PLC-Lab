//! Headless deterministic physical process models built from data-driven scenes.
//!
//! The scene runtime owns physical state only. It never reads PLC memory, never
//! paces itself on host time and never renders. Time advances only through
//! [`SceneRuntime::advance_to`] with world time supplied by the engine. Continuous
//! quantities are integrated exactly with fixed-point arithmetic between discrete
//! transitions (valve travel, contactor pick-up), so results are independent of how
//! the caller partitions time.

mod fixed;

use fixed::{Volume, crossing, integrate};
use lab_model::{
    Identifier,
    fault::StuckPosition,
    scene::{
        AnalogSignal, ComponentKind, ContactType, Coupling, LampColor, OutputStyle, PortKind,
        SceneDefinition,
    },
};
use simulation_core::{WorldTime, deterministic_hash};
use std::collections::{BTreeMap, BTreeSet};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Saturation limits of a 4-20 mA transmitter output (NAMUR NE 43 measurement range
/// 3.8 mA .. 20.5 mA). Failure signalling below/above these limits is not modeled
/// unless a fault forces it.
pub const NE43_MIN_UA: i64 = 3_800;
pub const NE43_MAX_UA: i64 = 20_500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneError {
    UnknownComponent(String),
    UnknownPort { component: String, port: String },
    NotOperable(String),
    InvalidPosition(String),
    TimeReversal,
}
impl std::fmt::Display for SceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownComponent(c) => write!(f, "Component '{c}' does not exist in this scene."),
            Self::UnknownPort { component, port } => {
                write!(f, "Component '{component}' has no port '{port}'.")
            }
            Self::NotOperable(c) => write!(f, "Component '{c}' cannot be operated this way."),
            Self::InvalidPosition(c) => write!(f, "Selector '{c}' has no such position."),
            Self::TimeReversal => f.write_str("The process cannot move backwards in world time."),
        }
    }
}
impl std::error::Error for SceneError {}

/// A physical quantity exposed for visualization, tracing, evidence and network registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Quantity {
    Bool(bool),
    Integer(i64),
}
impl Quantity {
    pub fn as_i64(self) -> i64 {
        match self {
            Self::Bool(b) => i64::from(b),
            Self::Integer(i) => i,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigitalField {
    pub conducting: bool,
    pub style: OutputStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalogUnit {
    Microamp,
    Millivolt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalogField {
    pub unit: AnalogUnit,
    pub value: i64,
    pub signal: AnalogSignal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProcessEventKind {
    OverflowStarted,
    OverflowEnded,
    DryRunStarted,
    DryRunEnded,
    HighHighOn,
    HighHighOff,
    LowLowOn,
    LowLowOff,
    MotorOverloadTripped,
    MotorOverloadReset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessEvent {
    pub time: WorldTime,
    pub component: Identifier,
    pub kind: ProcessEventKind,
}

/// Physical faults applied to one component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentFault {
    SensorStuck(bool),
    ContactType(ContactType),
    OutputStyle(OutputStyle),
    AnalogDrift(i32),
    AnalogStuck(u32),
    AnalogNoise(u32),
    LoopBreak,
    ValveStuck(StuckPosition),
    FeedbackMissing(String),
    MotorOverload,
    /// The drive is in local (keypad) control and ignores its run terminal.
    DriveLocalControl,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Faults {
    stuck: Option<bool>,
    contact: Option<ContactType>,
    style: Option<OutputStyle>,
    drift: i32,
    analog_stuck: Option<u32>,
    noise: u32,
    loop_break: bool,
    valve_stuck: Option<StuckPosition>,
    feedback_missing: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Motion {
    Closed,
    Opening { since: u64 },
    Open,
    Closing { since: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Contactor {
    Open,
    PickingUp { at: u64 },
    Closed { since: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    PushButton {
        contact: ContactType,
        pressed: bool,
    },
    Selector {
        positions: u8,
        position: u8,
    },
    Lamp {
        color: LampColor,
        lit: bool,
    },
    Starter {
        pickup: u64,
        run_up: u64,
        rated_ma: u32,
        coil: bool,
        contactor: Contactor,
        tripped: bool,
    },
    Valve {
        travel: u64,
        flow: u64,
        command: bool,
        motion: Motion,
    },
    Pump {
        flow: u64,
        command: bool,
        dry: bool,
    },
    Tank {
        capacity: u64,
        volume: Volume,
        spilled: u64,
        overflowing: bool,
        hh: Option<u64>,
        ll: Option<u64>,
        hh_on: bool,
        ll_on: bool,
    },
    LevelSwitch {
        tank: usize,
        level: u64,
        contact: ContactType,
        style: OutputStyle,
    },
    Transmitter {
        tank: usize,
        min: u64,
        max: u64,
        signal: AnalogSignal,
    },
    Presence {
        contact: ContactType,
        style: OutputStyle,
        present: bool,
    },
    Drive {
        ramp: u64,
        rated_ma: u32,
        run: bool,
        local: bool,
        started: Option<u64>,
    },
}

#[derive(Debug, Clone)]
pub struct SceneRuntime {
    now: u64,
    seed: u64,
    ids: Vec<Identifier>,
    labels: Vec<String>,
    kinds: Vec<&'static str>,
    sinks: Vec<Vec<&'static str>>,
    index: BTreeMap<Identifier, usize>,
    states: Vec<State>,
    faults: Vec<Faults>,
    fills: Vec<(usize, usize)>,
    drains: Vec<(usize, usize)>,
    pumps: Vec<(usize, usize)>,
    events: Vec<ProcessEvent>,
}

fn ms(value: u32) -> u64 {
    u64::from(value) * 1_000
}

impl SceneRuntime {
    /// Build the initial physical state. The scene must already be validated.
    pub fn new(scene: &SceneDefinition, seed: u64) -> Result<Self, SceneError> {
        let index: BTreeMap<Identifier, usize> = scene
            .components
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id.clone(), i))
            .collect();
        let find = |id: &Identifier| {
            index
                .get(id)
                .copied()
                .ok_or_else(|| SceneError::UnknownComponent(id.to_string()))
        };
        let mut states = Vec::with_capacity(scene.components.len());
        for c in &scene.components {
            states.push(match &c.kind {
                ComponentKind::PushButton { contact, .. } => State::PushButton {
                    contact: *contact,
                    pressed: false,
                },
                ComponentKind::SelectorSwitch { positions, initial } => State::Selector {
                    positions: u8::try_from(positions.len()).unwrap_or(u8::MAX),
                    position: *initial,
                },
                ComponentKind::PilotLamp { color } => State::Lamp {
                    color: *color,
                    lit: false,
                },
                ComponentKind::MotorStarter {
                    pickup_ms,
                    run_up_ms,
                    rated_current_ma,
                } => State::Starter {
                    pickup: ms(*pickup_ms),
                    run_up: ms(*run_up_ms).max(1),
                    rated_ma: *rated_current_ma,
                    coil: false,
                    contactor: Contactor::Open,
                    tripped: false,
                },
                ComponentKind::OnOffValve {
                    travel_ms,
                    flow_ml_per_s,
                } => State::Valve {
                    travel: ms(*travel_ms).max(2),
                    flow: *flow_ml_per_s,
                    command: false,
                    motion: Motion::Closed,
                },
                ComponentKind::Pump { flow_ml_per_s } => State::Pump {
                    flow: *flow_ml_per_s,
                    command: false,
                    dry: false,
                },
                ComponentKind::Tank {
                    capacity_ml,
                    initial_ml,
                    high_high_ml,
                    low_low_ml,
                } => State::Tank {
                    capacity: *capacity_ml,
                    volume: Volume {
                        ml: (*initial_ml).min(*capacity_ml),
                        remainder: 0,
                    },
                    spilled: 0,
                    overflowing: false,
                    hh: *high_high_ml,
                    ll: *low_low_ml,
                    hh_on: high_high_ml.is_some_and(|h| *initial_ml >= h),
                    ll_on: low_low_ml.is_some_and(|l| *initial_ml <= l),
                },
                ComponentKind::LevelSwitch {
                    tank,
                    switch_level_ml,
                    contact,
                    output,
                } => State::LevelSwitch {
                    tank: find(tank)?,
                    level: *switch_level_ml,
                    contact: *contact,
                    style: *output,
                },
                ComponentKind::LevelTransmitter {
                    tank,
                    range_min_ml,
                    range_max_ml,
                    signal,
                } => State::Transmitter {
                    tank: find(tank)?,
                    min: *range_min_ml,
                    max: (*range_max_ml).max(range_min_ml + 1),
                    signal: *signal,
                },
                ComponentKind::PresenceSensor { output, contact } => State::Presence {
                    contact: *contact,
                    style: *output,
                    present: false,
                },
                ComponentKind::VariableFrequencyDrive {
                    ramp_ms,
                    rated_current_ma,
                } => State::Drive {
                    ramp: ms(*ramp_ms).max(1),
                    rated_ma: *rated_current_ma,
                    run: false,
                    local: false,
                    started: None,
                },
            });
        }
        let mut fills = Vec::new();
        let mut drains = Vec::new();
        let mut pumps = Vec::new();
        for coupling in &scene.couplings {
            match coupling {
                Coupling::ValveFillsTank { valve, tank } => fills.push((find(valve)?, find(tank)?)),
                Coupling::ValveDrainsTank { valve, tank } => {
                    drains.push((find(valve)?, find(tank)?))
                }
                Coupling::PumpDrainsTank { pump, tank } => pumps.push((find(pump)?, find(tank)?)),
            }
        }
        Ok(Self {
            now: 0,
            seed,
            ids: scene.components.iter().map(|c| c.id.clone()).collect(),
            labels: scene.components.iter().map(|c| c.label.clone()).collect(),
            kinds: scene
                .components
                .iter()
                .map(|c| c.kind.type_name())
                .collect(),
            sinks: scene
                .components
                .iter()
                .map(|c| {
                    c.kind
                        .ports()
                        .into_iter()
                        .filter(|(_, kind)| *kind == PortKind::DigitalSink)
                        .map(|(name, _)| name)
                        .collect()
                })
                .collect(),
            index,
            faults: vec![Faults::default(); states.len()],
            states,
            fills,
            drains,
            pumps,
            events: Vec::new(),
        })
    }

    pub fn now(&self) -> WorldTime {
        WorldTime::from_micros(self.now)
    }
    pub fn component_count(&self) -> usize {
        self.ids.len()
    }
    pub fn component_id(&self, index: usize) -> Option<&Identifier> {
        self.ids.get(index)
    }
    pub fn component_label(&self, index: usize) -> Option<&str> {
        self.labels.get(index).map(String::as_str)
    }
    pub fn component_kind(&self, index: usize) -> Option<&'static str> {
        self.kinds.get(index).copied()
    }
    /// Whether `port` of the component is a digital sink (coil, solenoid, run terminal).
    pub fn is_digital_sink(&self, index: usize, port: &str) -> bool {
        self.sinks
            .get(index)
            .is_some_and(|ports| ports.contains(&port))
    }
    pub fn index_of(&self, id: &Identifier) -> Result<usize, SceneError> {
        self.index
            .get(id)
            .copied()
            .ok_or_else(|| SceneError::UnknownComponent(id.to_string()))
    }
    pub fn index_of_str(&self, id: &str) -> Result<usize, SceneError> {
        self.ids
            .iter()
            .position(|i| i.as_str() == id)
            .ok_or_else(|| SceneError::UnknownComponent(id.to_string()))
    }
    pub fn drain_events(&mut self) -> Vec<ProcessEvent> {
        std::mem::take(&mut self.events)
    }

    /// Advance physics to world time `target`, applying every internal transition in order.
    pub fn advance_to(&mut self, target: WorldTime) -> Result<(), SceneError> {
        let target = target.as_micros();
        if target < self.now {
            return Err(SceneError::TimeReversal);
        }
        self.apply_transitions();
        while self.now < target {
            let next = self
                .next_transition()
                .filter(|t| *t <= target)
                .unwrap_or(target);
            self.integrate(next - self.now);
            self.now = next;
            self.apply_transitions();
        }
        Ok(())
    }

    fn next_transition(&self) -> Option<u64> {
        let mut next: Option<u64> = None;
        let mut consider = |t: u64| {
            if t > self.now {
                next = Some(next.map_or(t, |n| n.min(t)));
            }
        };
        for state in &self.states {
            match *state {
                State::Valve {
                    travel,
                    motion: Motion::Opening { since } | Motion::Closing { since },
                    ..
                } => {
                    consider(since + travel / 2);
                    consider(since + travel);
                }
                State::Starter {
                    contactor: Contactor::PickingUp { at },
                    ..
                } => consider(at),
                _ => {}
            }
        }
        next
    }

    fn apply_transitions(&mut self) {
        let now = self.now;
        for state in &mut self.states {
            match state {
                State::Valve { travel, motion, .. } => match *motion {
                    Motion::Opening { since } if now >= since + *travel => *motion = Motion::Open,
                    Motion::Closing { since } if now >= since + *travel => *motion = Motion::Closed,
                    _ => {}
                },
                State::Starter { contactor, .. } => {
                    if let Contactor::PickingUp { at } = *contactor
                        && now >= at
                    {
                        *contactor = Contactor::Closed { since: at };
                    }
                }
                _ => {}
            }
        }
    }

    fn valve_flowing(&self, valve: usize) -> u64 {
        let State::Valve {
            travel,
            flow,
            motion,
            ..
        } = self.states[valve]
        else {
            return 0;
        };
        let open = match self.faults[valve].valve_stuck {
            Some(StuckPosition::Open) => true,
            Some(StuckPosition::Closed) => false,
            None => match motion {
                Motion::Closed => false,
                Motion::Open => true,
                Motion::Opening { since } => self.now >= since + travel / 2,
                Motion::Closing { since } => self.now < since + travel / 2,
            },
        };
        if open { flow } else { 0 }
    }

    fn pump_flow(&self, pump: usize) -> u64 {
        match self.states[pump] {
            State::Pump {
                flow,
                command: true,
                ..
            } => flow,
            _ => 0,
        }
    }

    fn tank_rate(&self, tank: usize) -> i64 {
        let gain: u64 = self
            .fills
            .iter()
            .filter(|(_, t)| *t == tank)
            .map(|(v, _)| self.valve_flowing(*v))
            .sum();
        let loss: u64 = self
            .drains
            .iter()
            .filter(|(_, t)| *t == tank)
            .map(|(v, _)| self.valve_flowing(*v))
            .chain(
                self.pumps
                    .iter()
                    .filter(|(_, t)| *t == tank)
                    .map(|(p, _)| self.pump_flow(*p)),
            )
            .sum();
        i64::try_from(gain).unwrap_or(i64::MAX) - i64::try_from(loss).unwrap_or(i64::MAX)
    }

    fn integrate(&mut self, dt: u64) {
        let start = self.now;
        for tank in 0..self.states.len() {
            if !matches!(self.states[tank], State::Tank { .. }) {
                continue;
            }
            let rate = self.tank_rate(tank);
            let id = self.ids[tank].clone();
            let State::Tank {
                capacity,
                volume,
                spilled,
                overflowing,
                hh,
                ll,
                hh_on,
                ll_on,
            } = &mut self.states[tank]
            else {
                continue;
            };
            let before = *volume;
            let result = integrate(volume, rate, dt, *capacity);
            *spilled = spilled.saturating_add(result.spilled_ml);
            let mut events = Vec::new();
            let full_now = volume.ml >= *capacity && rate > 0;
            if full_now && !*overflowing {
                events.push((
                    result.full_at.unwrap_or(0),
                    ProcessEventKind::OverflowStarted,
                ));
            }
            if !full_now && *overflowing {
                events.push((0, ProcessEventKind::OverflowEnded));
            }
            *overflowing = full_now;
            for (limit, active, rising, on, off) in [
                (
                    *hh,
                    hh_on,
                    true,
                    ProcessEventKind::HighHighOn,
                    ProcessEventKind::HighHighOff,
                ),
                (
                    *ll,
                    ll_on,
                    false,
                    ProcessEventKind::LowLowOn,
                    ProcessEventKind::LowLowOff,
                ),
            ] {
                let Some(limit) = limit else { continue };
                let now_active = if rising {
                    volume.ml >= limit
                } else {
                    volume.ml <= limit
                };
                if now_active != *active {
                    let at = crossing(before, rate, limit, dt).unwrap_or(dt);
                    events.push((at, if now_active { on } else { off }));
                    *active = now_active;
                }
            }
            let empty_at = result.empty_at;
            let empty = volume.ml == 0 && volume.remainder == 0;
            for (offset, kind) in events {
                self.events.push(ProcessEvent {
                    time: WorldTime::from_micros(start + offset),
                    component: id.clone(),
                    kind,
                });
            }
            for p in 0..self.pumps.len() {
                let (pump, pump_tank) = self.pumps[p];
                if pump_tank != tank {
                    continue;
                }
                let running = self.pump_flow(pump) > 0;
                let pump_id = self.ids[pump].clone();
                if let State::Pump { dry, .. } = &mut self.states[pump] {
                    let now_dry = running && empty;
                    if now_dry != *dry {
                        *dry = now_dry;
                        let at = if now_dry { empty_at.unwrap_or(dt) } else { 0 };
                        self.events.push(ProcessEvent {
                            time: WorldTime::from_micros(start + at),
                            component: pump_id,
                            kind: if now_dry {
                                ProcessEventKind::DryRunStarted
                            } else {
                                ProcessEventKind::DryRunEnded
                            },
                        });
                    }
                }
            }
        }
    }

    fn component_mut(&mut self, id: &str) -> Result<usize, SceneError> {
        self.index_of_str(id)
    }

    /// Energize or de-energize a coil, solenoid or lamp at the current world time.
    pub fn set_coil(
        &mut self,
        component: usize,
        port: &str,
        energized: bool,
    ) -> Result<(), SceneError> {
        let now = self.now;
        let id = self
            .ids
            .get(component)
            .cloned()
            .ok_or_else(|| SceneError::UnknownComponent(component.to_string()))?;
        let stuck = self.faults[component].valve_stuck.is_some();
        match (&mut self.states[component], port) {
            (State::Lamp { lit, .. }, "coil") => *lit = energized,
            (
                State::Starter {
                    pickup,
                    coil,
                    contactor,
                    tripped,
                    ..
                },
                "coil",
            ) => {
                *coil = energized;
                if !energized {
                    *contactor = Contactor::Open;
                } else if !*tripped && matches!(contactor, Contactor::Open) {
                    *contactor = Contactor::PickingUp { at: now + *pickup };
                }
            }
            (
                State::Valve {
                    travel,
                    command,
                    motion,
                    ..
                },
                "solenoid",
            ) => {
                *command = energized;
                if !stuck {
                    *motion = retarget(*motion, energized, now, *travel);
                }
            }
            (State::Pump { command, .. }, "motor") => *command = energized,
            (
                State::Drive {
                    run,
                    local,
                    started,
                    ..
                },
                "run",
            ) => {
                *run = energized;
                if !energized || *local {
                    *started = None;
                } else if started.is_none() {
                    *started = Some(now);
                }
            }
            _ => {
                return Err(SceneError::UnknownPort {
                    component: id.to_string(),
                    port: port.into(),
                });
            }
        }
        self.apply_transitions();
        Ok(())
    }

    pub fn press(&mut self, component: &str, pressed: bool) -> Result<(), SceneError> {
        let i = self.component_mut(component)?;
        match &mut self.states[i] {
            State::PushButton { pressed: p, .. } => {
                *p = pressed;
                Ok(())
            }
            _ => Err(SceneError::NotOperable(component.into())),
        }
    }
    pub fn set_selector(&mut self, component: &str, position: u8) -> Result<(), SceneError> {
        let i = self.component_mut(component)?;
        match &mut self.states[i] {
            State::Selector {
                positions,
                position: p,
            } => {
                if position >= *positions {
                    return Err(SceneError::InvalidPosition(component.into()));
                }
                *p = position;
                Ok(())
            }
            _ => Err(SceneError::NotOperable(component.into())),
        }
    }
    pub fn set_presence(&mut self, component: &str, present: bool) -> Result<(), SceneError> {
        let i = self.component_mut(component)?;
        match &mut self.states[i] {
            State::Presence { present: p, .. } => {
                *p = present;
                Ok(())
            }
            _ => Err(SceneError::NotOperable(component.into())),
        }
    }

    pub fn apply_fault(
        &mut self,
        component: usize,
        fault: ComponentFault,
    ) -> Result<(), SceneError> {
        let id = self
            .ids
            .get(component)
            .cloned()
            .ok_or_else(|| SceneError::UnknownComponent(component.to_string()))?;
        let f = &mut self.faults[component];
        match fault {
            ComponentFault::SensorStuck(v) => f.stuck = Some(v),
            ComponentFault::ContactType(c) => f.contact = Some(c),
            ComponentFault::OutputStyle(s) => f.style = Some(s),
            ComponentFault::AnalogDrift(o) => f.drift = o,
            ComponentFault::AnalogStuck(v) => f.analog_stuck = Some(v),
            ComponentFault::AnalogNoise(a) => f.noise = a,
            ComponentFault::LoopBreak => f.loop_break = true,
            ComponentFault::ValveStuck(p) => f.valve_stuck = Some(p),
            ComponentFault::FeedbackMissing(port) => {
                f.feedback_missing.insert(port);
            }
            ComponentFault::DriveLocalControl => {
                if let State::Drive { local, started, .. } = &mut self.states[component] {
                    *local = true;
                    *started = None;
                } else {
                    return Err(SceneError::NotOperable(id.to_string()));
                }
            }
            ComponentFault::MotorOverload => {
                if let State::Starter {
                    tripped, contactor, ..
                } = &mut self.states[component]
                {
                    if !*tripped {
                        *tripped = true;
                        *contactor = Contactor::Open;
                        self.events.push(ProcessEvent {
                            time: WorldTime::from_micros(self.now),
                            component: id,
                            kind: ProcessEventKind::MotorOverloadTripped,
                        });
                    }
                } else {
                    return Err(SceneError::NotOperable(id.to_string()));
                }
            }
        }
        Ok(())
    }

    /// Remove a fault. Repairs restore the component's designed behavior.
    pub fn clear_fault(
        &mut self,
        component: usize,
        fault: &ComponentFault,
    ) -> Result<(), SceneError> {
        let now = self.now;
        let id = self
            .ids
            .get(component)
            .cloned()
            .ok_or_else(|| SceneError::UnknownComponent(component.to_string()))?;
        let f = &mut self.faults[component];
        match fault {
            ComponentFault::SensorStuck(_) => f.stuck = None,
            ComponentFault::ContactType(_) => f.contact = None,
            ComponentFault::OutputStyle(_) => f.style = None,
            ComponentFault::AnalogDrift(_) => f.drift = 0,
            ComponentFault::AnalogStuck(_) => f.analog_stuck = None,
            ComponentFault::AnalogNoise(_) => f.noise = 0,
            ComponentFault::LoopBreak => f.loop_break = false,
            ComponentFault::ValveStuck(_) => {
                let was = f.valve_stuck.take();
                if let (
                    Some(was),
                    State::Valve {
                        travel,
                        command,
                        motion,
                        ..
                    },
                ) = (was, &mut self.states[component])
                {
                    let from = if was == StuckPosition::Open {
                        Motion::Open
                    } else {
                        Motion::Closed
                    };
                    *motion = retarget(from, *command, now, *travel);
                }
            }
            ComponentFault::FeedbackMissing(port) => {
                f.feedback_missing.remove(port);
            }
            ComponentFault::DriveLocalControl => {
                if let State::Drive {
                    run,
                    local,
                    started,
                    ..
                } = &mut self.states[component]
                {
                    *local = false;
                    if *run && started.is_none() {
                        *started = Some(now);
                    }
                }
            }
            ComponentFault::MotorOverload => {
                if let State::Starter {
                    tripped,
                    coil,
                    contactor,
                    pickup,
                    ..
                } = &mut self.states[component]
                    && *tripped
                {
                    *tripped = false;
                    if *coil {
                        *contactor = Contactor::PickingUp { at: now + *pickup };
                    }
                    self.events.push(ProcessEvent {
                        time: WorldTime::from_micros(now),
                        component: id,
                        kind: ProcessEventKind::MotorOverloadReset,
                    });
                }
            }
        }
        Ok(())
    }

    fn tank_volume(&self, tank: usize) -> u64 {
        match self.states.get(tank) {
            Some(State::Tank { volume, .. }) => volume.ml,
            _ => 0,
        }
    }

    /// Field signal of a contact or switching sensor at the current world time.
    pub fn digital_output(&self, component: usize, port: &str) -> Result<DigitalField, SceneError> {
        let id = || {
            self.ids
                .get(component)
                .map(ToString::to_string)
                .unwrap_or_default()
        };
        let unknown = || SceneError::UnknownPort {
            component: id(),
            port: port.into(),
        };
        let state = self.states.get(component).ok_or_else(unknown)?;
        let f = &self.faults[component];
        let contact = |designed: ContactType, actuated: bool| {
            let installed = f.contact.unwrap_or(designed);
            actuated ^ (installed == ContactType::NormallyClosed)
        };
        let (conducting, style) = match (state, port) {
            (
                State::PushButton {
                    contact: c,
                    pressed,
                },
                "contact",
            ) => (contact(*c, *pressed), OutputStyle::DryContact),
            (
                State::Selector {
                    position,
                    positions,
                },
                p,
            ) => {
                let wanted = p
                    .strip_prefix("position")
                    .and_then(|n| n.parse::<u8>().ok())
                    .filter(|n| n < positions)
                    .ok_or_else(unknown)?;
                (*position == wanted, OutputStyle::DryContact)
            }
            (State::Starter { contactor, .. }, "aux") => (
                matches!(contactor, Contactor::Closed { .. }),
                OutputStyle::DryContact,
            ),
            (State::Starter { tripped, .. }, "overload") => (!*tripped, OutputStyle::DryContact),
            (State::Valve { motion, .. }, "openSwitch") => {
                let open = match f.valve_stuck {
                    Some(p) => p == StuckPosition::Open,
                    None => *motion == Motion::Open,
                };
                (open, OutputStyle::DryContact)
            }
            (State::Valve { motion, .. }, "closedSwitch") => {
                let closed = match f.valve_stuck {
                    Some(p) => p == StuckPosition::Closed,
                    None => *motion == Motion::Closed,
                };
                (closed, OutputStyle::DryContact)
            }
            (
                State::LevelSwitch {
                    tank,
                    level,
                    contact: c,
                    style,
                },
                "contact",
            ) => {
                let wet = self.tank_volume(*tank) >= *level;
                (contact(*c, wet), *style)
            }
            (
                State::Presence {
                    contact: c,
                    style,
                    present,
                },
                "output",
            ) => (contact(*c, *present), *style),
            (State::Drive { .. }, "ready") => (true, OutputStyle::DryContact),
            (State::Drive { started, .. }, "running") => {
                (started.is_some(), OutputStyle::DryContact)
            }
            (State::Drive { local, .. }, "remote") => (!*local, OutputStyle::DryContact),
            _ => return Err(unknown()),
        };
        let conducting = if f.feedback_missing.contains(port) {
            false
        } else {
            f.stuck.unwrap_or(conducting)
        };
        Ok(DigitalField {
            conducting,
            style: f.style.unwrap_or(style),
        })
    }

    /// Analog field signal at the current world time.
    pub fn analog_output(&self, component: usize, port: &str) -> Result<AnalogField, SceneError> {
        let id = self
            .ids
            .get(component)
            .map(ToString::to_string)
            .unwrap_or_default();
        let Some(State::Transmitter {
            tank,
            min,
            max,
            signal,
        }) = self.states.get(component)
        else {
            return Err(SceneError::UnknownPort {
                component: id,
                port: port.into(),
            });
        };
        if port != "signal" {
            return Err(SceneError::UnknownPort {
                component: id,
                port: port.into(),
            });
        }
        let level = i128::from(self.tank_volume(*tank));
        let fraction_ppm = ((level - i128::from(*min)) * 1_000_000) / i128::from(max - min);
        let (unit, base) = match signal {
            AnalogSignal::Current4To20mA => {
                let v = 4_000 + fraction_ppm * 16_000 / 1_000_000;
                (
                    AnalogUnit::Microamp,
                    (v as i64).clamp(NE43_MIN_UA, NE43_MAX_UA),
                )
            }
            AnalogSignal::Current0To20mA => {
                let v = fraction_ppm * 20_000 / 1_000_000;
                (AnalogUnit::Microamp, (v as i64).clamp(0, NE43_MAX_UA))
            }
            AnalogSignal::Voltage0To10V => {
                let v = fraction_ppm * 10_000 / 1_000_000;
                (AnalogUnit::Millivolt, (v as i64).clamp(0, 10_500))
            }
        };
        let f = &self.faults[component];
        let value = if f.loop_break {
            0
        } else if let Some(stuck) = f.analog_stuck {
            i64::from(stuck)
        } else {
            let noise = if f.noise == 0 {
                0
            } else {
                let span = u64::from(f.noise) * 2 + 1;
                let r = deterministic_hash(self.seed, component as u64, self.now / 1_000) % span;
                i64::try_from(r).unwrap_or(0) - i64::from(f.noise)
            };
            (base + i64::from(f.drift) + noise).max(0)
        };
        Ok(AnalogField {
            unit,
            value,
            signal: *signal,
        })
    }

    /// Named physical quantities of one component, in a fixed order.
    pub fn quantities(&self, component: usize) -> Vec<(&'static str, Quantity)> {
        let now = self.now;
        let Some(state) = self.states.get(component) else {
            return Vec::new();
        };
        let f = &self.faults[component];
        match state {
            State::PushButton { pressed, .. } => vec![("pressed", Quantity::Bool(*pressed))],
            State::Selector { position, .. } => {
                vec![("position", Quantity::Integer(i64::from(*position)))]
            }
            State::Lamp { lit, color } => vec![
                ("lit", Quantity::Bool(*lit)),
                ("color", Quantity::Integer(*color as i64)),
            ],
            State::Starter {
                coil,
                contactor,
                tripped,
                run_up,
                rated_ma,
                ..
            } => {
                let (running, speed) = match contactor {
                    Contactor::Closed { since } => {
                        (true, ((now - since) * 1000 / run_up).min(1000))
                    }
                    _ => (false, 0),
                };
                vec![
                    ("command", Quantity::Bool(*coil)),
                    ("running", Quantity::Bool(running)),
                    ("speedPermille", Quantity::Integer(speed as i64)),
                    (
                        "currentMa",
                        Quantity::Integer(if running { i64::from(*rated_ma) } else { 0 }),
                    ),
                    ("tripped", Quantity::Bool(*tripped)),
                ]
            }
            State::Valve {
                travel,
                command,
                motion,
                ..
            } => {
                let position = match f.valve_stuck {
                    Some(StuckPosition::Open) => 1000,
                    Some(StuckPosition::Closed) => 0,
                    None => match motion {
                        Motion::Closed => 0,
                        Motion::Open => 1000,
                        Motion::Opening { since } => ((now - since) * 1000 / travel).min(1000),
                        Motion::Closing { since } => {
                            1000 - ((now - since) * 1000 / travel).min(1000)
                        }
                    },
                };
                vec![
                    ("command", Quantity::Bool(*command)),
                    ("positionPermille", Quantity::Integer(position as i64)),
                    ("flowing", Quantity::Bool(self.valve_flowing(component) > 0)),
                    ("stuck", Quantity::Bool(f.valve_stuck.is_some())),
                ]
            }
            State::Pump { command, dry, flow } => vec![
                ("command", Quantity::Bool(*command)),
                ("running", Quantity::Bool(*command)),
                ("dry", Quantity::Bool(*dry)),
                (
                    "flowMlPerS",
                    Quantity::Integer(if *command && !*dry { *flow as i64 } else { 0 }),
                ),
            ],
            State::Tank {
                capacity,
                volume,
                spilled,
                overflowing,
                hh_on,
                ll_on,
                ..
            } => vec![
                ("levelMl", Quantity::Integer(volume.ml as i64)),
                (
                    "levelPermille",
                    Quantity::Integer(
                        (u128::from(volume.ml) * 1000 / u128::from((*capacity).max(1))) as i64,
                    ),
                ),
                ("capacityMl", Quantity::Integer(*capacity as i64)),
                ("spilledMl", Quantity::Integer(*spilled as i64)),
                ("overflowing", Quantity::Bool(*overflowing)),
                ("highHigh", Quantity::Bool(*hh_on)),
                ("lowLow", Quantity::Bool(*ll_on)),
                (
                    "netFlowMlPerS",
                    Quantity::Integer(self.tank_rate(component)),
                ),
            ],
            State::LevelSwitch { tank, level, .. } => {
                let conducting = self
                    .digital_output(component, "contact")
                    .map(|d| d.conducting)
                    .unwrap_or(false);
                vec![
                    ("wet", Quantity::Bool(self.tank_volume(*tank) >= *level)),
                    ("conducting", Quantity::Bool(conducting)),
                ]
            }
            State::Transmitter { .. } => {
                let signal = self
                    .analog_output(component, "signal")
                    .map(|a| a.value)
                    .unwrap_or(0);
                vec![("signal", Quantity::Integer(signal))]
            }
            State::Presence { present, .. } => {
                let conducting = self
                    .digital_output(component, "output")
                    .map(|d| d.conducting)
                    .unwrap_or(false);
                vec![
                    ("present", Quantity::Bool(*present)),
                    ("conducting", Quantity::Bool(conducting)),
                ]
            }
            State::Drive {
                ramp,
                rated_ma,
                run,
                local,
                started,
            } => {
                let speed = started.map_or(0, |since| ((now - since) * 1000 / ramp).min(1000));
                vec![
                    ("command", Quantity::Bool(*run)),
                    ("running", Quantity::Bool(started.is_some())),
                    ("speedPermille", Quantity::Integer(speed as i64)),
                    (
                        "currentMa",
                        Quantity::Integer(if started.is_some() {
                            i64::from(*rated_ma) * (200 + speed as i64 * 8 / 10) / 1000
                        } else {
                            0
                        }),
                    ),
                    ("remote", Quantity::Bool(!*local)),
                ]
            }
        }
    }

    pub fn quantity(&self, component: usize, name: &str) -> Option<Quantity> {
        self.quantities(component)
            .into_iter()
            .find(|(n, _)| *n == name)
            .map(|(_, q)| q)
    }

    /// True when the component is operated by a person (buttons, selectors, presence targets).
    pub fn is_operable(&self, component: usize) -> bool {
        matches!(
            self.states.get(component),
            Some(State::PushButton { .. } | State::Selector { .. } | State::Presence { .. })
        )
    }
}

fn retarget(motion: Motion, open: bool, now: u64, travel: u64) -> Motion {
    match (motion, open) {
        (Motion::Closed, true) => Motion::Opening { since: now },
        (Motion::Open, false) => Motion::Closing { since: now },
        (Motion::Closing { since }, true) => {
            let elapsed = (now - since).min(travel);
            Motion::Opening {
                since: now.saturating_sub(travel - elapsed),
            }
        }
        (Motion::Opening { since }, false) => {
            let elapsed = (now - since).min(travel);
            Motion::Closing {
                since: now.saturating_sub(travel - elapsed),
            }
        }
        (m, _) => m,
    }
}
