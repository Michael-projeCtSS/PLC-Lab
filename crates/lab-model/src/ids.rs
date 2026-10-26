//! Validated textual identifiers used by data files.

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifierError(pub String);
impl std::fmt::Display for IdentifierError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "'{}' is not a valid identifier: use 1-128 lowercase ASCII letters, digits, '.', '_' or '-', starting with a letter or digit.",
            self.0
        )
    }
}
impl std::error::Error for IdentifierError {}

/// Lowercase ASCII identifier: `[a-z0-9][a-z0-9._-]{0,127}`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Identifier(String);

impl Identifier {
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        let value = value.into();
        let bytes = value.as_bytes();
        let valid = !bytes.is_empty()
            && bytes.len() <= 128
            && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
            && bytes.iter().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(IdentifierError(value))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Identifier::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// PLC tag name as written in a program: ASCII identifier, case-insensitive for lookup.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct TagName(String);
impl TagName {
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        let value = value.into();
        let mut bytes = value.bytes();
        let valid = value.len() <= 64
            && bytes
                .next()
                .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
            && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_');
        if valid {
            Ok(Self(value))
        } else {
            Err(IdentifierError(value))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn key(&self) -> String {
        self.0.to_ascii_uppercase()
    }
}
impl std::fmt::Display for TagName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for TagName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        TagName::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identifiers_are_checked() {
        for ok in ["a", "scene.tank-filling", "s7_1500", "0ab"] {
            assert!(Identifier::new(ok).is_ok(), "{ok}");
        }
        for bad in ["", "A", "-a", "a b", "ä", &"a".repeat(129)] {
            assert!(Identifier::new(bad).is_err(), "{bad}");
        }
        assert!(TagName::new("StartPB").is_ok());
        assert!(TagName::new("1x").is_err());
        assert_eq!(TagName::new("Motor").unwrap().key(), "MOTOR");
        assert!(serde_json::from_str::<Identifier>("\"Bad\"").is_err());
    }
}
