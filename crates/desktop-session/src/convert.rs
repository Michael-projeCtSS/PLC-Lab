//! Engine and model values to transport DTOs, including human-readable event and
//! diagnostic text. Presentation strings are produced here, never in the engine.

use app_contract::*;
use hardware_model::{
    CpuState, DiagnosticEntry, DiagnosticEvent, DiagnosticKind, DiagnosticSource, LedState,
    ModuleStatus, Quality, StartupKind,
};
use lab_engine::{
    AlarmCategory, AlarmEvent, AlarmPriority, AlarmState, AlarmStatus, AlarmTransition,
    CauseEffect, CycleOutcome, LabEvent, LabEventKind, LabSession, ModeReason, OperatorAction,
    PlcMode, SignalKind, SignalValue, StartKind,
};
use lab_model::message;
use lab_model::message::{Arg, IntoArg, Message, term};
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
    diagnostic_in(d, &[])
}

/// The project document id of compiled document `number`: 1 is the main program
/// (`None`), 2 and above the library documents in the order they were compiled
/// (ADR-0045).
pub fn document_id(number: u32, library: &[String]) -> Option<String> {
    let index = usize::try_from(number).ok()?.checked_sub(2)?;
    library.get(index).cloned()
}

/// A diagnostic of a program compiled with the library documents `library` (their ids).
pub fn diagnostic_in(d: &Diagnostic, library: &[String]) -> DiagnosticDto {
    DiagnosticDto {
        document: d
            .source()
            .and_then(|s| document_id(s.reference().document_id().get(), library)),
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
        expected: f.expected.as_ref().map(|m| m.text.clone()),
        detected: f.detected.as_ref().map(|m| m.text.clone()),
        message: f.message.text.clone(),
        detail: message(&f.message),
        expected_message: f.expected.as_ref().map(message),
        detected_message: f.detected.as_ref().map(message),
    }
}

/// A backend message for the interface: code, named values and the English text.
pub fn message(m: &Message) -> MessageDto {
    MessageDto {
        code: m.code.into(),
        text: m.text.clone(),
        args: m
            .args
            .iter()
            .map(|(name, arg)| MessageArgDto {
                name: (*name).into(),
                text: arg.to_string(),
                term: match arg {
                    Arg::Term { code, .. } => Some(code.clone()),
                    _ => None,
                },
                message: match arg {
                    Arg::Message(nested) => Some(Box::new(message(nested))),
                    _ => None,
                },
            })
            .collect(),
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
        DiagnosticKind::ModeChange { from, to } => format!(
            "CPU operating state changed from {} to {}.",
            cpu_state_name(from),
            cpu_state_name(to)
        ),
        DiagnosticKind::ProgramDownloaded => "Program downloaded into the CPU.".into(),
        DiagnosticKind::HardwareDownloaded => "Hardware configuration downloaded into the CPU.".into(),
        DiagnosticKind::Startup {
            kind: StartupKind::Warm,
            retained,
        } => format!(
            "Warm restart: non-retentive data initialized, {retained} retentive variable(s) kept."
        ),
        DiagnosticKind::Startup {
            kind: StartupKind::Cold,
            ..
        } => "Cold restart: all program data initialized, retentive variables included.".into(),
        DiagnosticKind::FirstScan { kind } => format!(
            "First program scan after the {} start executed.",
            startup_name(kind)
        ),
        DiagnosticKind::MemoryReset => {
            "Memory reset: all program data initialized and every force removed.".into()
        }
    }
}

