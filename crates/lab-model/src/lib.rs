//! Versioned, data-driven PLC Lab definitions.
//!
//! This crate contains only data shapes, parsing and structural validation for scenes,
//! physical components, hardware configuration, platform profiles, network installation,
//! faults and exercises. It contains no simulation behavior: physics lives in
//! `process-simulation`, hardware behavior in `hardware-model`, network behavior in
//! `network-core`, and orchestration in `lab-engine`.

pub mod exercise;
pub mod fault;
pub mod hardware;
pub mod ids;
pub mod network;
pub mod profile;
pub mod scene;
mod validate;

pub use ids::{Identifier, IdentifierError};
pub use validate::{ModelError, validate_exercise, validate_scene, validate_station};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Current schema version for scene, exercise and station documents.
pub const SCHEMA_VERSION: u32 = 1;

/// Parse JSON text into a model type with field-level error context.
pub fn parse_json<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, ModelError> {
    serde_json::from_str(text).map_err(|e| ModelError::Parse(e.to_string()))
}
