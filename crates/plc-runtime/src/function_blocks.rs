//! Private standard-block state, distinct from PLC symbol memory.
//!
//! Every block uses the scan-start PLC task time. Working copies are cloned per scan and
//! published only when the whole scan commits, so faults roll back block state too.
//! Unbound optional inputs keep their previous value (function-block inputs persist).
use crate::errors::InternalError;
use plc_types::{
    PlcTime, PlcValue,
    function_blocks::StandardFunctionBlock,
    identity::{FunctionBlockInstanceId, ParameterId},
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionBlockStateSnapshot {
    Ton { q: bool, et: PlcTime },
    Tof { q: bool, et: PlcTime },
    Tp { q: bool, et: PlcTime },
    RTrig { q: bool },
    FTrig { q: bool },
    Ctu { q: bool, cv: i16 },
    Ctd { q: bool, cv: i16 },
}
impl FunctionBlockStateSnapshot {
    pub fn kind(self) -> StandardFunctionBlock {
        match self {
            Self::Ton { .. } => StandardFunctionBlock::Ton,
            Self::Tof { .. } => StandardFunctionBlock::Tof,
            Self::Tp { .. } => StandardFunctionBlock::Tp,
            Self::RTrig { .. } => StandardFunctionBlock::RTrig,
            Self::FTrig { .. } => StandardFunctionBlock::FTrig,
            Self::Ctu { .. } => StandardFunctionBlock::Ctu,
            Self::Ctd { .. } => StandardFunctionBlock::Ctd,
        }
    }
    /// Public outputs in signature order.
    pub fn outputs(self) -> Vec<(&'static str, PlcValue)> {
        match self {
            Self::Ton { q, et } | Self::Tof { q, et } | Self::Tp { q, et } => {
                vec![("Q", PlcValue::Bool(q)), ("ET", PlcValue::Time(et))]
            }
            Self::RTrig { q } | Self::FTrig { q } => vec![("Q", PlcValue::Bool(q))],
            Self::Ctu { q, cv } | Self::Ctd { q, cv } => vec![("Q", PlcValue::Bool(q)), ("CV", PlcValue::Int(cv))],
        }
    }
    pub fn output(self, parameter: ParameterId) -> Option<PlcValue> {
        let name = self
            .kind()
            .signature()
            .iter()
            .find(|p| p.id == parameter.get())?
            .name;
        self.outputs().into_iter().find(|(n, _)| *n == name).map(|(_, v)| v)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InstanceState {
    Ton { active_since: Option<PlcTime>, q: bool, et: PlcTime },
    Tof { off_since: Option<PlcTime>, q: bool, et: PlcTime },
    Tp { start: Option<PlcTime>, previous: bool, q: bool, et: PlcTime },
    RTrig { previous: bool, q: bool },
    FTrig { previous: bool, q: bool },
    Ctu { previous: bool, reset: bool, preset: i16, q: bool, cv: i16 },
    Ctd { previous: bool, load: bool, preset: i16, q: bool, cv: i16 },
}
pub(crate) type InstanceStates = BTreeMap<FunctionBlockInstanceId, InstanceState>;

fn elapsed(now: PlcTime, since: PlcTime) -> Result<u64, InternalError> {
    now.as_micros()
        .checked_sub(since.as_micros())
        .ok_or_else(|| InternalError::new("A timer activation time exceeds the scan-start virtual clock."))
}

/// Bound input values of one call, by signature parameter ID.
pub(crate) struct CallInputs<'a>(pub(crate) &'a [(u32, PlcValue)]);
impl CallInputs<'_> {
    fn get(&self, id: u32) -> Option<PlcValue> {
        self.0.iter().find(|(p, _)| *p == id).map(|(_, v)| *v)
    }
    fn bool(&self, id: u32, fallback: bool) -> Result<bool, InternalError> {
        match self.get(id) {
            None => Ok(fallback),
            Some(PlcValue::Bool(b)) => Ok(b),
            Some(_) => Err(InternalError::new("A validated block input has an incompatible type.")),
        }
    }
    fn time(&self, id: u32) -> Result<PlcTime, InternalError> {
        match self.get(id) {
            Some(PlcValue::Time(t)) => Ok(t),
            _ => Err(InternalError::new("A validated timer preset is missing or mistyped.")),
        }
    }
    fn int(&self, id: u32, fallback: i16) -> Result<i16, InternalError> {
        match self.get(id) {
            None => Ok(fallback),
            Some(PlcValue::Int(i)) => Ok(i),
            Some(_) => Err(InternalError::new("A validated counter input has an incompatible type.")),
        }
    }
}

impl InstanceState {
    pub(crate) fn new(kind: StandardFunctionBlock) -> Self {
        match kind {
            StandardFunctionBlock::Ton => Self::Ton { active_since: None, q: false, et: PlcTime::ZERO },
            StandardFunctionBlock::Tof => Self::Tof { off_since: None, q: false, et: PlcTime::ZERO },
            StandardFunctionBlock::Tp => Self::Tp { start: None, previous: false, q: false, et: PlcTime::ZERO },
            StandardFunctionBlock::RTrig => Self::RTrig { previous: false, q: false },
            // Previous CLK starts FALSE, so a FALSE input at the first call is not an edge.
            StandardFunctionBlock::FTrig => Self::FTrig { previous: false, q: false },
            StandardFunctionBlock::Ctu => Self::Ctu { previous: false, reset: false, preset: 0, q: false, cv: 0 },
            StandardFunctionBlock::Ctd => Self::Ctd { previous: false, load: false, preset: 0, q: true, cv: 0 },
        }
    }
    pub(crate) fn snapshot(&self) -> FunctionBlockStateSnapshot {
        match *self {
            Self::Ton { q, et, .. } => FunctionBlockStateSnapshot::Ton { q, et },
            Self::Tof { q, et, .. } => FunctionBlockStateSnapshot::Tof { q, et },
            Self::Tp { q, et, .. } => FunctionBlockStateSnapshot::Tp { q, et },
            Self::RTrig { q, .. } => FunctionBlockStateSnapshot::RTrig { q },
            Self::FTrig { q, .. } => FunctionBlockStateSnapshot::FTrig { q },
            Self::Ctu { q, cv, .. } => FunctionBlockStateSnapshot::Ctu { q, cv },
            Self::Ctd { q, cv, .. } => FunctionBlockStateSnapshot::Ctd { q, cv },
        }
    }
    pub(crate) fn output(&self, parameter: ParameterId) -> Option<PlcValue> {
        self.snapshot().output(parameter)
    }

    /// Execute one call. All checks complete before any field changes.
    pub(crate) fn call(&mut self, inputs: CallInputs<'_>, now: PlcTime) -> Result<(), InternalError> {
        match self {
            Self::Ton { active_since, q, et } => {
                let input = inputs.bool(1, false)?;
                let preset = inputs.time(2)?;
                if !input {
                    *active_since = None;
                    *q = false;
                    *et = PlcTime::ZERO;
                    return Ok(());
                }
                let epoch = active_since.unwrap_or(now);
                let e = elapsed(now, epoch)?;
                *active_since = Some(epoch);
                *q = e >= preset.as_micros();
                *et = PlcTime::from_micros(e.min(preset.as_micros()));
            }
            Self::Tof { off_since, q, et } => {
                let input = inputs.bool(1, false)?;
                let preset = inputs.time(2)?;
                if input {
                    *off_since = None;
                    *q = true;
                    *et = PlcTime::ZERO;
                } else if *q {
                    let since = off_since.unwrap_or(now);
                    let e = elapsed(now, since)?;
                    if e >= preset.as_micros() {
                        *q = false;
                        *off_since = None;
                        *et = preset;
                    } else {
                        *off_since = Some(since);
                        *et = PlcTime::from_micros(e);
                    }
                }
            }
            Self::Tp { start, previous, q, et } => {
                let input = inputs.bool(1, false)?;
                let preset = inputs.time(2)?;
                let rising = input && !*previous;
                if start.is_none() && rising {
                    *start = Some(now);
                }
                if let Some(s) = *start {
                    let e = elapsed(now, s)?;
                    if e >= preset.as_micros() {
                        *start = None;
                        *q = false;
                        *et = preset;
                    } else {
                        *q = true;
                        *et = PlcTime::from_micros(e);
                    }
                }
                if start.is_none() && !input {
                    *et = PlcTime::ZERO;
                }
                *previous = input;
            }
            Self::RTrig { previous, q } => {
                let clk = inputs.bool(1, false)?;
                *q = clk && !*previous;
                *previous = clk;
            }
            Self::FTrig { previous, q } => {
                let clk = inputs.bool(1, false)?;
                *q = !clk && *previous;
                *previous = clk;
            }
            Self::Ctu { previous, reset, preset, q, cv } => {
                let cu = inputs.bool(1, false)?;
                let r = inputs.bool(2, *reset)?;
                let pv = inputs.int(3, *preset)?;
                if r {
                    *cv = 0;
                } else if cu && !*previous && *cv < i16::MAX {
                    *cv += 1;
                }
                *previous = cu;
                *reset = r;
                *preset = pv;
                *q = *cv >= pv;
            }
            Self::Ctd { previous, load, preset, q, cv } => {
                let cd = inputs.bool(1, false)?;
                let ld = inputs.bool(2, *load)?;
                let pv = inputs.int(3, *preset)?;
                if ld {
                    *cv = pv;
                } else if cd && !*previous && *cv > i16::MIN {
                    *cv -= 1;
                }
                *previous = cd;
                *load = ld;
                *preset = pv;
                *q = *cv <= 0;
            }
        }
        Ok(())
    }
}

pub(crate) fn initial_states(instances: &BTreeMap<FunctionBlockInstanceId, StandardFunctionBlock>) -> InstanceStates {
    instances
        .iter()
        .map(|(id, kind)| (*id, InstanceState::new(*kind)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ms(v: u64) -> PlcTime {
        PlcTime::from_micros(v * 1000)
    }
    fn timer(state: &mut InstanceState, input: bool, pt: u64, now: u64) -> (bool, u64) {
        state
            .call(CallInputs(&[(1, PlcValue::Bool(input)), (2, PlcValue::Time(ms(pt)))]), ms(now))
            .unwrap();
        match state.snapshot() {
            FunctionBlockStateSnapshot::Ton { q, et }
            | FunctionBlockStateSnapshot::Tof { q, et }
            | FunctionBlockStateSnapshot::Tp { q, et } => (q, et.as_micros() / 1000),
            _ => panic!("timer expected"),
        }
    }

    #[test]
    fn impossible_clock_reversal_is_internal_and_preserves_state() {
        let mut state = InstanceState::new(StandardFunctionBlock::Ton);
        timer(&mut state, true, 10, 5);
        let before = state.clone();
        let error = state
            .call(CallInputs(&[(1, PlcValue::Bool(true)), (2, PlcValue::Time(ms(10)))]), ms(4))
            .unwrap_err();
        assert_eq!(error.diagnostic().code(), "APP-CORE-001");
        assert_eq!(state, before);
    }

    #[test]
    fn tof_holds_q_for_preset_after_falling_input() {
        let mut s = InstanceState::new(StandardFunctionBlock::Tof);
        assert_eq!(timer(&mut s, false, 300, 0), (false, 0), "no output without a prior TRUE input");
        assert_eq!(timer(&mut s, true, 300, 100), (true, 0));
        assert_eq!(timer(&mut s, false, 300, 200), (true, 0));
        assert_eq!(timer(&mut s, false, 300, 300), (true, 100));
        assert_eq!(timer(&mut s, false, 300, 500), (false, 300));
        assert_eq!(timer(&mut s, false, 300, 600), (false, 300));
        assert_eq!(timer(&mut s, true, 300, 700), (true, 0));
    }

    #[test]
    fn tp_is_a_non_retriggerable_pulse() {
        let mut s = InstanceState::new(StandardFunctionBlock::Tp);
        assert_eq!(timer(&mut s, true, 300, 0), (true, 0));
        assert_eq!(timer(&mut s, false, 300, 100), (true, 100));
        assert_eq!(timer(&mut s, true, 300, 200), (true, 200), "a new edge during the pulse is ignored");
        assert_eq!(timer(&mut s, true, 300, 300), (false, 300));
        assert_eq!(timer(&mut s, true, 300, 400), (false, 300), "ET holds while IN stays TRUE");
        assert_eq!(timer(&mut s, false, 300, 500), (false, 0));
        assert_eq!(timer(&mut s, true, 300, 600), (true, 0));
    }

    #[test]
    fn edge_triggers_and_counters() {
        let mut r = InstanceState::new(StandardFunctionBlock::RTrig);
        let mut f = InstanceState::new(StandardFunctionBlock::FTrig);
        let mut seen = Vec::new();
        for clk in [false, true, true, false, false, true] {
            r.call(CallInputs(&[(1, PlcValue::Bool(clk))]), PlcTime::ZERO).unwrap();
            f.call(CallInputs(&[(1, PlcValue::Bool(clk))]), PlcTime::ZERO).unwrap();
            seen.push((r.output(ParameterId::new(2).unwrap()), f.output(ParameterId::new(2).unwrap())));
        }
        let b = |x| Some(PlcValue::Bool(x));
        assert_eq!(
            seen,
            [(b(false), b(false)), (b(true), b(false)), (b(false), b(false)), (b(false), b(true)), (b(false), b(false)), (b(true), b(false))]
        );
        let mut c = InstanceState::new(StandardFunctionBlock::Ctu);
        let pulse = |c: &mut InstanceState, cu: bool| {
            c.call(CallInputs(&[(1, PlcValue::Bool(cu)), (3, PlcValue::Int(2))]), PlcTime::ZERO).unwrap();
            c.snapshot()
        };
        pulse(&mut c, true);
        pulse(&mut c, true);
        pulse(&mut c, false);
        assert_eq!(pulse(&mut c, true), FunctionBlockStateSnapshot::Ctu { q: true, cv: 2 });
        c.call(CallInputs(&[(1, PlcValue::Bool(false)), (2, PlcValue::Bool(true)), (3, PlcValue::Int(2))]), PlcTime::ZERO)
            .unwrap();
        assert_eq!(c.snapshot(), FunctionBlockStateSnapshot::Ctu { q: false, cv: 0 });
        // R persists when a later call omits it.
        assert_eq!(pulse(&mut c, true), FunctionBlockStateSnapshot::Ctu { q: false, cv: 0 });
        let mut d = InstanceState::new(StandardFunctionBlock::Ctd);
        d.call(CallInputs(&[(1, PlcValue::Bool(false)), (2, PlcValue::Bool(true)), (3, PlcValue::Int(2))]), PlcTime::ZERO)
            .unwrap();
        assert_eq!(d.snapshot(), FunctionBlockStateSnapshot::Ctd { q: false, cv: 2 });
        for cd in [true, false, true] {
            d.call(CallInputs(&[(1, PlcValue::Bool(cd)), (2, PlcValue::Bool(false)), (3, PlcValue::Int(2))]), PlcTime::ZERO)
                .unwrap();
        }
        assert_eq!(d.snapshot(), FunctionBlockStateSnapshot::Ctd { q: true, cv: 0 });
    }
}
