//! Network data: the physically installed network devices and the station's configured
//! interface and Modbus TCP I/O scanning. Behavior lives in `network-core`.

use crate::ids::{Identifier, TagName};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// IPv4 address written in dotted-decimal form in data files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ipv4([u8; 4]);

impl Ipv4 {
    pub fn new(octets: [u8; 4]) -> Self {
        Self(octets)
    }
    pub fn octets(self) -> [u8; 4] {
        self.0
    }
    pub fn to_u32(self) -> u32 {
        u32::from_be_bytes(self.0)
    }
    pub fn parse(text: &str) -> Option<Self> {
        let parts: Vec<&str> = text.split('.').collect();
        if parts.len() != 4 {
            return None;
        }
        let mut out = [0u8; 4];
        for (slot, part) in out.iter_mut().zip(parts) {
            if part.is_empty() || part.len() > 3 || !part.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            if part.len() > 1 && part.starts_with('0') {
                return None;
            }
            *slot = part.parse().ok()?;
        }
        Some(Self(out))
    }
    /// True when `mask` is a contiguous prefix mask.
    pub fn is_valid_mask(self) -> bool {
        let m = self.to_u32();
        m.leading_ones() + m.trailing_zeros() == 32
    }
    pub fn same_subnet(self, other: Ipv4, mask: Ipv4) -> bool {
        self.to_u32() & mask.to_u32() == other.to_u32() & mask.to_u32()
    }
}
impl std::fmt::Display for Ipv4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [a, b, c, d] = self.0;
        write!(f, "{a}.{b}.{c}.{d}")
    }
}
impl Serialize for Ipv4 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for Ipv4 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Ipv4::parse(&text).ok_or_else(|| {
            serde::de::Error::custom(format!("'{text}' is not a dotted IPv4 address"))
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterfaceConfig {
    pub ip: Ipv4,
    pub mask: Ipv4,
    #[serde(default)]
    pub gateway: Option<Ipv4>,
}

/// Physically installed network.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkInstallation {
    /// One-way frame latency on the segment, microseconds.
    pub latency_us: u32,
    #[serde(default = "connected")]
    pub plc_link_up: bool,
    pub devices: Vec<NetworkDevice>,
}
fn connected() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkDevice {
    pub id: Identifier,
    pub label: String,
    pub interface: InterfaceConfig,
    #[serde(default = "connected")]
    pub link_up: bool,
    pub role: DeviceRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum DeviceRole {
    /// A device that answers ARP but runs no Modbus server (for example an HMI panel).
    Host,
    ModbusTcpServer {
        unit_ids: UnitIdBehavior,
        processing_us: u32,
        holding_registers: Vec<RegisterBinding>,
        #[serde(default)]
        input_registers: Vec<RegisterBinding>,
    },
}

/// How a server treats the MBAP unit identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum UnitIdBehavior {
    /// Directly addressed TCP device: the unit identifier is not evaluated.
    Ignore,
    /// Gateway-style device that only answers the listed unit identifiers.
    Accept { units: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegisterBinding {
    /// Zero-based protocol register address (the PDU address, not 4xxxx notation).
    pub address: u16,
    pub value: RegisterValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RegisterValue {
    Constant {
        value: u16,
    },
    /// A component quantity multiplied by `scale` and divided by `divisor`, saturated to i16.
    ScaledInt16 {
        component: Identifier,
        quantity: String,
        scale: i32,
        divisor: i32,
    },
    /// A component quantity divided by `divisor` as IEEE 754 binary32 in two registers
    /// (`address` and `address + 1`) using the given word order.
    Float32 {
        component: Identifier,
        quantity: String,
        divisor: i32,
        word_order: WordOrder,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WordOrder {
    /// High word in the lower register address.
    HighWordFirst,
    LowWordFirst,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StationNetworkConfig {
    #[serde(default)]
    pub interface: Option<InterfaceConfig>,
    #[serde(default)]
    pub modbus_tcp: Vec<ModbusTcpScan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModbusReadFunction {
    /// Function code 03.
    ReadHoldingRegisters,
    /// Function code 04.
    ReadInputRegisters,
}
impl ModbusReadFunction {
    pub fn code(self) -> u8 {
        match self {
            Self::ReadHoldingRegisters => 3,
            Self::ReadInputRegisters => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ModbusDataType {
    Int16,
    Float32 { word_order: WordOrder },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModbusValueMapping {
    /// Offset from the request start address, in registers.
    pub offset: u16,
    pub data_type: ModbusDataType,
    pub tag: TagName,
}

/// A cyclic Modbus TCP read configured on the station (an I/O scanner line).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModbusTcpScan {
    pub id: Identifier,
    pub server: Ipv4,
    #[serde(default = "modbus_port")]
    pub port: u16,
    pub unit_id: u8,
    pub function: ModbusReadFunction,
    /// Zero-based protocol start address.
    pub start_address: u16,
    pub quantity: u16,
    pub period_ms: u32,
    pub timeout_ms: u32,
    pub values: Vec<ModbusValueMapping>,
    #[serde(default)]
    pub status_tag: Option<TagName>,
}
fn modbus_port() -> u16 {
    502
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ipv4_parsing_and_subnets_are_exact() {
        assert_eq!(
            Ipv4::parse("192.168.0.10").unwrap().octets(),
            [192, 168, 0, 10]
        );
        for bad in [
            "1.2.3",
            "1.2.3.4.5",
            "256.0.0.1",
            "01.2.3.4",
            "a.b.c.d",
            "1..2.3",
        ] {
            assert!(Ipv4::parse(bad).is_none(), "{bad}");
        }
        let mask = Ipv4::parse("255.255.255.0").unwrap();
        assert!(mask.is_valid_mask());
        assert!(!Ipv4::parse("255.0.255.0").unwrap().is_valid_mask());
        let a = Ipv4::parse("192.168.0.10").unwrap();
        assert!(a.same_subnet(Ipv4::parse("192.168.0.200").unwrap(), mask));
        assert!(!a.same_subnet(Ipv4::parse("192.168.1.20").unwrap(), mask));
    }
}
