//! Plant alarms (ADR-0037): generic alarm definitions derived from the plant model (scene
//! components, configured I/O modules and Modbus connections) and the conditions behind
//! them, evaluated against the authoritative world state at the end of every cycle.
//!
//! Conditions only read state the engine already owns: process quantities, channel
//! readings, module status and connection status. Messages name observable symptoms, never
//! a hidden cause.

use crate::session::LabSession;
use alarm_core::{AlarmCategory, AlarmDefinition, AlarmPriority};
use hardware_model::{ModuleStatus, Quality};
use lab_model::{
    hardware::{ChannelRef, ModuleClass, SlotRef},
    scene::ComponentKind,
};
use network_core::ScanStatus;
use process_simulation::Quantity;

/// What must hold for an alarm to be active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AlarmCondition {
    /// A boolean quantity of a component equals `when`.
    Quantity {
        component: usize,
        name: &'static str,
        when: bool,
    },
    /// A valve's position differs from its command.
    Discrepancy { component: usize },
    /// An analog input channel reports an invalid signal.
    SignalInvalid { channel: ChannelRef },
    /// A module reports a missing field (load) supply.
    FieldPower { slot: SlotRef },
    /// A module does not answer the CPU as configured.
    ModuleFault { slot: SlotRef },
    /// A Modbus connection fails.
    Connection { scan: usize },
    /// The active sequence step timed out (ADR-0043).
    SequenceTimeout,
    /// The sequence monitor latched a deviation.
    SequenceDeviation,
    /// A sequence interlock is active where it applies.
    SequenceInterlock { index: usize },
}

pub(crate) struct PlantAlarm {
    pub definition: AlarmDefinition,
    pub condition: AlarmCondition,
}

struct Spec {
    code: &'static str,
    slug: &'static str,
    priority: AlarmPriority,
    text: &'static str,
}

const fn spec(
    code: &'static str,
    slug: &'static str,
    priority: AlarmPriority,
    text: &'static str,
) -> Spec {
    Spec {
        code,
        slug,
        priority,
        text,
    }
}

fn plant_alarm(
    spec: &Spec,
    source: &str,
    label: &str,
    category: AlarmCategory,
    on_delay_us: u64,
    condition: AlarmCondition,
) -> PlantAlarm {
    PlantAlarm {
        definition: AlarmDefinition {
            id: format!("{source}.{}", spec.slug),
            code: spec.code.into(),
            source: source.into(),
            source_label: label.into(),
            category,
            priority: spec.priority,
            message: format!("{label}: {}", spec.text),
            on_delay_us,
        },
        condition,
    }
}

const OVERFLOW: Spec = spec("OVERFLOW", "overflow", AlarmPriority::Critical, "overflow");
const HIGH_LEVEL: Spec = spec(
    "HIGH_LEVEL",
    "high-level",
    AlarmPriority::Alarm,
    "level high-high",
);
const LOW_LEVEL: Spec = spec(
    "LOW_LEVEL",
    "low-level",
    AlarmPriority::Warning,
    "level low-low",
);
const DRY_RUN: Spec = spec("DRY_RUN", "dry-run", AlarmPriority::Alarm, "running dry");
const MOTOR_FAULT: Spec = spec(
    "MOTOR_FAULT",
    "motor-fault",
    AlarmPriority::Alarm,
    "motor overload tripped",
);
const DRIVE_FAULT: Spec = spec(
    "DRIVE_FAULT",
    "drive-fault",
    AlarmPriority::Alarm,
    "drive fault",
);
const DRIVE_NOT_READY: Spec = spec(
    "DRIVE_NOT_READY",
    "drive-not-ready",
    AlarmPriority::Warning,
    "drive not ready",
);
const PROCESS_TIMEOUT: Spec = spec(
    "PROCESS_TIMEOUT",
    "process-timeout",
    AlarmPriority::Warning,
    "commanded position not reached in time",
);
const SENSOR_INVALID: Spec = spec(
    "SENSOR_INVALID",
    "sensor-invalid",
    AlarmPriority::Alarm,
    "signal invalid",
);
const FIELD_POWER_LOST: Spec = spec(
    "FIELD_POWER_LOST",
    "field-power-lost",
    AlarmPriority::Alarm,
    "field supply missing",
);
const MODULE_FAULT: Spec = spec(
    "MODULE_FAULT",
    "module-fault",
    AlarmPriority::Alarm,
    "module fault (see the diagnostic buffer)",
);
const COMMUNICATION_LOST: Spec = spec(
    "COMMUNICATION_LOST",
    "communication-lost",
    AlarmPriority::Alarm,
    "communication lost",
);

