//! Opaque identities allocated by a compiled-project assembler, never by names or clocks.

use serde::Serialize;
use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidIdentity;

impl std::fmt::Display for InvalidIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Semantic identities must be nonzero.")
    }
}
impl std::error::Error for InvalidIdentity {}

macro_rules! identity {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(NonZeroU32);

        impl $name {
            pub fn new(value: u32) -> Result<Self, InvalidIdentity> {
                NonZeroU32::new(value).map(Self).ok_or(InvalidIdentity)
            }
            pub fn get(self) -> u32 {
                self.0.get()
            }
        }
    };
}

identity!(SymbolId);
identity!(PouId);
identity!(DocumentId);
identity!(SourceElementId);
identity!(ComponentId);
identity!(FunctionBlockId);
identity!(FunctionBlockInstanceId);
identity!(ParameterId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identities_reject_zero_and_preserve_boundary_values() {
        macro_rules! check {
            ($id:ident) => {
                assert_eq!($id::new(0), Err(InvalidIdentity));
                assert_eq!($id::new(1).unwrap().get(), 1);
                assert_eq!($id::new(u32::MAX).unwrap().get(), u32::MAX);
                assert_eq!(serde_json::to_value($id::new(17).unwrap()).unwrap(), 17);
            };
        }
        check!(SymbolId);
        check!(PouId);
        check!(DocumentId);
        check!(SourceElementId);
        check!(ComponentId);
        check!(FunctionBlockId);
        check!(FunctionBlockInstanceId);
        check!(ParameterId);
    }
}