/// [`diagnostic_text`] as a coded message with the same English text.
pub fn diagnostic_message(session: &LabSession, entry: &DiagnosticEntry) -> Message {
    let profile = session.profile();
    // The same text as module_name: the product name and the generic name as a term.
    let module = |m: ModuleType| match profile.product_name(m) {
        Some(product) => message!(
            "module.product",
            "{product} ({name})",
            product = product,
            name = m.name_term()
        )
        .into_arg(),
        None => m.name_term(),
    };
    let text = diagnostic_text(session, entry.kind);
    let (code, args): (&'static str, Vec<(&'static str, Arg)>) = match entry.kind {
        DiagnosticKind::ModuleMissing => ("event.diag.ModuleMissing", vec![]),
        DiagnosticKind::ModuleMismatch {
            configured,
            installed,
        } => (
            "event.diag.ModuleMismatch",
            vec![
                (
                    "configured",
                    configured.map_or_else(|| term("diag.value.none", "none"), module),
                ),
                ("installed", module(installed)),
            ],
        ),
        DiagnosticKind::UnconfiguredModule => ("event.diag.UnconfiguredModule", vec![]),
        DiagnosticKind::NoFieldPower => ("event.diag.NoFieldPower", vec![]),
        DiagnosticKind::WireBreak => ("event.diag.WireBreak", vec![]),
        DiagnosticKind::Overflow => ("event.diag.Overflow", vec![]),
        DiagnosticKind::Underflow => ("event.diag.Underflow", vec![]),
        DiagnosticKind::ConnectionTimeout => ("event.diag.ConnectionTimeout", vec![]),
        DiagnosticKind::ConnectionRefused => ("event.diag.ConnectionRefused", vec![]),
        DiagnosticKind::NoRoute => ("event.diag.NoRoute", vec![]),
        DiagnosticKind::ModbusException(code) => (
            "event.diag.ModbusException",
            vec![
                ("exception", Arg::Data(format!("{code:02X}h"))),
                ("name", exception_term(code)),
            ],
        ),
        DiagnosticKind::IpAddressConflict => ("event.diag.IpAddressConflict", vec![]),
        DiagnosticKind::LinkDown => ("event.diag.LinkDown", vec![]),
        DiagnosticKind::PlcFault => ("event.diag.PlcFault", vec![]),
        DiagnosticKind::ForceActive => ("event.diag.ForceActive", vec![]),
        DiagnosticKind::ModeChange { from, to } => (
            "event.diag.ModeChange",
            vec![
                ("from", Arg::Data(cpu_state_name(from).into())),
                ("to", Arg::Data(cpu_state_name(to).into())),
            ],
        ),
        DiagnosticKind::ProgramDownloaded => ("event.diag.ProgramDownloaded", vec![]),
        DiagnosticKind::HardwareDownloaded => ("event.diag.HardwareDownloaded", vec![]),
        DiagnosticKind::Startup {
            kind: StartupKind::Warm,
            retained,
        } => (
            "event.diag.Startup.warm",
            vec![("retained", Arg::Data(retained.to_string()))],
        ),
        DiagnosticKind::Startup {
            kind: StartupKind::Cold,
            ..
        } => ("event.diag.Startup.cold", vec![]),
        DiagnosticKind::FirstScan {
            kind: StartupKind::Warm,
        } => ("event.diag.FirstScan.warm", vec![]),
        DiagnosticKind::FirstScan {
            kind: StartupKind::Cold,
        } => ("event.diag.FirstScan.cold", vec![]),
        DiagnosticKind::MemoryReset => ("event.diag.MemoryReset", vec![]),
    };
    Message { code, args, text }
}

pub fn start_kind(kind: StartKind) -> &'static str {
    match kind {
        StartKind::Warm => "WARM",
        StartKind::Cold => "COLD",
    }
}

pub fn startup_name(kind: StartupKind) -> &'static str {
    match kind {
        StartupKind::Warm => "warm",
        StartupKind::Cold => "cold",
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

/// Diagnostic buffer entries for the view (oldest first) with what the backend derives:
/// severity, category, structured details and, for an incoming condition, whether it is
/// still active or when it cleared.
pub fn diagnostic_entries(
    session: &LabSession,
    entries: &[&DiagnosticEntry],
) -> Vec<DiagnosticEntryDto> {
    entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let cleared = if e.event == DiagnosticEvent::Incoming {
                entries[i + 1..]
                    .iter()
                    .find(|later| clears(later, e))
                    .map(|later| later.time.as_micros())
            } else {
                None
            };
            let active = e.event == DiagnosticEvent::Incoming
                && cleared.is_none()
                && session
                    .station()
                    .active_diagnostics()
                    .any(|(s, k)| *s == e.source && *k == e.kind);
            diagnostic_entry(session, e, cleared, active)
        })
        .collect()
}