const SEQUENCE_TIMEOUT: Spec = spec(
    "SEQUENCE_TIMEOUT",
    "step-timeout",
    AlarmPriority::Alarm,
    "step timeout",
);
const SEQUENCE_DEVIATION: Spec = spec(
    "SEQUENCE_DEVIATION",
    "deviation",
    AlarmPriority::Alarm,
    "sequence deviation",
);
const SEQUENCE_INTERLOCK: Spec = spec(
    "SEQUENCE_INTERLOCK",
    "interlock",
    AlarmPriority::Warning,
    "interlock active",
);

/// A valve is given twice its travel time, and at least one second, to reach a position.
fn discrepancy_delay_us(travel_ms: u32) -> u64 {
    (u64::from(travel_ms) * 2).max(1000) * 1000
}

/// The alarms of the plant as the session currently models it.
pub(crate) fn plant_alarms(session: &LabSession) -> Vec<PlantAlarm> {
    let mut out = Vec::new();
    let scene = &session.scene;
    for index in 0..scene.component_count() {
        let Some(id) = scene.component_id(index) else {
            continue;
        };
        let (id, label) = (
            id.as_str(),
            scene.component_label(index).unwrap_or(id.as_str()),
        );
        let names: Vec<&'static str> = scene.quantities(index).into_iter().map(|q| q.0).collect();
        let has = |name: &str| names.contains(&name);
        let mut add = |spec: &Spec, delay: u64, condition: AlarmCondition| {
            let alarm = plant_alarm(spec, id, label, AlarmCategory::Process, delay, condition);
            out.push(alarm);
        };
        let quantity = |name: &'static str, when: bool| AlarmCondition::Quantity {
            component: index,
            name,
            when,
        };
        for (name, spec) in [
            ("overflowing", &OVERFLOW),
            ("highHigh", &HIGH_LEVEL),
            ("lowLow", &LOW_LEVEL),
            ("dry", &DRY_RUN),
        ] {
            if has(name) {
                add(spec, 0, quantity(name, true));
            }
        }
        if has("ready") {
            add(&DRIVE_FAULT, 0, quantity("tripped", true));
            add(&DRIVE_NOT_READY, 0, quantity("ready", false));
        } else if has("tripped") {
            add(&MOTOR_FAULT, 0, quantity("tripped", true));
        }
        let travel = session.scene_def.components.iter().find_map(|c| {
            match (c.id.as_str() == id, &c.kind) {
                (true, ComponentKind::OnOffValve { travel_ms, .. }) => Some(*travel_ms),
                _ => None,
            }
        });
        if let Some(travel_ms) = travel {
            let delay = discrepancy_delay_us(travel_ms);
            add(
                &PROCESS_TIMEOUT,
                delay,
                AlarmCondition::Discrepancy { component: index },
            );
        }
    }
    let configuration = session.station.configuration();
    for module in &configuration.modules {
        let slot = module.position;
        let source = module.id.as_str();
        let category = AlarmCategory::Hardware;
        if !matches!(
            module.module.class(),
            ModuleClass::Cpu | ModuleClass::PowerSupply
        ) {
            let id = format!("io.{source}");
            let condition = AlarmCondition::FieldPower { slot };
            out.push(plant_alarm(
                &FIELD_POWER_LOST,
                &id,
                source,
                category,
                0,
                condition,
            ));
            let condition = AlarmCondition::ModuleFault { slot };
            out.push(plant_alarm(
                &MODULE_FAULT,
                &id,
                source,
                category,
                0,
                condition,
            ));
        }
    }
    for mapping in &configuration.io_map {
        let channel = mapping.channel;
        let analog_input = configuration
            .modules
            .iter()
            .find(|m| m.position == channel.slot_ref())
            .filter(|m| matches!(m.module.class(), ModuleClass::AnalogInput { .. }));
        if let Some(module) = analog_input {
            let source = format!("io.{}", module.channel_id(channel.channel));
            let label = format!("{} ({})", mapping.tag, session.channel_address(channel));
            let condition = AlarmCondition::SignalInvalid { channel };
            let category = AlarmCategory::Hardware;
            out.push(plant_alarm(
                &SENSOR_INVALID,
                &source,
                &label,
                category,
                0,
                condition,
            ));
        }
    }
    if let Some(monitor) = session.sequence() {
        let sequence = monitor.definition();
        let source = format!("sequence.{}", sequence.id);
        let category = AlarmCategory::Process;
        out.push(plant_alarm(
            &SEQUENCE_TIMEOUT,
            &source,
            &sequence.name,
            category,
            0,
            AlarmCondition::SequenceTimeout,
        ));
        out.push(plant_alarm(
            &SEQUENCE_DEVIATION,
            &source,
            &sequence.name,
            category,
            0,
            AlarmCondition::SequenceDeviation,
        ));
        for (index, interlock) in sequence.interlocks.iter().enumerate() {
            out.push(PlantAlarm {
                definition: AlarmDefinition {
                    id: format!("{source}.interlock.{}", interlock.id),
                    code: SEQUENCE_INTERLOCK.code.into(),
                    source: source.clone(),
                    source_label: interlock.name.clone(),
                    category,
                    priority: SEQUENCE_INTERLOCK.priority,
                    message: format!("{}: {}", interlock.name, SEQUENCE_INTERLOCK.text),
                    on_delay_us: 0,
                },
                condition: AlarmCondition::SequenceInterlock { index },
            });
        }
    }
    for (scan, view) in session.network.scan_views().into_iter().enumerate() {
        let source = format!("modbus.{}", view.id);
        let label = format!("Modbus TCP {} unit {}", view.server, view.unit_id);
        let condition = AlarmCondition::Connection { scan };
        let category = AlarmCategory::Network;
        out.push(plant_alarm(
            &COMMUNICATION_LOST,
            &source,
            &label,
            category,
            0,
            condition,
        ));
    }
    out
}

