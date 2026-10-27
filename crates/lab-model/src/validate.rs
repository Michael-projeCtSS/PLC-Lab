//! Structural and cross-reference validation. Every document is validated before use.

use crate::{
    SCHEMA_VERSION,
    exercise::{Condition, EvidenceSource, ExerciseDefinition, Rule, ScenarioAction},
    fault::FaultSpec,
    hardware::{ChannelMode, ModuleClass, ModuleType, StationConfig},
    ids::Identifier,
    network::{ModbusDataType, RegisterValue},
    scene::{ComponentKind, Coupling, PortKind, SceneDefinition},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    Parse(String),
    Invalid(Vec<String>),
}
impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(message) => write!(f, "Invalid document: {message}"),
            Self::Invalid(problems) => write!(f, "Invalid document: {}", problems.join("; ")),
        }
    }
}
impl std::error::Error for ModelError {}

struct Problems(Vec<String>);
impl Problems {
    fn check(&mut self, ok: bool, message: impl FnOnce() -> String) {
        if !ok {
            self.0.push(message());
        }
    }
    fn finish(self) -> Result<(), ModelError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(ModelError::Invalid(self.0))
        }
    }
}

fn port_kind(scene: &SceneDefinition, component: &Identifier, port: &str) -> Option<PortKind> {
    scene
        .components
        .iter()
        .find(|c| &c.id == component)?
        .kind
        .ports()
        .into_iter()
        .find(|(name, _)| *name == port)
        .map(|(_, kind)| kind)
}

