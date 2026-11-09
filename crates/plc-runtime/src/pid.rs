//! The PID standard block (ADR-0046): positional PID with derivative on the measurement,
//! conditional-integration anti-windup, bumpless manual/automatic transfer, output
//! limits and a sample time on the virtual task clock.
//!
//! Every value is IEEE 754 binary32 (REAL) and every operation an addition,
//! subtraction, multiplication or division in a fixed order, so the controller gives
//! bit-identical results on every platform. A non-finite intermediate value is the REAL
//! fault, an inverted output range the limits fault; neither changes the state.
use crate::{
    errors::FaultKind,
    function_blocks::{BlockError, CallInputs, elapsed},
};
use plc_types::{PlcReal, PlcTime};

/// The block's outputs (MV, E, P, I, D, SAT) and the inputs it reports for online
/// observation (SP, PV, KP, TI, TD, MANUAL, MV_MAN, ADR-0048).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PidOutputs {
    pub mv: PlcReal,
    pub e: PlcReal,
    pub p: PlcReal,
    pub i: PlcReal,
    pub d: PlcReal,
    pub sat: bool,
    pub sp: PlcReal,
    pub pv: PlcReal,
    pub kp: PlcReal,
    pub ti: PlcTime,
    pub td: PlcTime,
    pub manual: bool,
    pub mv_man: PlcReal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PidState {
    // Optional inputs keep their last bound value.
    kp: PlcReal,
    ti: PlcTime,
    td: PlcTime,
    mv_min: PlcReal,
    mv_max: PlcReal,
    manual: bool,
    mv_man: PlcReal,
    enable: bool,
    reset: bool,
    ts: PlcTime,
    // The required inputs of the last call, for observation.
    sp: PlcReal,
    pv: PlcReal,
    // Controller memory.
    integral: PlcReal,
    previous_pv: Option<PlcReal>,
    last_sample: Option<PlcTime>,
    outputs: PidOutputs,
}

fn real(value: f32) -> Result<PlcReal, BlockError> {
    PlcReal::new(value).map_err(|_| BlockError::Fault(FaultKind::NonFiniteReal))
}
fn zero() -> PlcReal {
    PlcReal::new(0.0).expect("zero is finite")
}

impl Default for PidState {
    fn default() -> Self {
        Self {
            kp: PlcReal::new(1.0).expect("finite"),
            ti: PlcTime::ZERO,
            td: PlcTime::ZERO,
            mv_min: zero(),
            mv_max: PlcReal::new(100.0).expect("finite"),
            manual: false,
            mv_man: zero(),
            enable: true,
            reset: false,
            ts: PlcTime::ZERO,
            sp: zero(),
            pv: zero(),
            integral: zero(),
            previous_pv: None,
            last_sample: None,
            outputs: PidOutputs {
                mv: zero(),
                e: zero(),
                p: zero(),
                i: zero(),
                d: zero(),
                sat: false,
                sp: zero(),
                pv: zero(),
                kp: zero(),
                ti: PlcTime::ZERO,
                td: PlcTime::ZERO,
                manual: false,
                mv_man: zero(),
            },
        }
    }
}

impl PidState {
    pub(crate) fn outputs(&self) -> PidOutputs {
        PidOutputs {
            sp: self.sp,
            pv: self.pv,
            kp: self.kp,
            ti: self.ti,
            td: self.td,
            manual: self.manual,
            mv_man: self.mv_man,
            ..self.outputs
        }
    }

