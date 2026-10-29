//! Engine and model values to transport DTOs, including human-readable event and
//! diagnostic text. Presentation strings are produced here, never in the engine.

use app_contract::*;
use hardware_model::{
    DiagnosticEntry, DiagnosticEvent, DiagnosticKind, DiagnosticSource, LedState, ModuleStatus,
    Quality,
};
use lab_engine::{
    CauseEffect, CycleOutcome, LabEvent, LabEventKind, LabSession, ModeReason, OperatorAction,
    PlcMode, SignalKind, SignalValue,
};
use lab_model::network::{
    ModbusDataType, ModbusReadFunction, ModbusTcpScan, ModbusValueMapping, StationNetworkConfig,
    WordOrder,
};
use lab_model::{
    hardware::{ChannelMode, ModuleClass, ModuleType, StationConfig},
    profile::{
        AddressingScheme, AnalogRepresentation, FidelityLevel, FidelityStatement, PlatformProfile,
    },
};
use network_core::{NetworkEvent, NetworkEventKind, ScanStatus, ScanValue};
use plc_types::diagnostics::{Diagnostic, DiagnosticCategory, Severity};
use process_simulation::ProcessEventKind;

/// The request a configured Modbus TCP connection sends.
pub fn modbus_request(scan: &ModbusTcpScan) -> ModbusRequestDto {
    ModbusRequestDto {
        function: match scan.function {
            ModbusReadFunction::ReadHoldingRegisters => "readHoldingRegisters",
            ModbusReadFunction::ReadInputRegisters => "readInputRegisters",
        }
        .into(),
        function_code: scan.function.code(),
        start_address: scan.start_address,
        quantity: scan.quantity,
        port: scan.port,
        period_ms: scan.period_ms,
        timeout_ms: scan.timeout_ms,
    }
}

/// A configured value of a connection with its decoded content.
pub fn scan_value(mapping: &ModbusValueMapping, value: Option<ScanValue>) -> ScanValueDto {
    let (data_type, word_order) = match mapping.data_type {
        ModbusDataType::Int16 => ("int16", None),
        ModbusDataType::Float32 { word_order } => (
            "float32",
            Some(match word_order {
                WordOrder::HighWordFirst => "highWordFirst",
                WordOrder::LowWordFirst => "lowWordFirst",
            }),
        ),
    };
    ScanValueDto {
        tag: mapping.tag.to_string(),
        offset: mapping.offset,
        data_type: data_type.into(),
        word_order: word_order.map(Into::into),
        value: value.map(|v| match v {
            ScanValue::Int(i) => i.to_string(),
            ScanValue::Real(r) => format!("{r}"),
            ScanValue::NotFinite => "not a finite number".into(),
        }),
    }
}

/// One network event as structured traffic for the network view.
pub fn traffic(seq: u64, event: &NetworkEvent, config: &StationNetworkConfig) -> TrafficDto {
    let mut t = TrafficDto {
        seq: ExactU64(seq),
        time_us: ExactU64(event.time.as_micros()),
        connection: config
            .modbus_tcp
            .get(event.scan)
            .map_or_else(|| format!("#{}", event.scan), |s| s.id.to_string()),
        transaction: event.transaction,
        peer: event.peer.to_string(),
        kind: String::new(),
        unit_id: None,
        function_code: None,
        address: None,
        quantity: None,
        round_trip_us: None,
        exception_code: None,
    };
    t.kind = match &event.kind {
        NetworkEventKind::RequestSent {
            function,
            address,
            quantity,
            unit,
        } => {
            t.function_code = Some(*function);
            t.address = Some(*address);
            t.quantity = Some(*quantity);
            t.unit_id = Some(*unit);
            "request"
        }
        NetworkEventKind::ResponseReceived { round_trip_us } => {
            t.round_trip_us = Some(ExactU64(*round_trip_us));
            "response"
        }
        NetworkEventKind::ExceptionReceived { code } => {
            t.exception_code = Some(*code);
            "exception"
        }
        NetworkEventKind::Timeout => "timeout",
        NetworkEventKind::NoRoute => "noRoute",
        NetworkEventKind::ArpUnresolved { .. } => "arpUnresolved",
        NetworkEventKind::ConnectionRefused => "refused",
        NetworkEventKind::AddressConflict { .. } => "addressConflict",
    }
    .into();
    t
}

