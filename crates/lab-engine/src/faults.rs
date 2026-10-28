//! Typed fault application. Each physical fault maps to exactly one subsystem effect.

use crate::{error::EngineError, session::LabSession};
use hardware_model::HardwareFault;
use lab_model::fault::FaultSpec;
use process_simulation::ComponentFault;

pub(crate) fn apply(
    session: &mut LabSession,
    fault: &FaultSpec,
    active: bool,
) -> Result<(), EngineError> {
    let now = session.now();
    let component = |s: &LabSession, id: &lab_model::Identifier| {
        s.scene.index_of(id).map_err(EngineError::from)
    };
    let on_component = |s: &mut LabSession,
                        id: &lab_model::Identifier,
                        f: ComponentFault|
     -> Result<(), EngineError> {
        let index = component(s, id)?;
        if active {
            s.scene.apply_fault(index, f)?;
        } else {
            s.scene.clear_fault(index, &f)?;
        }
        Ok(())
    };
    match fault {
        FaultSpec::SensorStuck {
            component: c,
            conducting,
        } => on_component(session, c, ComponentFault::SensorStuck(*conducting)),
        FaultSpec::ContactTypeMismatch {
            component: c,
            installed,
        } => on_component(session, c, ComponentFault::ContactType(*installed)),
        FaultSpec::OutputStyleMismatch {
            component: c,
            installed,
        } => on_component(session, c, ComponentFault::OutputStyle(*installed)),
        FaultSpec::AnalogDrift {
            component: c,
            offset,
        } => on_component(session, c, ComponentFault::AnalogDrift(*offset)),
        FaultSpec::AnalogStuck {
            component: c,
            value,
        } => on_component(session, c, ComponentFault::AnalogStuck(*value)),
        FaultSpec::AnalogNoise {
            component: c,
            amplitude,
        } => on_component(session, c, ComponentFault::AnalogNoise(*amplitude)),
        FaultSpec::LoopBreak { component: c } => {
            on_component(session, c, ComponentFault::LoopBreak)
        }
        FaultSpec::ValveStuck {
            component: c,
            position,
        } => on_component(session, c, ComponentFault::ValveStuck(*position)),
        FaultSpec::FeedbackMissing { component: c, port } => {
            on_component(session, c, ComponentFault::FeedbackMissing(port.clone()))
        }
        FaultSpec::DriveLocalControl { component: c } => {
            on_component(session, c, ComponentFault::DriveLocalControl)
        }
        FaultSpec::MotorOverloadTrip { component: c } => {
            on_component(session, c, ComponentFault::MotorOverload)
        }
        FaultSpec::OpenCircuit { channel } => {
            hardware(session, HardwareFault::OpenCircuit(*channel), active, now)
        }
        FaultSpec::ShortToSupply { channel } => {
            hardware(session, HardwareFault::ShortToSupply(*channel), active, now)
        }
        FaultSpec::SwappedWires { first, second } => hardware(
            session,
            HardwareFault::SwappedWires(*first, *second),
            active,
            now,
        ),
        FaultSpec::ModuleMissing { position } => hardware(
            session,
            HardwareFault::ModuleMissing(*position),
            active,
            now,
        ),
        FaultSpec::WrongModuleInstalled {
            position,
            installed,
        } => hardware(
            session,
            HardwareFault::WrongModule(*position, *installed),
            active,
            now,
        ),
        FaultSpec::ModuleInWrongSlot { from, to } => {
            hardware(session, HardwareFault::WrongSlot(*from, *to), active, now)
        }
        FaultSpec::FieldPowerMissing { position } => hardware(
            session,
            HardwareFault::FieldPowerMissing(*position),
            active,
            now,
        ),
        FaultSpec::NetworkLinkDown { device } => {
            network(session.network.set_device_link(device, !active), device)
        }
        FaultSpec::PlcLinkDown => {
            session.network.set_plc_link(!active);
            Ok(())
        }
        FaultSpec::DeviceOffline { device } => {
            network(session.network.set_device_online(device, !active), device)
        }
        FaultSpec::DeviceAddressChanged { device, ip } => network(
            session
                .network
                .set_device_ip(device, if active { Some(*ip) } else { None }),
            device,
        ),
    }
}

fn hardware(
    session: &mut LabSession,
    fault: HardwareFault,
    active: bool,
    now: simulation_core::WorldTime,
) -> Result<(), EngineError> {
    if active {
        session.station.apply_fault(fault, now);
    } else {
        session.station.clear_fault(&fault, now);
    }
    Ok(())
}

fn network(found: bool, device: &lab_model::Identifier) -> Result<(), EngineError> {
    if found {
        Ok(())
    } else {
        Err(EngineError::UnknownComponent(device.to_string()))
    }
}
