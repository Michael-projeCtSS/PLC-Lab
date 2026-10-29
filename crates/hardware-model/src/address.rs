//! Channel address display per platform convention. Presentation only: PLC Lab
//! programs always use symbolic tags, and these strings are conceptual illustrations of
//! each platform's notation, not generated vendor configuration.

use lab_model::{
    hardware::{ChannelRef, ModuleClass, ModuleType},
    profile::AddressingScheme,
};

/// The process image bytes a module occupies in a byte-oriented image: (input image?,
/// byte range). Digital modules use one bit per channel, analog modules one word.
pub fn image_range(module: ModuleType, start: u16) -> Option<(bool, std::ops::Range<u32>)> {
    let channels = u32::from(module.channel_count());
    let start = u32::from(start);
    let (input, bytes) = match module.class() {
        ModuleClass::DigitalInput { .. } => (true, channels.div_ceil(8)),
        ModuleClass::DigitalOutput { .. } => (false, channels.div_ceil(8)),
        ModuleClass::AnalogInput { .. } => (true, 2 * channels),
        ModuleClass::AnalogOutput { .. } => (false, 2 * channels),
        _ => return None,
    };
    Some((input, start..start + bytes))
}

pub fn format(
    scheme: AddressingScheme,
    module: ModuleType,
    start: u16,
    channel: ChannelRef,
    product: Option<&str>,
) -> String {
    let ch = u32::from(channel.channel);
    let start = u32::from(start);
    let (input, analog) = match module.class() {
        ModuleClass::DigitalInput { .. } => (true, false),
        ModuleClass::DigitalOutput { .. } => (false, false),
        ModuleClass::AnalogInput { .. } => (true, true),
        ModuleClass::AnalogOutput { .. } => (false, true),
        _ => return String::new(),
    };
    match scheme {
        AddressingScheme::IecDirect => {
            let area = if input { 'I' } else { 'Q' };
            if analog {
                format!("%{area}W{}", start + 2 * ch)
            } else {
                format!("%{area}X{}.{}", start + ch / 8, ch % 8)
            }
        }
        AddressingScheme::ByteBit => {
            let area = if input { 'I' } else { 'Q' };
            if analog {
                format!("%{area}W{}", start + 2 * ch)
            } else {
                format!("%{area}{}.{}", start + ch / 8, ch % 8)
            }
        }
        AddressingScheme::ModuleDefinedTags => {
            let dir = if input { 'I' } else { 'O' };
            if analog {
                format!("Local:{}:{dir}.Ch{ch}Data", channel.slot)
            } else {
                format!("Local:{}:{dir}.Data.{ch}", channel.slot)
            }
        }
        AddressingScheme::LinkedTerminal => {
            let dir = if input { "Input" } else { "Output" };
            let name = product.unwrap_or(module.id());
            format!("Term {} ({name}).Channel {}.{dir}", channel.slot, ch + 1)
        }
        AddressingScheme::OctalDevices => {
            if analog {
                format!("U{}\\G{}", channel.slot, ch)
            } else {
                let device = if input { 'X' } else { 'Y' };
                format!("{device}{:o}", start + ch)
            }
        }
        AddressingScheme::UnitPorts => {
            let dir = if input { "In" } else { "Out" };
            format!("Unit{}_Ch{}_{dir}", channel.slot, ch + 1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ch(slot: u8, channel: u8) -> ChannelRef {
        ChannelRef {
            rack: 0,
            slot,
            channel,
        }
    }
    #[test]
    fn notations() {
        assert_eq!(
            format(
                AddressingScheme::ByteBit,
                ModuleType::Di16Sinking,
                2,
                ch(2, 9),
                None
            ),
            "%I3.1"
        );
        assert_eq!(
            format(
                AddressingScheme::IecDirect,
                ModuleType::Do8,
                0,
                ch(3, 4),
                None
            ),
            "%QX0.4"
        );
        assert_eq!(
            format(
                AddressingScheme::ByteBit,
                ModuleType::Ai4,
                64,
                ch(4, 1),
                None
            ),
            "%IW66"
        );
        assert_eq!(
            format(
                AddressingScheme::ModuleDefinedTags,
                ModuleType::Di8Sinking,
                0,
                ch(1, 3),
                None
            ),
            "Local:1:I.Data.3"
        );
        assert_eq!(
            format(
                AddressingScheme::OctalDevices,
                ModuleType::Di16Sinking,
                0,
                ch(1, 9),
                None
            ),
            "X11"
        );
        assert_eq!(
            format(
                AddressingScheme::LinkedTerminal,
                ModuleType::Do8,
                0,
                ch(2, 0),
                Some("EL2008")
            ),
            "Term 2 (EL2008).Channel 1.Output"
        );
        assert_eq!(
            format(
                AddressingScheme::UnitPorts,
                ModuleType::Ai4,
                0,
                ch(5, 0),
                None
            ),
            "Unit5_Ch1_In"
        );
    }
}