pub fn validate_scene(scene: &SceneDefinition) -> Result<(), ModelError> {
    let mut p = Problems(Vec::new());
    p.check(scene.schema_version == SCHEMA_VERSION, || {
        format!(
            "scene schemaVersion {} is unsupported",
            scene.schema_version
        )
    });
    p.check(!scene.title.trim().is_empty(), || {
        "scene title is empty".into()
    });
    let mut kinds = BTreeMap::new();
    for c in &scene.components {
        p.check(kinds.insert(c.id.clone(), &c.kind).is_none(), || {
            format!("component '{}' is duplicated", c.id)
        });
    }
    let is = |id: &Identifier, f: fn(&ComponentKind) -> bool| kinds.get(id).is_some_and(|k| f(k));
    let tank = |k: &ComponentKind| matches!(k, ComponentKind::Tank { .. });
    for c in &scene.components {
        match &c.kind {
            ComponentKind::LevelSwitch {
                tank: t,
                switch_level_ml,
                ..
            } => {
                p.check(is(t, tank), || {
                    format!("'{}' references missing tank '{t}'", c.id)
                });
                if let Some(ComponentKind::Tank { capacity_ml, .. }) = kinds.get(t) {
                    p.check(switch_level_ml <= capacity_ml, || {
                        format!("'{}' switch level exceeds the tank capacity", c.id)
                    });
                }
            }
            ComponentKind::LevelTransmitter {
                tank: t,
                range_min_ml,
                range_max_ml,
                ..
            } => {
                p.check(is(t, tank), || {
                    format!("'{}' references missing tank '{t}'", c.id)
                });
                p.check(range_min_ml < range_max_ml, || {
                    format!("'{}' measuring range is empty", c.id)
                });
            }
            ComponentKind::Tank {
                capacity_ml,
                initial_ml,
                high_high_ml,
                low_low_ml,
            } => {
                p.check(*capacity_ml > 0 && initial_ml <= capacity_ml, || {
                    format!("tank '{}' has an invalid capacity or initial level", c.id)
                });
                for level in [high_high_ml, low_low_ml].into_iter().flatten() {
                    p.check(level <= capacity_ml, || {
                        format!("tank '{}' alarm level exceeds its capacity", c.id)
                    });
                }
            }
            ComponentKind::SelectorSwitch { positions, initial } => {
                p.check((2..=4).contains(&positions.len()), || {
                    format!("selector '{}' needs 2 to 4 positions", c.id)
                });
                p.check(usize::from(*initial) < positions.len(), || {
                    format!("selector '{}' initial position is out of range", c.id)
                });
            }
            ComponentKind::OnOffValve { travel_ms, .. } => {
                p.check(*travel_ms >= 2, || {
                    format!("valve '{}' travel time is too short", c.id)
                });
            }
            _ => {}
        }
    }
    for coupling in &scene.couplings {
        let (a, b, ok) = match coupling {
            Coupling::ValveFillsTank { valve, tank: t }
            | Coupling::ValveDrainsTank { valve, tank: t } => (
                valve,
                t,
                is(valve, |k| matches!(k, ComponentKind::OnOffValve { .. })) && is(t, tank),
            ),
            Coupling::PumpDrainsTank { pump, tank: t } => (
                pump,
                t,
                is(pump, |k| matches!(k, ComponentKind::Pump { .. })) && is(t, tank),
            ),
        };
        p.check(ok, || {
            format!("coupling between '{a}' and '{b}' references wrong component types")
        });
    }
    let mut positions = BTreeSet::new();
    let mut installed = BTreeMap::new();
    for m in &scene.installation.modules {
        p.check(positions.insert(m.position), || {
            format!(
                "installed slot R{}/S{} is duplicated",
                m.position.rack, m.position.slot
            )
        });
        p.check(
            m.module.is_io() || m.module == ModuleType::PowerSupply24Vdc,
            || "a CPU cannot be installed as an I/O module".into(),
        );
        installed.insert(m.position, m.module);
    }
    p.check(
        matches!(scene.installation.cpu.class(), ModuleClass::Cpu),
        || "installation.cpu must be a CPU module".into(),
    );
    let mut terminals = BTreeSet::new();
    for wire in &scene.installation.wiring {
        p.check(terminals.insert(wire.channel), || {
            format!("terminal {} is wired twice", wire.channel)
        });
        let Some(module) = installed.get(&wire.channel.slot_ref()) else {
            p.0.push(format!("wire to {} has no installed module", wire.channel));
            continue;
        };
        p.check(wire.channel.channel < module.channel_count(), || {
            format!(
                "terminal {} does not exist on {}",
                wire.channel,
                module.id()
            )
        });
        match port_kind(scene, &wire.component, &wire.port) {
            None => p.0.push(format!(
                "wire {} references missing port '{}.{}'",
                wire.channel, wire.component, wire.port
            )),
            Some(kind) => {
                let compatible = matches!(
                    (module.class(), kind),
                    (ModuleClass::DigitalInput { .. }, PortKind::DigitalSource)
                        | (ModuleClass::DigitalOutput { .. }, PortKind::DigitalSink)
                        | (ModuleClass::AnalogInput { .. }, PortKind::AnalogSource)
                );
                p.check(compatible, || {
                    format!(
                        "terminal {} cannot connect to port '{}.{}'",
                        wire.channel, wire.component, wire.port
                    )
                });
            }
        }
    }
    if let Some(network) = &scene.network {
        let mut ids = BTreeSet::new();
        for d in &network.devices {
            p.check(ids.insert(d.id.clone()), || {
                format!("network device '{}' is duplicated", d.id)
            });
            p.check(d.interface.mask.is_valid_mask(), || {
                format!("device '{}' has an invalid mask", d.id)
            });
            if let crate::network::DeviceRole::ModbusTcpServer {
                holding_registers,
                input_registers,
                ..
            } = &d.role
            {
                for binding in holding_registers.iter().chain(input_registers) {
                    if let RegisterValue::ScaledInt16 {
                        component, divisor, ..
                    }
                    | RegisterValue::Float32 {
                        component, divisor, ..
                    } = &binding.value
                    {
                        p.check(kinds.contains_key(component), || {
                            format!(
                                "register {} references missing component '{component}'",
                                binding.address
                            )
                        });
                        p.check(*divisor != 0, || {
                            format!("register {} divisor is zero", binding.address)
                        });
                    }
                }
            }
        }
    }
    for item in &scene.layout {
        p.check(kinds.contains_key(&item.component), || {
            format!("layout references missing component '{}'", item.component)
        });
    }
    let mut tags = BTreeSet::new();
    for s in &scene.interface {
        p.check(tags.insert(s.tag.key()), || {
            format!("interface tag '{}' is duplicated", s.tag)
        });
    }
    if let Err(ModelError::Invalid(problems)) = validate_station(&scene.reference_station) {
        p.0.extend(
            problems
                .into_iter()
                .map(|m| format!("referenceStation: {m}")),
        );
    }
    p.finish()
}