/// Whether `later` is the outgoing entry of the condition `e` raised.
fn clears(later: &DiagnosticEntry, e: &DiagnosticEntry) -> bool {
    later.event == DiagnosticEvent::Outgoing && later.source == e.source && later.kind == e.kind
}

fn diagnostic_entry(
    session: &LabSession,
    e: &DiagnosticEntry,
    cleared_us: Option<u64>,
    active: bool,
) -> DiagnosticEntryDto {
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
            DiagnosticEvent::Instant => "EVENT",
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
        severity: diagnostic_severity(e).into(),
        category: match e.source {
            DiagnosticSource::Cpu => "CPU",
            DiagnosticSource::Module(_) | DiagnosticSource::Channel(_) => "HARDWARE",
            DiagnosticSource::Connection(_) | DiagnosticSource::Interface => "NETWORK",
        }
        .into(),
        details: diagnostic_details(session, e),
        active,
        cleared_us: cleared_us.map(ExactU64),
    }
}

/// Faults are errors while they are incoming; limits, forces and unconfigured modules are
/// warnings; cleared conditions and single events are information.
fn diagnostic_severity(e: &DiagnosticEntry) -> &'static str {
    if e.event != DiagnosticEvent::Incoming {
        return "info";
    }
    match e.kind {
        DiagnosticKind::Overflow
        | DiagnosticKind::Underflow
        | DiagnosticKind::UnconfiguredModule
        | DiagnosticKind::ModbusException(_)
        | DiagnosticKind::ForceActive => "warning",
        _ => "error",
    }
}

fn diagnostic_details(session: &LabSession, e: &DiagnosticEntry) -> Vec<DiagnosticDetailDto> {
    let detail = |name: &str, value: String| DiagnosticDetailDto {
        name: name.into(),
        value,
    };
    let mut out = Vec::new();
    match e.source {
        DiagnosticSource::Channel(c) => {
            let config = session.station().configuration();
            let id = config.channel_id(c).unwrap_or_else(|| c.to_string());
            out.push(detail("channel", id));
            out.push(detail("address", session.channel_address(c)));
        }
        DiagnosticSource::Module(p) => {
            if let Some(m) = session.station().configuration().module_at(p) {
                out.push(detail("module", m.id.to_string()));
            }
        }
        _ => {}
    }
    match e.kind {
        DiagnosticKind::ModeChange { from, to } => {
            out.push(detail("from", cpu_state_name(from).into()));
            out.push(detail("to", cpu_state_name(to).into()));
        }
        DiagnosticKind::ModuleMismatch {
            configured,
            installed,
        } => {
            let profile = session.profile();
            let configured = match configured {
                Some(c) => module_name(c, profile),
                None => "none".into(),
            };
            out.push(detail("configured", configured));
            out.push(detail("installed", module_name(installed, profile)));
        }
        DiagnosticKind::ModbusException(code) => {
            out.push(detail("exception", format!("{code:02X}h")));
        }
        DiagnosticKind::Startup { kind, retained } => {
            out.push(detail("start", startup_name(kind).into()));
            out.push(detail("retained", retained.to_string()));
        }
        DiagnosticKind::FirstScan { kind } => {
            out.push(detail("start", startup_name(kind).into()));
        }
        _ => {}
    }
    out
}

