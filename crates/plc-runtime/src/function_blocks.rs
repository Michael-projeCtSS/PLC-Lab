//! Private standard-block state, distinct from PLC symbol memory.
use crate::errors::InternalError;
use plc_types::{
    PlcTime, function_blocks::StandardFunctionBlock, identity::FunctionBlockInstanceId,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionBlockStateSnapshot {
    Ton { q: bool, et: PlcTime },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InstanceState {
    Ton {
        active_since: Option<PlcTime>,
        q: bool,
        et: PlcTime,
    },
}
pub(crate) type InstanceStates = BTreeMap<FunctionBlockInstanceId, InstanceState>;
impl InstanceState {
    pub(crate) fn new(kind: StandardFunctionBlock) -> Self {
        match kind {
            StandardFunctionBlock::Ton => Self::Ton {
                active_since: None,
                q: false,
                et: PlcTime::ZERO,
            },
        }
    }
    pub(crate) fn snapshot(&self) -> FunctionBlockStateSnapshot {
        match self {
            Self::Ton { q, et, .. } => FunctionBlockStateSnapshot::Ton { q: *q, et: *et },
        }
    }
    pub(crate) fn ton(
        &mut self,
        input: bool,
        preset: PlcTime,
        now: PlcTime,
    ) -> Result<(bool, PlcTime), InternalError> {
        let Self::Ton {
            active_since,
            q,
            et,
        } = self;
        if !input {
            *active_since = None;
            *q = false;
            *et = PlcTime::ZERO;
            return Ok((*q, *et));
        }
        let epoch = active_since.unwrap_or(now);
        let elapsed = now
            .as_micros()
            .checked_sub(epoch.as_micros())
            .ok_or_else(|| {
                InternalError::new("A timer activation time exceeds the scan-start virtual clock.")
            })?;
        *active_since = Some(epoch);
        *q = elapsed >= preset.as_micros();
        *et = PlcTime::from_micros(elapsed.min(preset.as_micros()));
        Ok((*q, *et))
    }
}
pub(crate) fn initial_states(
    instances: &BTreeMap<FunctionBlockInstanceId, StandardFunctionBlock>,
) -> InstanceStates {
    instances
        .iter()
        .map(|(id, kind)| (*id, InstanceState::new(*kind)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn impossible_clock_reversal_is_internal_and_preserves_state() {
        let mut state = InstanceState::new(StandardFunctionBlock::Ton);
        state
            .ton(true, PlcTime::from_micros(10), PlcTime::from_micros(5))
            .unwrap();
        let before = state.clone();
        let error = state
            .ton(true, PlcTime::from_micros(10), PlcTime::from_micros(4))
            .unwrap_err();
        assert_eq!(error.diagnostic().code(), "APP-CORE-001");
        assert_eq!(state, before);
    }
}
