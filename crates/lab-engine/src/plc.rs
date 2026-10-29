//! The PLC participant: one CPU executing one compiled program with the accepted
//! transactional runtime. Mode handling (RUN/STOP/FAULT), I/O linking and forces live
//! here; PLC semantics stay in `plc-runtime`.

use crate::{
    error::{BindingIssue, EngineError},
    signals::{SignalId, SignalKind, SignalRegistry, SignalValue, ValueType},
};
use lab_model::{
    exercise::PlcModeRequest,
    hardware::{ChannelRef, ModuleClass, StationConfig},
    network::ModbusDataType,
    profile::{AnalogRepresentation, PlatformProfile},
};
use plc_runtime::{Runtime, RuntimeConfig, RuntimeSnapshot, ScanError};
use plc_types::{
    PlcTime, PlcType, PlcValue,
    diagnostics::Diagnostic,
    function_blocks::StandardFunctionBlock,
    identity::{FunctionBlockInstanceId, SymbolId},
    symbols::{Scope, VariableCategory},
};
use st_frontend::{CompiledProgram, CompilerConfig};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlcMode {
    Stop,
    Run,
    Fault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TagRole {
    Input,
    Output,
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagBinding {
    pub symbol: SymbolId,
    pub name: String,
    pub role: TagRole,
    pub plc_type: PlcType,
    pub signal: SignalId,
    pub source: Option<InputSource>,
    pub sink: Option<ChannelRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockBinding {
    pub instance: FunctionBlockInstanceId,
    pub name: String,
    pub kind: StandardFunctionBlock,
    pub outputs: Vec<(&'static str, SignalId)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSource {
    Channel(ChannelRef),
    Modbus { scan: usize, mapping: usize },
    ModbusStatus { scan: usize },
}

pub(crate) struct LoadedProgram {
    pub name: String,
    pub revision: u64,
    pub source: String,
    pub runtime: Runtime,
    pub tags: Vec<TagBinding>,
    pub blocks: Vec<BlockBinding>,
}

pub(crate) struct PlcParticipant {
    pub config: RuntimeConfig,
    pub switch: PlcModeRequest,
    pub mode: PlcMode,
    pub program: Option<LoadedProgram>,
    pub segment: u32,
    pub fault: Option<Diagnostic>,
    pub forces: BTreeMap<SignalId, SignalValue>,
    pub scans: u64,
}

fn type_value(kind: PlcType) -> ValueType {
    match kind {
        PlcType::Bool => ValueType::Bool,
        PlcType::Int => ValueType::Int,
        PlcType::Real => ValueType::Real,
        PlcType::Time => ValueType::Time,
    }
}
fn type_name(kind: PlcType) -> &'static str {
    match kind {
        PlcType::Bool => "BOOL",
        PlcType::Int => "INT",
        PlcType::Real => "REAL",
        PlcType::Time => "TIME",
    }
}

pub fn compile(source: &str) -> Result<CompiledProgram, Vec<Diagnostic>> {
    st_frontend::compile_st(source, CompilerConfig::default())
}

/// Linked tags, function-block outputs and non-blocking binding warnings.
pub(crate) type Linked = (Vec<TagBinding>, Vec<BlockBinding>, Vec<BindingIssue>);

/// Link compiled program tags to the station's I/O map and network scans.
pub(crate) fn link(
    compiled: &CompiledProgram,
    station: &StationConfig,
    profile: &PlatformProfile,
    registry: &mut SignalRegistry,
) -> Result<Linked, Vec<BindingIssue>> {
    let program = compiled.program();
    let scope = Scope::Pou(program.entry());
    let mut tags: Vec<TagBinding> = program
        .symbols()
        .symbols()
        .filter(|s| s.scope() == scope)
        .map(|s| {
            let role = match s.category() {
                VariableCategory::Input => TagRole::Input,
                VariableCategory::Output => TagRole::Output,
                _ => TagRole::Local,
            };
            let kind = match role {
                TagRole::Input => SignalKind::PlcInput,
                TagRole::Output => SignalKind::PlcOutput,
                TagRole::Local => SignalKind::PlcLocal,
            };
            TagBinding {
                symbol: s.id(),
                name: s.name().to_owned(),
                role,
                plc_type: s.plc_type(),
                signal: registry.ensure(
                    format!("plc.{}", s.name()),
                    s.name().to_owned(),
                    kind,
                    type_value(s.plc_type()),
                    None,
                ),
                source: None,
                sink: None,
            }
        })
        .collect();
    let blocks = program
        .function_blocks()
        .instances()
        .filter(|i| i.owner() == program.entry())
        .filter_map(|i| {
            let definition = program.function_blocks().definition(i.definition())?;
            let plc_types::function_blocks::FunctionBlockKind::Standard(kind) = definition.kind()
            else {
                return None;
            };
            let outputs = kind
                .signature()
                .iter()
                .filter(|p| p.direction == plc_types::function_blocks::ParameterDirection::Output)
                .map(|p| {
                    (
                        p.name,
                        registry.ensure(
                            format!("plc.{}.{}", i.name(), p.name),
                            format!("{}.{}", i.name(), p.name),
                            SignalKind::BlockOutput,
                            type_value(p.plc_type),
                            None,
                        ),
                    )
                })
                .collect();
            Some(BlockBinding {
                instance: i.id(),
                name: i.name().to_owned(),
                kind,
                outputs,
            })
        })
        .collect();
    let mut issues = Vec::new();
    let find = |tags: &[TagBinding], name: &str| {
        tags.iter().position(|t| t.name.eq_ignore_ascii_case(name))
    };
    for mapping in &station.io_map {
        let location = mapping.channel.to_string();
        let Some(index) = find(&tags, mapping.tag.as_str()) else {
            issues.push(BindingIssue {
                tag: mapping.tag.to_string(),
                location,
                message: format!(
                    "I/O tag '{}' on {} is not declared in the program; the channel is not used.",
                    mapping.tag, mapping.channel
                ),
                blocking: false,
            });
            continue;
        };
        let Some(module) = station.module_at(mapping.channel.slot_ref()) else {
            continue;
        };
        let (role, kind) = match module.module.class() {
            ModuleClass::DigitalInput { .. } => (TagRole::Input, PlcType::Bool),
            ModuleClass::DigitalOutput { .. } => (TagRole::Output, PlcType::Bool),
            ModuleClass::AnalogInput { .. } => (
                TagRole::Input,
                if profile.analog == AnalogRepresentation::EngineeringUnits {
                    PlcType::Real
                } else {
                    PlcType::Int
                },
            ),
            ModuleClass::AnalogOutput { .. } => (
                TagRole::Output,
                if profile.analog == AnalogRepresentation::EngineeringUnits {
                    PlcType::Real
                } else {
                    PlcType::Int
                },
            ),
            _ => {
                issues.push(BindingIssue {
                    tag: mapping.tag.to_string(),
                    location,
                    message: format!(
                        "Channel {} of {} cannot be linked to a tag in this simulation.",
                        mapping.channel,
                        module.module.generic_name()
                    ),
                    blocking: false,
                });
                continue;
            }
        };
        let tag = &mut tags[index];
        if tag.role != role || tag.plc_type != kind {
            issues.push(BindingIssue {
                tag: tag.name.clone(),
                location: location.clone(),
                message: format!(
                    "'{}' is {} {} but channel {} requires {} {}.",
                    tag.name,
                    role_name(tag.role),
                    type_name(tag.plc_type),
                    mapping.channel,
                    role_name(role),
                    type_name(kind)
                ),
                blocking: true,
            });
            continue;
        }
        match role {
            TagRole::Input => tag.source = Some(InputSource::Channel(mapping.channel)),
            _ => tag.sink = Some(mapping.channel),
        }
    }
    for (scan_index, scan) in station.network.modbus_tcp.iter().enumerate() {
        let mut link =
            |tags: &mut Vec<TagBinding>, name: &str, kind: PlcType, source: InputSource| {
                let Some(index) = find(tags, name) else {
                    issues.push(BindingIssue {
                        tag: name.to_owned(),
                        location: format!("Modbus scan '{}'", scan.id),
                        message: format!(
                            "Network tag '{name}' of scan '{}' is not declared in the program.",
                            scan.id
                        ),
                        blocking: false,
                    });
                    return;
                };
                let tag = &mut tags[index];
                if tag.role != TagRole::Input || tag.plc_type != kind {
                    issues.push(BindingIssue {
                        tag: tag.name.clone(),
                        location: format!("Modbus scan '{}'", scan.id),
                        message: format!(
                            "'{}' must be VAR_INPUT {} for scan '{}'.",
                            tag.name,
                            type_name(kind),
                            scan.id
                        ),
                        blocking: true,
                    });
                    return;
                }
                tag.source = Some(source);
            };
        for (mapping_index, value) in scan.values.iter().enumerate() {
            let kind = match value.data_type {
                ModbusDataType::Int16 => PlcType::Int,
                ModbusDataType::Float32 { .. } => PlcType::Real,
            };
            link(
                &mut tags,
                value.tag.as_str(),
                kind,
                InputSource::Modbus {
                    scan: scan_index,
                    mapping: mapping_index,
                },
            );
        }
        if let Some(status) = &scan.status_tag {
            link(
                &mut tags,
                status.as_str(),
                PlcType::Bool,
                InputSource::ModbusStatus { scan: scan_index },
            );
        }
    }
    if issues.iter().any(|i| i.blocking) {
        return Err(issues);
    }
    Ok((tags, blocks, issues))
}

fn role_name(role: TagRole) -> &'static str {
    match role {
        TagRole::Input => "VAR_INPUT",
        TagRole::Output => "VAR_OUTPUT",
        TagRole::Local => "VAR",
    }
}

pub(crate) enum ScanOutcome {
    Completed,
    Faulted(Diagnostic),
    NotScanned,
}

impl PlcParticipant {
    pub fn new(cycle_us: u64, budget: u64) -> Result<Self, EngineError> {
        Ok(Self {
            config: RuntimeConfig::new(PlcTime::from_micros(cycle_us), budget)
                .map_err(|e| EngineError::Internal(e.to_string()))?,
            switch: PlcModeRequest::Stop,
            mode: PlcMode::Stop,
            program: None,
            segment: 0,
            fault: None,
            forces: BTreeMap::new(),
            scans: 0,
        })
    }
    pub fn snapshot(&self) -> Option<RuntimeSnapshot> {
        self.program.as_ref().map(|p| p.runtime.snapshot())
    }
    /// Re-evaluate the actual mode from the mode switch, program and fault state.
    pub fn settle_mode(&mut self) -> Option<(PlcMode, PlcMode)> {
        let before = self.mode;
        let after = if self.mode == PlcMode::Fault {
            PlcMode::Fault
        } else if self.switch == PlcModeRequest::Run && self.program.is_some() {
            PlcMode::Run
        } else {
            PlcMode::Stop
        };
        if before != after {
            if before != PlcMode::Run && after == PlcMode::Run {
                self.warm_restart();
            }
            self.mode = after;
            return Some((before, after));
        }
        None
    }
    /// STOP to RUN: non-retentive memory and block state return to initial values.
    pub fn warm_restart(&mut self) {
        if let Some(p) = &mut self.program {
            p.runtime.reset();
        }
        self.segment += 1;
    }
    pub fn memory_reset(&mut self) {
        if let Some(p) = &mut self.program {
            p.runtime.reset();
        }
        self.fault = None;
        self.mode = PlcMode::Stop;
        self.segment += 1;
    }
    pub fn stage(&mut self, symbol: SymbolId, value: PlcValue) -> Result<(), EngineError> {
        let Some(p) = &mut self.program else {
            return Ok(());
        };
        p.runtime
            .set_input(symbol, value)
            .map_err(|e| EngineError::Internal(format!("input staging failed: {e}")))
    }
    pub fn scan(&mut self) -> Result<ScanOutcome, EngineError> {
        if self.mode != PlcMode::Run {
            return Ok(ScanOutcome::NotScanned);
        }
        let Some(p) = &mut self.program else {
            return Ok(ScanOutcome::NotScanned);
        };
        match p.runtime.single_scan() {
            Ok(_) => {
                self.scans += 1;
                Ok(ScanOutcome::Completed)
            }
            Err(ScanError::Fault(fault)) => {
                let diagnostic = fault.diagnostic().clone();
                self.fault = Some(diagnostic.clone());
                self.mode = PlcMode::Fault;
                Ok(ScanOutcome::Faulted(diagnostic))
            }
            Err(ScanError::Internal(e)) => {
                Err(EngineError::Internal(e.diagnostic().message().to_owned()))
            }
            Err(ScanError::Faulted) => Err(EngineError::Internal(
                "the runtime refused a scan in RUN mode".into(),
            )),
        }
    }
    pub fn tags(&self) -> &[TagBinding] {
        self.program.as_ref().map_or(&[], |p| p.tags.as_slice())
    }
    pub fn blocks(&self) -> &[BlockBinding] {
        self.program.as_ref().map_or(&[], |p| p.blocks.as_slice())
    }
    pub fn task_time(&self) -> u64 {
        self.snapshot().map_or(0, |s| s.time().as_micros())
    }
}