fn cpu_state_name(state: CpuState) -> &'static str {
    match state {
        CpuState::Run => "RUN",
        CpuState::Stop => "STOP",
        CpuState::Fault => "FAULT",
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

/// The Modbus exception name as a term with the interface's exception code.
fn exception_term(code: u8) -> Arg {
    term(
        format!("modbus.exception.{code:02X}h"),
        exception_name(code),
    )
}

/// (kind, severity, message, source)
pub fn describe(
    session: &LabSession,
    e: &LabEvent,
) -> (String, &'static str, Message, Option<String>) {
    match &e.kind {
        LabEventKind::ModeChanged { from, to, reason } => (
            "PLC_MODE".into(),
            if *to == PlcMode::Fault {
                "error"
            } else {
                "info"
            },
            message!(
                "event.mode",
                "CPU {from} → {to} ({reason})",
                from = mode(*from),
                to = mode(*to),
                reason = match reason {
                    ModeReason::Switch => term("event.reason.switch", "mode switch"),
                    ModeReason::RuntimeFault => term("event.reason.fault", "runtime error"),
                    ModeReason::Download => term("event.reason.download", "download"),
                    ModeReason::MemoryReset => term("event.reason.memoryReset", "memory reset"),
                    ModeReason::WorldReset => term("event.reason.worldReset", "simulation reset"),
                    ModeReason::WarmRestart => term("event.reason.warm", "warm restart"),
                    ModeReason::ColdRestart => term("event.reason.cold", "cold restart"),
                }
            ),
            Some("CPU".into()),
        ),
        LabEventKind::PlcFault { diagnostic } => (
            "PLC_FAULT".into(),
            "error",
            message!(
                "event.fault",
                "{code}: {detail}",
                code = diagnostic.code(),
                detail = diagnostic.message()
            ),
            diagnostic
                .source()
                .and_then(|s| s.text_span())
                .map(|s| format!("Program line {}", s.start().line())),
        ),
        LabEventKind::ProgramDownloaded { name, revision } => (
            "DOWNLOAD".into(),
            "info",
            message!(
                "event.programDownloaded",
                "Program {name} downloaded (revision {revision}).",
                name = name.as_str(),
                revision = *revision
            ),
            Some("CPU".into()),
        ),
        LabEventKind::HardwareDownloaded => (
            "DOWNLOAD".into(),
            "info",
            message!(
                "event.hardwareDownloaded",
                "Hardware configuration downloaded."
            ),
            Some("CPU".into()),
        ),
        LabEventKind::WorldReset => (
            "SIMULATION".into(),
            "info",
            message!(
                "event.worldReset",
                "Simulation reset to the initial plant state."
            ),
            None,
        ),
        LabEventKind::Process { component, kind } => {
            let label = component_label(session, component);
            let (severity, code, text) = match kind {
                ProcessEventKind::OverflowStarted => (
                    "error",
                    "event.process.overflow",
                    "overflow: the tank is full and still being filled",
                ),
                ProcessEventKind::OverflowEnded => {
                    ("info", "event.process.overflowEnded", "overflow ended")
                }
                ProcessEventKind::DryRunStarted => (
                    "error",
                    "event.process.dryRun",
                    "dry running: the pump is running on an empty tank",
                ),
                ProcessEventKind::DryRunEnded => {
                    ("info", "event.process.dryRunEnded", "dry running ended")
                }
                ProcessEventKind::HighHighOn => {
                    ("warning", "event.process.highHigh", "level high-high")
                }
                ProcessEventKind::HighHighOff => (
                    "info",
                    "event.process.highHighCleared",
                    "level high-high cleared",
                ),
                ProcessEventKind::LowLowOn => ("warning", "event.process.lowLow", "level low-low"),
                ProcessEventKind::LowLowOff => (
                    "info",
                    "event.process.lowLowCleared",
                    "level low-low cleared",
                ),
                ProcessEventKind::MotorOverloadTripped => {
                    ("error", "event.process.overload", "overload relay tripped")
                }
                ProcessEventKind::MotorOverloadReset => (
                    "info",
                    "event.process.overloadReset",
                    "overload relay reset",
                ),
            };
            (
                "PROCESS".into(),
                severity,
                message!(
                    "event.process",
                    "{label}: {event}",
                    label = label.as_str(),
                    event = term(code, text)
                ),
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
            message!(
                "event.diagnostic",
                "{arrow} {diagnostic}",
                arrow = if entry.event == DiagnosticEvent::Incoming {
                    "▲"
                } else {
                    "▼"
                },
                diagnostic = diagnostic_message(session, entry)
            ),
            Some(source_text(session, entry.source)),
        ),
        LabEventKind::Network { event } => {
            let tid = event.transaction;
            let (severity, text) = match &event.kind {
                NetworkEventKind::RequestSent {
                    function,
                    address,
                    quantity,
                    unit,
                } => (
                    "info",
                    message!(
                        "event.net.request",
                        "→ FC{function} read {quantity} register(s) from address {address} (unit {unit}), TID {tid}",
                        function = format!("{function:02}"),
                        quantity = *quantity,
                        address = *address,
                        unit = *unit,
                        tid = tid
                    ),
                ),
                NetworkEventKind::ResponseReceived { round_trip_us } => (
                    "info",
                    message!(
                        "event.net.response",
                        "← response TID {tid} after {time} ms",
                        tid = tid,
                        time = format!("{:.1}", *round_trip_us as f64 / 1000.0)
                    ),
                ),
                NetworkEventKind::ExceptionReceived { code } => (
                    "warning",
                    message!(
                        "event.net.exception",
                        "← exception {code}h ({name}) TID {tid}",
                        name = exception_term(*code),
                        code = format!("{code:02X}"),
                        tid = tid
                    ),
                ),
                NetworkEventKind::Timeout => (
                    "warning",
                    message!("event.net.timeout", "✕ timeout TID {tid}", tid = tid),
                ),
                NetworkEventKind::NoRoute => (
                    "warning",
                    message!(
                        "event.net.noRoute",
                        "✕ no route to host (destination outside the subnet, no gateway)"
                    ),
                ),
                NetworkEventKind::ArpUnresolved { ip } => (
                    "warning",
                    message!(
                        "event.net.arp",
                        "✕ ARP: no device answers for {ip}",
                        ip = ip.to_string()
                    ),
                ),
                NetworkEventKind::ConnectionRefused => (
                    "warning",
                    message!("event.net.refused", "✕ connection refused (TCP reset)"),
                ),
                NetworkEventKind::AddressConflict { ip } => (
                    "error",
                    message!(
                        "event.net.conflict",
                        "⚠ several devices answer for {ip}",
                        ip = ip.to_string()
                    ),
                ),
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
                OperatorAction::Press { component } => message!(
                    "event.op.press",
                    "Pressed {device}",
                    device = component_label(session, component)
                ),
                OperatorAction::Release { component } => message!(
                    "event.op.release",
                    "Released {device}",
                    device = component_label(session, component)
                ),
                OperatorAction::SetSelector {
                    component,
                    position,
                } => message!(
                    "event.op.selector",
                    "Switched {device} to position {position}",
                    device = component_label(session, component),
                    position = position.to_string()
                ),
                OperatorAction::SetPresence { component, present } => message!(
                    "event.op.presence",
                    "{device} target {state}",
                    device = component_label(session, component),
                    state = if *present {
                        term("event.op.present", "present")
                    } else {
                        term("event.op.removed", "removed")
                    }
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
                    message!(
                        "event.force",
                        "Force {name} = {value}",
                        name = name.as_str(),
                        value = format_value(*v)
                    ),
                    Some("CPU".into()),
                ),
                None => (
                    "FORCE".into(),
                    "info",
                    message!(
                        "event.forceRemoved",
                        "Force removed from {name}",
                        name = name.as_str()
                    ),
                    Some("CPU".into()),
                ),
            }
        }
        LabEventKind::Repair { label } => (
            "REPAIR".into(),
            "info",
            message!("event.repair", "Repair: {label}", label = label.as_str()),
            None,
        ),
        LabEventKind::Started { kind, retained } => (
            "PLC_START".into(),
            "info",
            match kind {
                StartKind::Warm => message!(
                    "event.warmStart",
                    "Warm start: non-retentive data initialized, {retained} retentive variable(s) kept",
                    retained = *retained
                ),
                StartKind::Cold => {
                    message!(
                        "event.coldStart",
                        "Cold start: all program data initialized"
                    )
                }
            },
            Some("CPU".into()),
        ),
        LabEventKind::FirstScan { kind } => (
            "PLC_FIRST_SCAN".into(),
            "info",
            match kind {
                StartKind::Warm => {
                    message!("event.firstScanWarm", "First scan after the warm start")
                }
                StartKind::Cold => {
                    message!("event.firstScanCold", "First scan after the cold start")
                }
            },
            Some("CPU".into()),
        ),
        LabEventKind::MemoryReset => (
            "PLC_MEMORY_RESET".into(),
            "warning",
            message!(
                "event.memoryReset",
                "Memory reset: program data initialized, forces removed"
            ),
            Some("CPU".into()),
        ),
        LabEventKind::Alarm {
            code,
            message: text,
            priority,
            transition,
            ..
        } => {
            let severity = match (transition, priority) {
                (AlarmTransition::Activated, AlarmPriority::Critical | AlarmPriority::Alarm) => {
                    "error"
                }
                (AlarmTransition::Activated, AlarmPriority::Warning) => "warning",
                _ => "info",
            };
            let what = match transition {
                AlarmTransition::Activated => term("event.alarm.activated", "alarm"),
                AlarmTransition::Acknowledged => term("event.alarm.acknowledged", "acknowledged"),
                AlarmTransition::ReturnedToNormal => {
                    term("event.alarm.returned", "returned to normal")
                }
                AlarmTransition::Withdrawn => term("event.alarm.withdrawn", "withdrawn"),
            };
            (
                "ALARM".into(),
                severity,
                message!(
                    "event.alarm",
                    "{alarm} ({what})",
                    alarm = term(format!("alarm.code.{code}"), text.as_str()),
                    what = what
                ),
                None,
            )
        }
        LabEventKind::Sequence { event } => {
            let (severity, message) = sequence_message(session, event);
            ("SEQUENCE".into(), severity, message, None)
        }
    }
}

/// A step as `S<number>` (language-neutral); the inspector shows localized step names.
fn step_label(session: &LabSession, step: Option<&lab_model::Identifier>) -> String {
    let number = step.and_then(|id| {
        session
            .sequence()
            .and_then(|m| m.definition().step(id))
            .map(|s| s.number)
    });
    number.map_or_else(|| "?".into(), |n| format!("S{n}"))
}

/// An event-log line for a sequence event (ADR-0043): coded, with identifiers as data.
fn sequence_message(
    session: &LabSession,
    e: &lab_engine::sequence_core::SequenceEvent,
) -> (&'static str, Message) {
    use lab_engine::sequence_core::{Deviation, RunStart, SequenceEventKind as K};
    let sequence = e.sequence.as_str();
    let step = step_label(session, e.step.as_ref());
    let from = step_label(session, e.from.as_ref());
    let transition = e.transition.as_ref().map_or("", |t| t.as_str());
    let interlock = e.interlock.as_ref().map_or("", |t| t.as_str());
    let reasons = e
        .permissives
        .iter()
        .chain(&e.interlocks)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    match e.kind {
        K::RunStarted { start } => (
            "info",
            message!(
                "event.sequence.runStarted",
                "Sequence {sequence}: new run in step {step} ({start})",
                sequence = sequence,
                step = step.as_str(),
                start = match start {
                    RunStart::Warm => term("sequence.start.warm", "warm start"),
                    RunStart::Cold => term("sequence.start.cold", "cold start"),
                    RunStart::Attached => term("sequence.start.attached", "program running"),
                }
            ),
        ),
        K::TransitionCompleted => (
            "info",
            message!(
                "event.sequence.transition",
                "Sequence {sequence}: transition {transition}, step {from} → {step}",
                sequence = sequence,
                transition = transition,
                from = from.as_str(),
                step = step.as_str()
            ),
        ),
        K::SequenceStarted => (
            "info",
            message!(
                "event.sequence.started",
                "Sequence {sequence} started",
                sequence = sequence
            ),
        ),
        K::SequenceCompleted => (
            "info",
            message!(
                "event.sequence.completed",
                "Sequence {sequence} completed",
                sequence = sequence
            ),
        ),
        K::SequenceAborted => (
            "warning",
            message!(
                "event.sequence.aborted",
                "Sequence {sequence} aborted in step {from}",
                sequence = sequence,
                from = from.as_str()
            ),
        ),
        K::StartBlocked => (
            "warning",
            message!(
                "event.sequence.startBlocked",
                "Sequence {sequence}: start refused ({reasons})",
                sequence = sequence,
                reasons = reasons.as_str()
            ),
        ),
        K::TransitionBlocked => (
            "warning",
            message!(
                "event.sequence.transitionBlocked",
                "Sequence {sequence}: transition {transition} refused ({reasons})",
                sequence = sequence,
                transition = transition,
                reasons = reasons.as_str()
            ),
        ),
        K::InterlockActivated => (
            "warning",
            message!(
                "event.sequence.interlockActive",
                "Sequence {sequence}: interlock {interlock} active in step {step}",
                sequence = sequence,
                interlock = interlock,
                step = step.as_str()
            ),
        ),
        K::InterlockCleared => (
            "info",
            message!(
                "event.sequence.interlockCleared",
                "Sequence {sequence}: interlock {interlock} cleared",
                sequence = sequence,
                interlock = interlock
            ),
        ),
        K::Held => (
            "info",
            message!(
                "event.sequence.held",
                "Sequence {sequence} held in step {step}",
                sequence = sequence,
                step = step.as_str()
            ),
        ),
        K::Resumed => (
            "info",
            message!(
                "event.sequence.resumed",
                "Sequence {sequence} resumed in step {step}",
                sequence = sequence,
                step = step.as_str()
            ),
        ),
        K::StepTimeout => (
            "error",
            message!(
                "event.sequence.timeout",
                "Sequence {sequence}: step {step} timed out",
                sequence = sequence,
                step = step.as_str()
            ),
        ),
        K::FaultCleared => (
            "info",
            message!(
                "event.sequence.faultCleared",
                "Sequence {sequence}: fault cleared",
                sequence = sequence
            ),
        ),
        K::Deviation(d) => {
            let deviation = match d {
                Deviation::UnknownStep => {
                    term("sequence.deviation.unknownStep", "unknown step value")
                }
                Deviation::UndefinedTransition => term(
                    "sequence.deviation.undefinedTransition",
                    "step change not defined",
                ),
                Deviation::ConditionNotMet => term(
                    "sequence.deviation.conditionNotMet",
                    "transition condition not met",
                ),
                Deviation::PermissiveViolated => term(
                    "sequence.deviation.permissiveViolated",
                    "started without permission",
                ),
                Deviation::InterlockViolated => term(
                    "sequence.deviation.interlockViolated",
                    "interlock not respected",
                ),
                Deviation::ProgressDuringHold => term(
                    "sequence.deviation.progressDuringHold",
                    "advanced while held",
                ),
                Deviation::ProgressAfterFault => term(
                    "sequence.deviation.progressAfterFault",
                    "advanced after a fault",
                ),
            };
            let at = if e.step.is_some() {
                step
            } else {
                e.step_value.map_or_else(|| "?".into(), |v| v.to_string())
            };
            (
                "error",
                message!(
                    "event.sequence.deviation",
                    "Sequence {sequence}: {deviation} (step {step})",
                    sequence = sequence,
                    deviation = deviation,
                    step = at.as_str()
                ),
            )
        }
    }
}

pub fn alarm_category(category: AlarmCategory) -> &'static str {
    match category {
        AlarmCategory::Process => "PROCESS",
        AlarmCategory::Hardware => "HARDWARE",
        AlarmCategory::Network => "NETWORK",
    }
}
pub fn alarm_priority(priority: AlarmPriority) -> &'static str {
    match priority {
        AlarmPriority::Information => "INFORMATION",
        AlarmPriority::Warning => "WARNING",
        AlarmPriority::Alarm => "ALARM",
        AlarmPriority::Critical => "CRITICAL",
    }
}
pub fn alarm_state(state: AlarmState) -> &'static str {
    match state {
        AlarmState::Normal => "NORMAL",
        AlarmState::ActiveUnacknowledged => "ACTIVE_UNACKNOWLEDGED",
        AlarmState::ActiveAcknowledged => "ACTIVE_ACKNOWLEDGED",
        AlarmState::ReturnedUnacknowledged => "RETURNED_UNACKNOWLEDGED",
    }
}