pub fn validate_station(station: &StationConfig) -> Result<(), ModelError> {
    let mut p = Problems(Vec::new());
    p.check(matches!(station.cpu.class(), ModuleClass::Cpu), || {
        "station cpu must be a CPU module".into()
    });
    let mut positions = BTreeSet::new();
    for m in &station.modules {
        p.check(positions.insert(m.position), || {
            format!(
                "configured slot R{}/S{} is duplicated",
                m.position.rack, m.position.slot
            )
        });
        let mut channels = BTreeSet::new();
        for c in &m.channels {
            p.check(channels.insert(c.channel), || {
                format!(
                    "R{}/S{} channel {} is configured twice",
                    m.position.rack, m.position.slot, c.channel
                )
            });
            p.check(c.channel < m.module.channel_count(), || {
                format!(
                    "R{}/S{} has no channel {}",
                    m.position.rack, m.position.slot, c.channel
                )
            });
            let mode_ok = match m.module.class() {
                ModuleClass::DigitalInput { .. } | ModuleClass::DigitalOutput { .. } => {
                    matches!(c.mode, ChannelMode::Digital | ChannelMode::Disabled)
                }
                ModuleClass::AnalogInput { .. } | ModuleClass::AnalogOutput { .. } => {
                    !matches!(c.mode, ChannelMode::Digital)
                }
                _ => false,
            };
            p.check(mode_ok, || {
                format!(
                    "R{}/S{} channel {} mode is not supported by {}",
                    m.position.rack,
                    m.position.slot,
                    c.channel,
                    m.module.id()
                )
            });
        }
    }
    let mut mapped_channels = BTreeSet::new();
    let mut mapped_tags = BTreeSet::new();
    for io in &station.io_map {
        p.check(mapped_channels.insert(io.channel), || {
            format!("channel {} is mapped twice", io.channel)
        });
        p.check(mapped_tags.insert(io.tag.key()), || {
            format!("tag '{}' is mapped twice", io.tag)
        });
        match station.module_at(io.channel.slot_ref()) {
            None => p.0.push(format!(
                "mapped channel {} has no configured module",
                io.channel
            )),
            Some(m) => p.check(io.channel.channel < m.module.channel_count(), || {
                format!("mapped channel {} does not exist", io.channel)
            }),
        }
    }
    if let Some(interface) = &station.network.interface {
        p.check(interface.mask.is_valid_mask(), || {
            "station interface mask is invalid".into()
        });
    }
    let mut scan_ids = BTreeSet::new();
    for scan in &station.network.modbus_tcp {
        p.check(scan_ids.insert(scan.id.clone()), || {
            format!("Modbus scan '{}' is duplicated", scan.id)
        });
        p.check((1..=125).contains(&scan.quantity), || {
            format!("Modbus scan '{}' quantity must be 1..125", scan.id)
        });
        p.check(scan.period_ms >= 10 && scan.timeout_ms >= 10, || {
            format!(
                "Modbus scan '{}' period and timeout must be at least 10 ms",
                scan.id
            )
        });
        p.check(station.network.interface.is_some(), || {
            format!(
                "Modbus scan '{}' requires a configured station interface",
                scan.id
            )
        });
        for v in &scan.values {
            let width = match v.data_type {
                ModbusDataType::Int16 => 1,
                ModbusDataType::Float32 { .. } => 2,
            };
            p.check(
                u32::from(v.offset) + width <= u32::from(scan.quantity),
                || {
                    format!(
                        "Modbus scan '{}' value '{}' lies outside the requested registers",
                        scan.id, v.tag
                    )
                },
            );
            p.check(mapped_tags.insert(v.tag.key()), || {
                format!("tag '{}' is mapped twice", v.tag)
            });
        }
        if let Some(tag) = &scan.status_tag {
            p.check(mapped_tags.insert(tag.key()), || {
                format!("tag '{tag}' is mapped twice")
            });
        }
    }
    p.finish()
}

fn condition_signals<'a>(condition: &'a Condition, out: &mut Vec<&'a Identifier>) {
    match condition {
        Condition::All { of } | Condition::Any { of } => {
            for c in of {
                condition_signals(c, out);
            }
        }
        Condition::Not { of } => condition_signals(of, out),
        Condition::Compare { signal, .. } => out.push(signal),
    }
}

fn fault_component(fault: &FaultSpec) -> Option<&Identifier> {
    match fault {
        FaultSpec::SensorStuck { component, .. }
        | FaultSpec::ContactTypeMismatch { component, .. }
        | FaultSpec::OutputStyleMismatch { component, .. }
        | FaultSpec::AnalogDrift { component, .. }
        | FaultSpec::AnalogStuck { component, .. }
        | FaultSpec::AnalogNoise { component, .. }
        | FaultSpec::LoopBreak { component }
        | FaultSpec::ValveStuck { component, .. }
        | FaultSpec::FeedbackMissing { component, .. }
        | FaultSpec::MotorOverloadTrip { component } => Some(component),
        _ => None,
    }
}