    /// One call at the scan-start task time. All checks complete before any field
    /// changes.
    pub(crate) fn call(&mut self, inputs: &CallInputs<'_>, now: PlcTime) -> Result<(), BlockError> {
        let mut next = self.clone();
        next.sp = inputs.real(1, None)?;
        next.pv = inputs.real(2, None)?;
        let (sp, pv) = (next.sp.get(), next.pv.get());
        next.kp = inputs.real(3, Some(self.kp))?;
        next.ti = inputs.time_or(4, self.ti)?;
        next.td = inputs.time_or(5, self.td)?;
        next.mv_min = inputs.real(6, Some(self.mv_min))?;
        next.mv_max = inputs.real(7, Some(self.mv_max))?;
        next.manual = inputs.flag(8, self.manual)?;
        next.mv_man = inputs.real(9, Some(self.mv_man))?;
        next.enable = inputs.flag(10, self.enable)?;
        next.reset = inputs.flag(11, self.reset)?;
        next.ts = inputs.time_or(12, self.ts)?;
        let (lo, hi) = (next.mv_min.get(), next.mv_max.get());
        if lo > hi {
            return Err(BlockError::Fault(FaultKind::InvalidLimits));
        }
        let limit = |u: f32| u.clamp(lo, hi);
        let kp = next.kp.get();
        let e = real(sp - pv)?;

        if !next.enable {
            next.integral = zero();
            next.previous_pv = None;
            next.last_sample = None;
            next.outputs = PidOutputs {
                mv: real(limit(0.0))?,
                e,
                p: zero(),
                i: zero(),
                d: zero(),
                sat: false,
                ..next.outputs
            };
            *self = next;
            return Ok(());
        }
        // RESET clears the whole history: while it is TRUE the integral stays 0 and the
        // next sample is a first sample.
        if next.reset {
            next.integral = zero();
            next.previous_pv = None;
            next.last_sample = None;
        }
        if next.manual {
            // The output follows MV_MAN; the integral tracks it so automatic mode
            // continues from it without a bump.
            let p = real(kp * e.get())?;
            let mv = real(limit(next.mv_man.get()))?;
            next.integral = real(mv.get() - p.get())?;
            next.previous_pv = Some(real(pv)?);
            next.last_sample = Some(now);
            next.outputs = PidOutputs {
                mv,
                e,
                p,
                i: next.integral,
                d: zero(),
                sat: next.mv_man.get() < lo || next.mv_man.get() > hi,
                ..next.outputs
            };
            *self = next;
            return Ok(());
        }

        let since = match next.last_sample {
            Some(last) => Some(elapsed(now, last)?),
            None => None,
        };
        let due = match since {
            None => true,
            Some(us) => next.ts == PlcTime::ZERO || us >= next.ts.as_micros(),
        };
        if !due {
            *self = next;
            return Ok(());
        }
        // Microseconds of virtual time; binary32 represents every interval below about
        // 16.7 s exactly and longer ones to 7 significant digits, the same everywhere.
        let dt = since.unwrap_or(0) as f32;
        let p = real(kp * e.get())?;
        let d = match next.previous_pv {
            Some(previous) if next.td != PlcTime::ZERO && dt > 0.0 => {
                let rate = real(next.td.as_micros() as f32 / dt)?;
                real(-(real(kp * rate.get())?.get()) * real(pv - previous.get())?.get())?
            }
            _ => zero(),
        };
        let mut i = next.integral;
        if next.ti != PlcTime::ZERO && dt > 0.0 {
            let share = real(dt / next.ti.as_micros() as f32)?;
            let delta = real(real(kp * share.get())?.get() * e.get())?;
            let candidate = real(i.get() + delta.get())?;
            let u = real(real(p.get() + candidate.get())?.get() + d.get())?;
            // Conditional integration: no integrating further into a saturated output.
            let winding =
                (u.get() > hi && delta.get() > 0.0) || (u.get() < lo && delta.get() < 0.0);
            if !winding {
                i = candidate;
            }
        }
        let u = real(real(p.get() + i.get())?.get() + d.get())?;
        next.integral = i;
        next.previous_pv = Some(real(pv)?);
        next.last_sample = Some(now);
        next.outputs = PidOutputs {
            mv: real(limit(u.get()))?,
            e,
            p,
            i,
            d,
            sat: u.get() > hi || u.get() < lo,
            ..next.outputs
        };
        *self = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plc_types::PlcValue;

    const SP: u32 = 1;
    const PV: u32 = 2;
    const KP: u32 = 3;
    const TI: u32 = 4;
    const TD: u32 = 5;
    const MV_MIN: u32 = 6;
    const MV_MAX: u32 = 7;
    const MANUAL: u32 = 8;
    const MV_MAN: u32 = 9;
    const ENABLE: u32 = 10;
    const RESET: u32 = 11;
    const TS: u32 = 12;

    fn r(v: f32) -> PlcValue {
        PlcValue::Real(PlcReal::new(v).unwrap())
    }
    fn s(seconds: f64) -> PlcValue {
        PlcValue::Time(PlcTime::from_micros((seconds * 1e6) as u64))
    }
    fn at(seconds: f64) -> PlcTime {
        PlcTime::from_micros((seconds * 1e6) as u64)
    }
    fn call(pid: &mut PidState, inputs: &[(u32, PlcValue)], t: f64) -> PidOutputs {
        pid.call(&CallInputs(inputs), at(t)).unwrap();
        pid.outputs()
    }
    fn mv(o: PidOutputs) -> f32 {
        o.mv.get()
    }

    #[test]
    fn proportional_action_is_gain_times_error_within_the_limits() {
        let mut pid = PidState::default();
        let o = call(&mut pid, &[(SP, r(50.0)), (PV, r(40.0)), (KP, r(2.0))], 0.0);
        assert_eq!(
            (o.e.get(), o.p.get(), o.i.get(), o.d.get()),
            (10.0, 20.0, 0.0, 0.0)
        );
        assert_eq!((mv(o), o.sat), (20.0, false));
        // A large error saturates at MV_MAX (100 by default), a negative one at MV_MIN.
        let o = call(&mut pid, &[(SP, r(100.0)), (PV, r(0.0))], 0.1);
        assert_eq!((mv(o), o.sat), (100.0, true));
        let o = call(&mut pid, &[(SP, r(0.0)), (PV, r(10.0))], 0.2);
        assert_eq!((mv(o), o.sat), (0.0, true));
        // Reverse action through a negative gain.
        let o = call(
            &mut pid,
            &[(SP, r(20.0)), (PV, r(30.0)), (KP, r(-3.0))],
            0.3,
        );
        assert_eq!(mv(o), 30.0);
    }

    #[test]
    fn integral_action_accumulates_kp_dt_over_ti_times_error() {
        let mut pid = PidState::default();
        let inputs = [(SP, r(10.0)), (PV, r(8.0)), (KP, r(1.0)), (TI, s(2.0))];
        // The first sample has no elapsed time: no integration yet.
        assert_eq!(call(&mut pid, &inputs, 0.0).i.get(), 0.0);
        // Each 0.5 s adds 1.0 * 0.5 / 2.0 * 2.0 = 0.5.
        for (n, t) in [0.5, 1.0, 1.5, 2.0].into_iter().enumerate() {
            let o = call(&mut pid, &inputs, t);
            assert_eq!(o.i.get(), 0.5 * (n as f32 + 1.0));
            assert_eq!(mv(o), 2.0 + 0.5 * (n as f32 + 1.0));
        }
        // After TI the integral equals the proportional part: the textbook definition.
        assert_eq!(pid.outputs().i, pid.outputs().p);
    }

    #[test]
    fn the_integral_does_not_wind_up_while_the_output_is_saturated() {
        let mut pid = PidState::default();
        let far = [(SP, r(100.0)), (PV, r(0.0)), (KP, r(0.5)), (TI, s(1.0))];
        call(&mut pid, &far, 0.0);
        let mut i = 0.0;
        for k in 1..=50 {
            let o = call(&mut pid, &far, f64::from(k) * 0.1);
            assert!(o.i.get() >= i);
            i = o.i.get();
        }
        // P = 50; the integral stops once P + I passes 100 instead of growing for 5 s.
        assert!(i <= 55.0, "integral wound up to {i}");
        assert_eq!(mv(pid.outputs()), 100.0);
        // When the error reverses, the output leaves the limit at once.
        let o = call(&mut pid, &[(SP, r(0.0)), (PV, r(10.0))], 5.1);
        assert!(mv(o) < 100.0, "{o:?}");
    }

    #[test]
    fn derivative_acts_on_the_measurement_without_setpoint_kick() {
        let mut pid = PidState::default();
        let base = [(KP, r(2.0)), (TD, s(1.0))];
        call(
            &mut pid,
            &[&base[..], &[(SP, r(50.0)), (PV, r(40.0))]].concat(),
            0.0,
        );
        // A setpoint step changes P only.
        let o = call(&mut pid, &[(SP, r(60.0)), (PV, r(40.0))], 0.5);
        assert_eq!((o.p.get(), o.d.get()), (40.0, 0.0));
        // A rising measurement brakes: D = -KP * TD/dt * dPV = -2 * 2 * 1 = -4.
        let o = call(&mut pid, &[(SP, r(60.0)), (PV, r(41.0))], 1.0);
        assert_eq!(o.d.get(), -4.0);
    }

    #[test]
    fn manual_mode_drives_the_output_and_returns_to_automatic_without_a_bump() {
        let mut pid = PidState::default();
        let loop_ = [(SP, r(50.0)), (PV, r(45.0)), (KP, r(2.0)), (TI, s(10.0))];
        let manual = [(MANUAL, PlcValue::Bool(true)), (MV_MAN, r(37.0))];
        let o = call(&mut pid, &[&loop_[..], &manual[..]].concat(), 0.0);
        assert_eq!((mv(o), o.p.get(), o.i.get()), (37.0, 10.0, 27.0));
        // Back to automatic with the same error: the output continues from 37 plus only
        // what the integral adds in this sample (2 * 0.1/10 * 5 = 0.1).
        let o = call(
            &mut pid,
            &[&loop_[..], &[(MANUAL, PlcValue::Bool(false))]].concat(),
            0.1,
        );
        assert!((mv(o) - 37.1).abs() < 1e-4, "{o:?}");
        // A manual output beyond the limits is limited and reported.
        let o = call(
            &mut pid,
            &[
                &loop_[..],
                &[(MANUAL, PlcValue::Bool(true)), (MV_MAN, r(140.0))],
            ]
            .concat(),
            0.2,
        );
        assert_eq!((mv(o), o.sat), (100.0, true));
    }

    #[test]
    fn the_sample_time_holds_the_output_between_samples() {
        let mut pid = PidState::default();
        let inputs = [
            (SP, r(10.0)),
            (PV, r(0.0)),
            (KP, r(1.0)),
            (TI, s(1.0)),
            (TS, s(0.3)),
        ];
        call(&mut pid, &inputs, 0.0);
        let held = call(&mut pid, &inputs, 0.1);
        assert_eq!(held, call(&mut pid, &inputs, 0.2));
        // The sample at 0.3 s integrates the whole 0.3 s: 1 * 0.3/1 * 10 = 3.
        let o = call(&mut pid, &inputs, 0.3);
        assert!((o.i.get() - 3.0).abs() < 1e-5, "{o:?}");
    }

    #[test]
    fn enable_and_reset() {
        let mut pid = PidState::default();
        let inputs = [
            (SP, r(10.0)),
            (PV, r(5.0)),
            (KP, r(1.0)),
            (TI, s(1.0)),
            (MV_MIN, r(-20.0)),
        ];
        call(&mut pid, &inputs, 0.0);
        call(&mut pid, &inputs, 1.0);
        assert_eq!(pid.outputs().i.get(), 5.0);
        // Disabled: output 0 within the limits, parts cleared.
        let off = call(
            &mut pid,
            &[&inputs[..], &[(ENABLE, PlcValue::Bool(false))]].concat(),
            2.0,
        );
        assert_eq!((mv(off), off.i.get(), off.sat), (0.0, 0.0, false));
        // Enabled again: a clean first sample.
        let on = call(
            &mut pid,
            &[&inputs[..], &[(ENABLE, PlcValue::Bool(true))]].concat(),
            3.0,
        );
        assert_eq!((on.i.get(), mv(on)), (0.0, 5.0));
        call(&mut pid, &inputs, 4.0);
        assert_eq!(pid.outputs().i.get(), 5.0);
        // RESET clears the integral while the controller keeps running.
        let reset = call(
            &mut pid,
            &[&inputs[..], &[(RESET, PlcValue::Bool(true))]].concat(),
            5.0,
        );
        assert_eq!((reset.i.get(), mv(reset)), (0.0, 5.0));
    }

    #[test]
    fn inverted_limits_and_overflow_are_faults_that_change_nothing() {
        let mut pid = PidState::default();
        call(&mut pid, &[(SP, r(1.0)), (PV, r(0.0))], 0.0);
        let before = pid.clone();
        let inverted = pid.call(
            &CallInputs(&[
                (SP, r(1.0)),
                (PV, r(0.0)),
                (MV_MIN, r(10.0)),
                (MV_MAX, r(5.0)),
            ]),
            at(0.1),
        );
        assert!(matches!(
            inverted,
            Err(BlockError::Fault(FaultKind::InvalidLimits))
        ));
        assert_eq!(pid, before);
        let huge = pid.call(
            &CallInputs(&[(SP, r(f32::MAX)), (PV, r(-f32::MAX)), (KP, r(1.0))]),
            at(0.2),
        );
        assert!(matches!(
            huge,
            Err(BlockError::Fault(FaultKind::NonFiniteReal))
        ));
        assert_eq!(pid, before);
    }

    #[test]
    fn the_same_inputs_give_bit_identical_outputs() {
        let run = || {
            let mut pid = PidState::default();
            let mut pv = 20.0f32;
            let mut trace = Vec::new();
            for k in 0..600 {
                let o = call(
                    &mut pid,
                    &[
                        (SP, r(60.0)),
                        (PV, r(pv)),
                        (KP, r(1.7)),
                        (TI, s(12.5)),
                        (TD, s(1.3)),
                    ],
                    f64::from(k) * 0.1,
                );
                // A crude first-order plant, only to make the loop move.
                pv += (o.mv.get() * 0.8 - (pv - 20.0)) * 0.01;
                trace.push(o.mv.get().to_bits());
            }
            trace
        };
        assert_eq!(run(), run());
    }
}
