//! Platform simulation profiles: how PLC Lab presents and approximates a vendor
//! platform. Profiles are data. Generic runtime code never branches on vendor names;
//! it branches only on the typed behavior selections below.

use crate::{hardware::ModuleType, ids::Identifier};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FidelityLevel {
    /// Behavior matches documented vendor behavior for the modeled scope.
    ExactBehavioral,
    /// Behavior is representative but simplified; see the profile notes.
    ApproximatedBehavioral,
    /// Explained and visualized, but not executed with vendor semantics.
    Conceptual,
    NotImplemented,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FidelityStatement {
    pub level: FidelityLevel,
    pub modeled: Vec<String>,
    pub approximated: Vec<String>,
    pub omitted: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tier {
    /// Deeper simulation and troubleshooting coverage.
    A,
    /// Knowledge plus representative hardware behavior.
    B,
    /// Vendor-neutral PLC Lab profile.
    Generic,
}

/// How channel addresses are displayed. Display only; execution uses symbolic tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AddressingScheme {
    /// IEC 61131-3 directly represented variables, e.g. `%IX2.0`, `%IW4`.
    IecDirect,
    /// Byte.bit process image, e.g. `%I2.0`, `%IW64`.
    ByteBit,
    /// Module-defined tags, e.g. `Local:2:I.Data.0`.
    ModuleDefinedTags,
    /// Linked terminal channels, e.g. `Term 2 . Channel 1 . Input`.
    LinkedTerminal,
    /// Octal X/Y device numbers, e.g. `X20`, `Y10`.
    OctalDevices,
    /// Named NX unit I/O ports, e.g. `Unit2_Ch1`.
    UnitPorts,
}

/// Raw value convention of analog input channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnalogRepresentation {
    /// PLC Lab convention: nominal range maps to INT 0..10000.
    PerTenThousand,
    /// Nominal range maps to INT 0..27648 with over/underrange bands.
    S7Nominal27648,
    /// Nominal range maps to INT 0..32767 (16-bit signed full scale).
    FullScale32767,
    /// Channel delivers REAL engineering units (4.0 .. 20.0 for mA, 0.0 .. 10.0 for V).
    EngineeringUnits,
}

/// What happens to user memory on a STOP to RUN transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StartupBehavior {
    /// Warm restart: non-retentive variables return to their initial values.
    ReinitializeNonRetentive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductName {
    pub module: ModuleType,
    pub product: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Term {
    pub generic: String,
    pub platform: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformProfile {
    pub id: Identifier,
    pub vendor: Identifier,
    pub family: Identifier,
    pub display_name: String,
    pub tier: Tier,
    pub fidelity: FidelityStatement,
    pub addressing: AddressingScheme,
    pub analog: AnalogRepresentation,
    pub startup: StartupBehavior,
    pub products: Vec<ProductName>,
    #[serde(default)]
    pub terminology: Vec<Term>,
}

impl PlatformProfile {
    pub fn product_name(&self, module: ModuleType) -> Option<&str> {
        self.products
            .iter()
            .find(|p| p.module == module)
            .map(|p| p.product.as_str())
    }
    /// The built-in vendor-neutral profile used when no pack profile is selected.
    pub fn generic() -> Self {
        Self {
            id: Identifier::new("generic.iec").expect("static id"),
            vendor: Identifier::new("generic").expect("static id"),
            family: Identifier::new("iec-61131").expect("static id"),
            display_name: "Generic IEC 61131-3 PLC".into(),
            tier: Tier::Generic,
            fidelity: FidelityStatement {
                level: FidelityLevel::ApproximatedBehavioral,
                modeled: vec![
                    "Cyclic scan with an input image, transactional execution and output update"
                        .into(),
                    "Alpha Structured Text with BOOL, INT, REAL and TIME".into(),
                ],
                approximated: vec!["Analog raw values use the PLC Lab 0..10000 convention".into()],
                omitted: vec!["Retentive memory".into(), "Multiple tasks".into()],
            },
            addressing: AddressingScheme::IecDirect,
            analog: AnalogRepresentation::PerTenThousand,
            startup: StartupBehavior::ReinitializeNonRetentive,
            products: Vec::new(),
            terminology: Vec::new(),
        }
    }
}