pub fn validate_exercise(
    exercise: &ExerciseDefinition,
    scene: &SceneDefinition,
) -> Result<(), ModelError> {
    let mut p = Problems(Vec::new());
    p.check(exercise.schema_version == SCHEMA_VERSION, || {
        format!(
            "exercise schemaVersion {} is unsupported",
            exercise.schema_version
        )
    });
    p.check(exercise.scene == scene.id, || {
        "exercise scene does not match the supplied scene".into()
    });
    let components: BTreeSet<&Identifier> = scene.components.iter().map(|c| &c.id).collect();
    let mut evidence = BTreeSet::new();
    for e in &exercise.evidence {
        p.check(evidence.insert(&e.name), || {
            format!("evidence '{}' is duplicated", e.name)
        });
        if let EvidenceSource::Component { component, .. } = &e.source {
            p.check(components.contains(component), || {
                format!(
                    "evidence '{}' references missing component '{component}'",
                    e.name
                )
            });
        }
    }
    let mut scenarios = BTreeSet::new();
    let mut fault_ids = BTreeSet::new();
    for s in &exercise.scenarios {
        p.check(scenarios.insert(&s.id), || {
            format!("scenario '{}' is duplicated", s.id)
        });
        p.check(s.duration_ms >= 100 && s.duration_ms <= 600_000, || {
            format!("scenario '{}' duration must be 100 ms to 10 minutes", s.id)
        });
        for a in &s.actions {
            p.check(a.at_ms <= s.duration_ms, || {
                format!("scenario '{}' action is after its end", s.id)
            });
            let component = match &a.action {
                ScenarioAction::Press { component }
                | ScenarioAction::Release { component }
                | ScenarioAction::SetSelector { component, .. }
                | ScenarioAction::SetPresence { component, .. } => Some(component),
                ScenarioAction::InjectFault { fault } => {
                    fault_ids.insert(fault.id.clone());
                    fault_component(&fault.fault)
                }
                _ => None,
            };
            if let Some(component) = component {
                p.check(components.contains(component), || {
                    format!(
                        "scenario '{}' references missing component '{component}'",
                        s.id
                    )
                });
            }
        }
    }
    let mut criteria = BTreeSet::new();
    let mut total = 0u64;
    for c in &exercise.criteria {
        p.check(criteria.insert(&c.id), || {
            format!("criterion '{}' is duplicated", c.id)
        });
        p.check(scenarios.contains(&c.scenario), || {
            format!(
                "criterion '{}' references missing scenario '{}'",
                c.id, c.scenario
            )
        });
        total += u64::from(c.points);
        let mut signals = Vec::new();
        match &c.rule {
            Rule::Always { condition, .. }
            | Rule::Never { condition }
            | Rule::Eventually { condition, .. }
            | Rule::StateAt { condition, .. } => condition_signals(condition, &mut signals),
            Rule::RespondsWithin {
                trigger, response, ..
            } => {
                condition_signals(trigger, &mut signals);
                condition_signals(response, &mut signals);
            }
            Rule::NoPlcFault | Rule::EventAbsent { .. } => {}
        }
        for s in signals {
            p.check(evidence.contains(s), || {
                format!("criterion '{}' references undefined evidence '{s}'", c.id)
            });
        }
    }
    p.check(total > 0, || "exercise criteria award no points".into());
    if let Some(t) = &exercise.troubleshooting {
        let options: BTreeSet<&Identifier> = t.diagnosis_options.iter().map(|o| &o.id).collect();
        p.check(options.len() == t.diagnosis_options.len(), || {
            "diagnosis options are duplicated".into()
        });
        p.check(!t.root_causes.is_empty(), || {
            "troubleshooting needs at least one root cause".into()
        });
        for r in &t.root_causes {
            p.check(options.contains(r), || {
                format!("root cause '{r}' is not a diagnosis option")
            });
        }
        for f in &t.faults {
            p.check(fault_ids.insert(f.id.clone()), || {
                format!("fault '{}' is duplicated", f.id)
            });
            if let Some(component) = fault_component(&f.fault) {
                p.check(components.contains(component), || {
                    format!(
                        "fault '{}' references missing component '{component}'",
                        f.id
                    )
                });
            }
        }
        for r in &t.repairs {
            for cleared in &r.clears {
                p.check(t.faults.iter().any(|f| &f.id == cleared), || {
                    format!("repair '{}' clears unknown fault '{cleared}'", r.id)
                });
            }
        }
        for v in &t.verification {
            p.check(criteria.contains(v), || {
                format!("verification references missing criterion '{v}'")
            });
        }
    }
    if let Some(station) = &exercise.starter_station
        && let Err(ModelError::Invalid(problems)) = validate_station(station)
    {
        p.0.extend(problems.into_iter().map(|m| format!("starterStation: {m}")));
    }
    p.finish()
}