pub fn value(v: SignalValue) -> SignalValueDto {
    match v {
        SignalValue::Bool(value) => SignalValueDto::Bool { value },
        SignalValue::Int(value) => SignalValueDto::Int { value },
        SignalValue::Real(value) => SignalValueDto::Real { value },
        SignalValue::Time(t) => SignalValueDto::Time { value: ExactU64(t) },
        SignalValue::Count(value) => SignalValueDto::Count { value },
    }
}

pub fn value_from_dto(v: SignalValueDto) -> Option<SignalValue> {
    Some(match v {
        SignalValueDto::Bool { value } => SignalValue::Bool(value),
        SignalValueDto::Int { value } => SignalValue::Int(value),
        SignalValueDto::Real { value } if value.is_finite() => SignalValue::Real(value),
        SignalValueDto::Real { .. } => return None,
        SignalValueDto::Time { value } => SignalValue::Time(value.0),
        SignalValueDto::Count { value } => SignalValue::Count(value),
    })
}

pub fn diagnostic(d: &Diagnostic) -> DiagnosticDto {
    DiagnosticDto {
        code: d.code().to_owned(),
        severity: match d.severity() {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
        .into(),
        message: d.message().to_owned(),
        span: d.source().and_then(|s| s.text_span()).map(|s| SpanDto {
            start: PositionDto {
                line: s.start().line(),
                column: s.start().column(),
            },
            end: PositionDto {
                line: s.end().line(),
                column: s.end().column(),
            },
        }),
        category: match d.category() {
            DiagnosticCategory::CompilerDiagnostic => "compiler",
            DiagnosticCategory::RuntimeFault => "runtime",
            _ => "internal",
        }
        .into(),
    }
}

pub fn fidelity(f: &FidelityStatement) -> FidelityDto {
    FidelityDto {
        level: match f.level {
            FidelityLevel::ExactBehavioral => "EXACT_BEHAVIORAL",
            FidelityLevel::ApproximatedBehavioral => "APPROXIMATED_BEHAVIORAL",
            FidelityLevel::Conceptual => "CONCEPTUAL",
            FidelityLevel::NotImplemented => "NOT_IMPLEMENTED",
        }
        .into(),
        modeled: f.modeled.clone(),
        approximated: f.approximated.clone(),
        omitted: f.omitted.clone(),
    }
}

pub fn platform(p: &PlatformProfile) -> PlatformDto {
    PlatformDto {
        id: p.id.to_string(),
        display_name: p.display_name.clone(),
        vendor: p.vendor.to_string(),
        family: p.family.to_string(),
        tier: format!("{:?}", p.tier).to_uppercase(),
        fidelity: fidelity(&p.fidelity),
        addressing: match p.addressing {
            AddressingScheme::IecDirect => "IEC direct addresses (%IX0.0)",
            AddressingScheme::ByteBit => "Byte.bit process image (%I0.0)",
            AddressingScheme::ModuleDefinedTags => "Module-defined tags (Local:1:I.Data.0)",
            AddressingScheme::LinkedTerminal => "Linked terminal channels",
            AddressingScheme::OctalDevices => "Octal X/Y devices",
            AddressingScheme::UnitPorts => "NX unit I/O ports",
        }
        .into(),
        analog: match p.analog {
            AnalogRepresentation::PerTenThousand => "0..10000 = 0..100 % (PLC Lab generic)",
            AnalogRepresentation::S7Nominal27648 => "0..27648 nominal, banded over/underrange",
            AnalogRepresentation::FullScale32767 => "0..32767 full scale",
            AnalogRepresentation::EngineeringUnits => "REAL engineering units (mA / V)",
        }
        .into(),
    }
}

pub fn station_to_dto(station: &StationConfig) -> Result<StationDto, IpcError> {
    serde_json::to_value(station)
        .and_then(serde_json::from_value)
        .map_err(|e| {
            IpcError::new(
                "APP-CONTRACT-STATION",
                format!("Station configuration could not be represented: {e}"),
            )
        })
}

pub fn station_from_dto(dto: &StationDto) -> Result<StationConfig, IpcError> {
    let station: StationConfig = serde_json::to_value(dto)
        .and_then(serde_json::from_value)
        .map_err(|e| {
            IpcError::new(
                "PLC-HARDWARE-INVALID",
                format!("The hardware configuration is invalid: {e}"),
            )
        })?;
    lab_model::validate_station(&station)
        .map_err(|e| IpcError::new("PLC-HARDWARE-INVALID", e.to_string()))?;
    Ok(station)
}

pub fn signal_kind(kind: SignalKind) -> &'static str {
    match kind {
        SignalKind::PlcInput => "PLC_INPUT",
        SignalKind::PlcOutput => "PLC_OUTPUT",
        SignalKind::PlcLocal => "PLC_LOCAL",
        SignalKind::BlockOutput => "BLOCK_OUTPUT",
        SignalKind::Physical => "PHYSICAL",
        SignalKind::Channel => "CHANNEL",
    }
}