pub fn alarm(status: &AlarmStatus) -> AlarmDto {
    let d = &status.definition;
    AlarmDto {
        component: d.source.clone(),
        kind: d.code.clone(),
        text: d.message.clone(),
        id: d.id.clone(),
        source_label: d.source_label.clone(),
        category: alarm_category(d.category).into(),
        priority: alarm_priority(d.priority).into(),
        state: alarm_state(status.state).into(),
        condition: status.condition,
        activated_us: status.activated.map(|t| ExactU64(t.as_micros())),
        changed_us: status.changed.map(|t| ExactU64(t.as_micros())),
        acknowledgments: status.acknowledgments,
    }
}

pub fn alarm_event(e: &AlarmEvent) -> AlarmEventDto {
    let d = &e.definition;
    AlarmEventDto {
        seq: ExactU64(e.seq),
        time_us: ExactU64(e.time.as_micros()),
        alarm: d.id.clone(),
        code: d.code.clone(),
        source: d.source.clone(),
        source_label: d.source_label.clone(),
        category: alarm_category(d.category).into(),
        priority: alarm_priority(d.priority).into(),
        message: d.message.clone(),
        transition: match e.transition {
            AlarmTransition::Activated => "ACTIVATED",
            AlarmTransition::Acknowledged => "ACKNOWLEDGED",
            AlarmTransition::ReturnedToNormal => "RETURNED_TO_NORMAL",
            AlarmTransition::Withdrawn => "WITHDRAWN",
        }
        .into(),
        from: alarm_state(e.from).into(),
        to: alarm_state(e.to).into(),
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

/// A user function or function block for the library browser (ADR-0044), with the
/// library document that defines it (ADR-0045).
pub fn pou(p: &st_frontend::PouSummary, library: &[String]) -> PouDto {
    PouDto {
        document: document_id(p.document, library),
        name: p.name.clone(),
        kind: p.kind.clone(),
        return_type: p.return_type.clone(),
        parameters: p
            .parameters
            .iter()
            .map(|x| PouParameterDto {
                name: x.name.clone(),
                data_type: x.type_name.clone(),
                direction: x.direction.clone(),
                description: x.description.clone(),
            })
            .collect(),
        description: p.description.clone(),
        line: p.span.start().line(),
    }
}

/// The standard function blocks of the runtime with their signatures.
pub fn standard_blocks() -> Vec<StandardBlockDto> {
    use plc_types::function_blocks::{ParameterDirection, StandardFunctionBlock};
    StandardFunctionBlock::ALL
        .iter()
        .map(|block| StandardBlockDto {
            name: block.name().into(),
            parameters: block
                .signature()
                .iter()
                .map(|p| StandardParameterDto {
                    name: p.name.into(),
                    data_type: plc_types_name(p.plc_type).into(),
                    direction: match p.direction {
                        ParameterDirection::Input => "Input",
                        ParameterDirection::Output => "Output",
                        ParameterDirection::InOut => "InOut",
                    }
                    .into(),
                    required: p.required,
                })
                .collect(),
        })
        .collect()
}

fn plc_types_name(t: plc_types::PlcType) -> &'static str {
    match t {
        plc_types::PlcType::Bool => "BOOL",
        plc_types::PlcType::Int => "INT",
        plc_types::PlcType::Real => "REAL",
        plc_types::PlcType::Time => "TIME",
    }
}