/// Whether the condition holds in the current world state.
pub(crate) fn holds(session: &LabSession, condition: &AlarmCondition) -> bool {
    let scene = &session.scene;
    match *condition {
        AlarmCondition::Quantity {
            component,
            name,
            when,
        } => scene.quantity(component, name) == Some(Quantity::Bool(when)),
        AlarmCondition::Discrepancy { component } => {
            let command = scene.quantity(component, "command");
            let position = scene.quantity(component, "positionPermille");
            match (command, position) {
                (Some(Quantity::Bool(true)), Some(Quantity::Integer(p))) => p < 1000,
                (Some(Quantity::Bool(false)), Some(Quantity::Integer(p))) => p > 0,
                _ => false,
            }
        }
        AlarmCondition::SignalInvalid { channel } => {
            session.station.reading(channel).is_some_and(|r| {
                matches!(
                    r.quality,
                    Quality::WireBreak
                        | Quality::Overflow
                        | Quality::Underflow
                        | Quality::Overrange
                        | Quality::Underrange
                )
            })
        }
        AlarmCondition::FieldPower { slot } => {
            session.station.module_status(slot) == ModuleStatus::NoFieldPower
        }
        AlarmCondition::ModuleFault { slot } => matches!(
            session.station.module_status(slot),
            ModuleStatus::Missing | ModuleStatus::NoModulePower | ModuleStatus::Mismatch(_)
        ),
        AlarmCondition::Connection { scan } => !matches!(
            session.network.scan_status(scan),
            None | Some(ScanStatus::Idle | ScanStatus::Ok)
        ),
        AlarmCondition::SequenceTimeout => session.sequence().is_some_and(|m| m.timed_out()),
        AlarmCondition::SequenceDeviation => {
            session.sequence().is_some_and(|m| m.deviation_latched())
        }
        AlarmCondition::SequenceInterlock { index } => session
            .sequence()
            .is_some_and(|m| m.interlock_active(index)),
    }
}