pub fn signals(session: &LabSession) -> Vec<SignalDto> {
    session
        .signals()
        .iter()
        .map(|d| SignalDto {
            id: d.id.0,
            path: d.path.clone(),
            label: d.label.clone(),
            kind: signal_kind(d.kind).into(),
            value_type: format!("{:?}", d.value_type).to_uppercase(),
            unit: d.unit.map(str::to_owned),
            active: d.active,
        })
        .collect()
}

pub fn mode(m: PlcMode) -> &'static str {
    match m {
        PlcMode::Run => "RUN",
        PlcMode::Stop => "STOP",
        PlcMode::Fault => "FAULT",
    }
}

pub fn led(state: LedState) -> &'static str {
    match state {
        LedState::Off => "OFF",
        LedState::Green => "GREEN",
        LedState::GreenFlashing => "GREEN_FLASHING",
        LedState::Red => "RED",
        LedState::RedFlashing => "RED_FLASHING",
        LedState::Amber => "AMBER",
    }
}

pub fn cpu_leds(session: &LabSession) -> Vec<LedDto> {
    let run = match session.plc_mode() {
        PlcMode::Run => LedState::Green,
        PlcMode::Stop => LedState::Amber,
        PlcMode::Fault => LedState::Red,
    };
    let error = if session.station().active_diagnostics().next().is_some() {
        LedState::RedFlashing
    } else {
        LedState::Off
    };
    let force = if session.forces().is_empty() {
        LedState::Off
    } else {
        LedState::Amber
    };
    let mut leds = vec![
        LedDto {
            name: "RUN/STOP".into(),
            state: led(run).into(),
        },
        LedDto {
            name: "ERROR".into(),
            state: led(error).into(),
        },
        LedDto {
            name: "FORCE".into(),
            state: led(force).into(),
        },
    ];
    if session
        .station()
        .configuration()
        .network
        .interface
        .is_some()
    {
        let link = if session.network().plc_link_up() {
            LedState::Green
        } else {
            LedState::Off
        };
        leds.push(LedDto {
            name: "LINK".into(),
            state: led(link).into(),
        });
    }
    leds
}

pub fn outcome(o: CycleOutcome) -> &'static str {
    match o {
        CycleOutcome::Scanned => "SCANNED",
        CycleOutcome::Faulted => "FAULTED",
        CycleOutcome::NotScanned(PlcMode::Fault) => "NOT_SCANNED_FAULT",
        CycleOutcome::NotScanned(_) => "NOT_SCANNED_STOP",
    }
}

