//! Process presentation DTOs only; no process ownership or stepping.
use crate::RuntimeSessionId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProcessSceneKind {
    TankFilling,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
pub enum InputAuthority {
    User,
    Process,
}

fn permille<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u16, D::Error> {
    let value = u16::deserialize(deserializer)?;
    if value <= 1000 {
        Ok(value)
    } else {
        Err(serde::de::Error::custom("Permille must be in 0..1000."))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TankProcessSnapshotDto {
    #[serde(deserialize_with = "permille")]
    pub level_permille: u16,
    pub low_level: bool,
    pub high_level: bool,
    pub inlet_valve_open: bool,
    pub drain_pump_running: bool,
    #[serde(deserialize_with = "permille")]
    pub low_threshold_permille: u16,
    #[serde(deserialize_with = "permille")]
    pub high_threshold_permille: u16,
    pub fill_rate_permille_per_second: u16,
    pub drain_rate_permille_per_second: u16,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ProcessSnapshotDto {
    #[serde(rename = "TANK_FILLING")]
    TankFilling(TankProcessSnapshotDto),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttachProcessSceneRequest {
    pub contract_version: u32,
    pub session_id: RuntimeSessionId,
    pub scene_kind: ProcessSceneKind,
}
pub(crate) fn process_typescript(config: &ts_rs::Config) -> String {
    [
        ProcessSceneKind::decl(config),
        InputAuthority::decl(config),
        TankProcessSnapshotDto::decl(config),
        ProcessSnapshotDto::decl(config),
        AttachProcessSceneRequest::decl(config),
    ]
    .into_iter()
    .map(|d| format!("\nexport {d}\n"))
    .collect()
}