pub fn cause_effect(ce: &CauseEffect) -> CauseEffectDto {
    let effects = |list: &[lab_engine::Effect]| {
        list.iter()
            .map(|e| EffectDto {
                signal: e.signal.0,
                value: value(e.value),
                changed: e.changed,
            })
            .collect()
    };
    CauseEffectDto {
        cycle: ExactU64(ce.header.cycle),
        start_us: ExactU64(ce.header.start_us),
        end_us: ExactU64(ce.header.end_us),
        segment: ce.header.segment,
        outcome: outcome(ce.header.outcome).into(),
        saw: effects(&ce.saw),
        decided: effects(&ce.decided),
        applied: effects(&ce.applied),
        process: ce
            .process
            .iter()
            .map(|c| ChangeDto {
                signal: c.signal.0,
                before: c.before.map(value),
                after: value(c.after),
            })
            .collect(),
        next: effects(&ce.next),
        fault: ce.fault.as_ref().map(diagnostic),
    }
}

pub fn quality(q: Quality) -> &'static str {
    match q {
        Quality::Good => "GOOD",
        Quality::NotMapped => "NOT_MAPPED",
        Quality::ChannelDisabled => "CHANNEL_DISABLED",
        Quality::ModuleFault => "MODULE_FAULT",
        Quality::NoFieldPower => "NO_FIELD_POWER",
        Quality::WireBreak => "WIRE_BREAK",
        Quality::Overflow => "OVERFLOW",
        Quality::Underflow => "UNDERFLOW",
        Quality::Overrange => "OVERRANGE",
        Quality::Underrange => "UNDERRANGE",
    }
}

pub fn hardware_finding(f: hardware_model::HardwareFinding) -> HardwareFindingDto {
    HardwareFindingDto {
        category: f.category.code().into(),
        severity: match f.severity {
            hardware_model::FindingSeverity::Warning => "warning",
            hardware_model::FindingSeverity::Error => "error",
        }
        .into(),
        slot: f.slot.map(|s| SlotRefDto {
            rack: s.rack,
            slot: s.slot,
        }),
        channel: f.channel.map(|c| ChannelRefDto {
            rack: c.rack,
            slot: c.slot,
            channel: c.channel,
        }),
        module_id: f.module_id,
        expected: f.expected,
        detected: f.detected,
        message: f.message,
    }
}

pub fn module_status(s: ModuleStatus) -> &'static str {
    match s {
        ModuleStatus::Ok => "OK",
        ModuleStatus::Missing => "MISSING",
        ModuleStatus::Mismatch(_) => "MISMATCH",
        ModuleStatus::NoFieldPower => "NO_FIELD_POWER",
        ModuleStatus::NoModulePower => "NO_MODULE_POWER",
        ModuleStatus::Unconfigured => "UNCONFIGURED",
    }
}

pub fn channel_mode(m: ChannelMode) -> &'static str {
    match m {
        ChannelMode::Disabled => "disabled",
        ChannelMode::Digital => "digital",
        ChannelMode::Current4To20mA => "current4To20mA",
        ChannelMode::Current0To20mA => "current0To20mA",
        ChannelMode::Voltage0To10V => "voltage0To10V",
    }
}

pub fn module_name(module: ModuleType, profile: &PlatformProfile) -> String {
    match profile.product_name(module) {
        Some(product) => format!("{product} ({})", module.generic_name()),
        None => module.generic_name().to_owned(),
    }
}

pub fn module_class(module: ModuleType) -> &'static str {
    match module.class() {
        ModuleClass::Cpu => "CPU",
        ModuleClass::PowerSupply => "PS",
        ModuleClass::DigitalInput { .. } => "DI",
        ModuleClass::DigitalOutput { .. } => "DO",
        ModuleClass::AnalogInput { .. } => "AI",
        ModuleClass::AnalogOutput { .. } => "AO",
    }
}

fn source_text(session: &LabSession, source: DiagnosticSource) -> String {
    match source {
        DiagnosticSource::Cpu => "CPU".into(),
        DiagnosticSource::Module(p) => format!("Rack {} slot {}", p.rack, p.slot),
        DiagnosticSource::Channel(c) => format!(
            "Rack {} slot {} channel {} ({})",
            c.rack,
            c.slot,
            c.channel,
            session.channel_address(c)
        ),
        DiagnosticSource::Connection(i) => session
            .station()
            .configuration()
            .network
            .modbus_tcp
            .get(usize::from(i))
            .map_or_else(
                || format!("Connection {i}"),
                |s| format!("Modbus TCP connection '{}' to {}", s.id, s.server),
            ),
        DiagnosticSource::Interface => "Ethernet interface".into(),
    }
}

pub fn diagnostic_text(session: &LabSession, kind: DiagnosticKind) -> String {
    let profile = session.profile();
    match kind {
        DiagnosticKind::ModuleMissing => "Module missing: the configured module is not plugged in or does not respond.".into(),
        DiagnosticKind::ModuleMismatch { configured, installed } => format!(
            "Configured module differs from installed module (configured {}, found {}).",
            configured.map_or_else(|| "none".into(), |c| module_name(c, profile)),
            module_name(installed, profile)
        ),
        DiagnosticKind::UnconfiguredModule => "A module is plugged in at a position that is not configured.".into(),
        DiagnosticKind::NoFieldPower => "Load/field supply voltage missing on the module.".into(),
        DiagnosticKind::WireBreak => "Wire break on analog input (loop current below the detection limit).".into(),
        DiagnosticKind::Overflow => "Analog input overflow (signal above the measuring range).".into(),
        DiagnosticKind::Underflow => "Analog input underflow (signal below the measuring range).".into(),
        DiagnosticKind::ConnectionTimeout => "No response from the remote device within the timeout.".into(),
        DiagnosticKind::ConnectionRefused => "The remote device refused the connection (no Modbus server on that address).".into(),
        DiagnosticKind::NoRoute => "No route to the remote device: the address is outside the PLC subnet and no gateway is configured.".into(),
        DiagnosticKind::ModbusException(code) => format!("Modbus exception {code:02X}h: {}", exception_name(code)),
        DiagnosticKind::IpAddressConflict => "IP address conflict detected on the network.".into(),
        DiagnosticKind::LinkDown => "Ethernet link down.".into(),
        DiagnosticKind::PlcFault => "CPU in FAULT: a runtime error stopped the program.".into(),
        DiagnosticKind::ForceActive => "Force job active: I/O values are overridden.".into(),
    }
}

pub fn exception_name(code: u8) -> &'static str {
    match code {
        1 => "illegal function",
        2 => "illegal data address",
        3 => "illegal data value",
        4 => "server device failure",
        0x0A => "gateway path unavailable",
        0x0B => "gateway target device failed to respond",
        _ => "unknown exception",
    }
}

pub fn diagnostic_entry(session: &LabSession, e: &DiagnosticEntry) -> DiagnosticEntryDto {
    let (slot, channel) = match e.source {
        DiagnosticSource::Module(p) => (Some(p), None),
        DiagnosticSource::Channel(c) => (Some(c.slot_ref()), Some(c)),
        _ => (None, None),
    };
    DiagnosticEntryDto {
        time_us: ExactU64(e.time.as_micros()),
        source: source_text(session, e.source),
        kind: format!("{:?}", e.kind)
            .split([' ', '{', '('])
            .next()
            .unwrap_or_default()
            .to_owned(),
        event: match e.event {
            DiagnosticEvent::Incoming => "INCOMING",
            DiagnosticEvent::Outgoing => "OUTGOING",
        }
        .into(),
        text: diagnostic_text(session, e.kind),
        slot: slot.map(|p| SlotRefDto {
            rack: p.rack,
            slot: p.slot,
        }),
        channel: channel.map(|c| ChannelRefDto {
            rack: c.rack,
            slot: c.slot,
            channel: c.channel,
        }),
        module_id: slot
            .and_then(|p| session.station().configuration().module_at(p))
            .map(|m| m.id.to_string()),
    }
}

fn component_label(session: &LabSession, id: &str) -> String {
    session
        .scene()
        .index_of_str(id)
        .ok()
        .and_then(|i| session.scene().component_label(i))
        .unwrap_or(id)
        .to_owned()
}

pub fn scan_status(s: ScanStatus) -> String {
    match s {
        ScanStatus::Idle => "IDLE".into(),
        ScanStatus::Ok => "OK".into(),
        ScanStatus::Timeout => "TIMEOUT".into(),
        ScanStatus::NoRoute => "NO_ROUTE".into(),
        ScanStatus::ConnectionRefused => "CONNECTION_REFUSED".into(),
        ScanStatus::Exception(c) => format!("EXCEPTION_{c:02X}"),
        ScanStatus::NoInterface => "NO_INTERFACE".into(),
    }
}

/// (kind, severity, text, source)
pub fn describe(
    session: &LabSession,
    e: &LabEvent,
) -> (String, &'static str, String, Option<String>) {
    match &e.kind {
        LabEventKind::ModeChanged { from, to, reason } => (
            "PLC_MODE".into(),
            if *to == PlcMode::Fault {
                "error"
            } else {
                "info"
            },
            format!(
                "CPU {} → {}{}",
                mode(*from),
                mode(*to),
                match reason {
                    ModeReason::Switch => " (mode switch)",
                    ModeReason::RuntimeFault => " (runtime error)",
                    ModeReason::Download => " (download)",
                    ModeReason::MemoryReset => " (memory reset)",
                    ModeReason::WorldReset => " (simulation reset)",
                }
            ),
            Some("CPU".into()),
        ),
        LabEventKind::PlcFault { diagnostic } => (
            "PLC_FAULT".into(),
            "error",
            format!("{}: {}", diagnostic.code(), diagnostic.message()),
            diagnostic
                .source()
                .and_then(|s| s.text_span())
                .map(|s| format!("Program line {}", s.start().line())),
        ),
        LabEventKind::ProgramDownloaded { name, revision } => (
            "DOWNLOAD".into(),
            "info",
            format!("Program {name} downloaded (revision {revision})."),
            Some("CPU".into()),
        ),
        LabEventKind::HardwareDownloaded => (
            "DOWNLOAD".into(),
            "info",
            "Hardware configuration downloaded.".into(),
            Some("CPU".into()),
        ),
        LabEventKind::WorldReset => (
            "SIMULATION".into(),
            "info",
            "Simulation reset to the initial plant state.".into(),
            None,
        ),
        LabEventKind::Process { component, kind } => {
            let label = component_label(session, component);
            let (severity, text) = match kind {
                ProcessEventKind::OverflowStarted => {
                    ("error", "overflow: the tank is full and still being filled")
                }
                ProcessEventKind::OverflowEnded => ("info", "overflow ended"),
                ProcessEventKind::DryRunStarted => {
                    ("error", "dry running: the pump is running on an empty tank")
                }
                ProcessEventKind::DryRunEnded => ("info", "dry running ended"),
                ProcessEventKind::HighHighOn => ("warning", "level high-high"),
                ProcessEventKind::HighHighOff => ("info", "level high-high cleared"),
                ProcessEventKind::LowLowOn => ("warning", "level low-low"),
                ProcessEventKind::LowLowOff => ("info", "level low-low cleared"),
                ProcessEventKind::MotorOverloadTripped => ("error", "overload relay tripped"),
                ProcessEventKind::MotorOverloadReset => ("info", "overload relay reset"),
            };
            (
                "PROCESS".into(),
                severity,
                format!("{label}: {text}"),
                Some(label),
            )
        }
        LabEventKind::Diagnostic { entry } => (
            "DIAGNOSTIC".into(),
            if entry.event == DiagnosticEvent::Incoming {
                "warning"
            } else {
                "info"
            },
            format!(
                "{} {}",
                if entry.event == DiagnosticEvent::Incoming {
                    "▲"
                } else {
                    "▼"
                },
                diagnostic_text(session, entry.kind)
            ),
            Some(source_text(session, entry.source)),
        ),
        LabEventKind::Network { event } => {
            let (severity, text) = match &event.kind {
                NetworkEventKind::RequestSent {
                    function,
                    address,
                    quantity,
                    unit,
                } => (
                    "info",
                    format!(
                        "→ FC{function:02} read {quantity} register(s) from address {address} (unit {unit}), TID {}",
                        event.transaction
                    ),
                ),
                NetworkEventKind::ResponseReceived { round_trip_us } => (
                    "info",
                    format!(
                        "← response TID {} after {:.1} ms",
                        event.transaction,
                        *round_trip_us as f64 / 1000.0
                    ),
                ),
                NetworkEventKind::ExceptionReceived { code } => (
                    "warning",
                    format!(
                        "← exception {code:02X}h ({}) TID {}",
                        exception_name(*code),
                        event.transaction
                    ),
                ),
                NetworkEventKind::Timeout => {
                    ("warning", format!("✕ timeout TID {}", event.transaction))
                }
                NetworkEventKind::NoRoute => (
                    "warning",
                    "✕ no route to host (destination outside the subnet, no gateway)".into(),
                ),
                NetworkEventKind::ArpUnresolved { ip } => {
                    ("warning", format!("✕ ARP: no device answers for {ip}"))
                }
                NetworkEventKind::ConnectionRefused => {
                    ("warning", "✕ connection refused (TCP reset)".into())
                }
                NetworkEventKind::AddressConflict { ip } => {
                    ("error", format!("⚠ several devices answer for {ip}"))
                }
            };
            (
                "NETWORK".into(),
                severity,
                text,
                Some(format!("Modbus TCP → {}", event.peer)),
            )
        }
        LabEventKind::Operator { action } => {
            let text = match action {
                OperatorAction::Press { component } => {
                    format!("Pressed {}", component_label(session, component))
                }
                OperatorAction::Release { component } => {
                    format!("Released {}", component_label(session, component))
                }
                OperatorAction::SetSelector {
                    component,
                    position,
                } => {
                    format!(
                        "Switched {} to position {}",
                        component_label(session, component),
                        position
                    )
                }
                OperatorAction::SetPresence { component, present } => format!(
                    "{} target {}",
                    component_label(session, component),
                    if *present { "present" } else { "removed" }
                ),
            };
            ("OPERATOR".into(), "info", text, None)
        }
        LabEventKind::Force { signal, value } => {
            let name = session
                .signals()
                .get(*signal)
                .map_or_else(|| signal.0.to_string(), |d| d.label.clone());
            match value {
                Some(v) => (
                    "FORCE".into(),
                    "warning",
                    format!("Force {name} = {}", format_value(*v)),
                    Some("CPU".into()),
                ),
                None => (
                    "FORCE".into(),
                    "info",
                    format!("Force removed from {name}"),
                    Some("CPU".into()),
                ),
            }
        }
        LabEventKind::Repair { label } => {
            ("REPAIR".into(), "info", format!("Repair: {label}"), None)
        }
    }
}

pub fn format_value(v: SignalValue) -> String {
    match v {
        SignalValue::Bool(b) => if b { "TRUE" } else { "FALSE" }.into(),
        SignalValue::Int(i) => i.to_string(),
        SignalValue::Real(r) => format!("{r}"),
        SignalValue::Time(t) => format!("T#{}ms", t / 1000),
        SignalValue::Count(c) => c.to_string(),
    }
}
