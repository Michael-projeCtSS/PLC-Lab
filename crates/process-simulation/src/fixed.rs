//! Exact fixed-point volume integration.
//!
//! Volume is stored as whole millilitres plus a remainder in units of
//! 1e-6 ml (the product of ml/s and microseconds). Integrating any partition of an
//! interval yields exactly the same result as integrating the whole interval, so the
//! process is independent of the PLC cycle or event spacing.

pub(crate) const SCALE: i128 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Volume {
    pub(crate) ml: u64,
    /// Fraction of a millilitre in 1e-6 ml units, always `0..SCALE`.
    pub(crate) remainder: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Integration {
    /// Millilitres that did not fit into the vessel.
    pub(crate) spilled_ml: u64,
    /// Offset in microseconds at which the vessel became full during the interval.
    pub(crate) full_at: Option<u64>,
    /// Offset in microseconds at which the vessel became empty during the interval.
    pub(crate) empty_at: Option<u64>,
}

fn total(volume: Volume) -> i128 {
    i128::from(volume.ml) * SCALE + i128::from(volume.remainder)
}

/// Offset (µs, rounded up) after which `rate` (ml/s) moves `from` across `threshold_ml`.
pub(crate) fn crossing(volume: Volume, rate: i64, threshold_ml: u64, dt: u64) -> Option<u64> {
    let start = total(volume);
    let target = i128::from(threshold_ml) * SCALE;
    let rate = i128::from(rate);
    let offset = if rate > 0 && start < target {
        (target - start + rate - 1) / rate
    } else if rate < 0 && start > target {
        (start - target + (-rate) - 1) / (-rate)
    } else {
        return None;
    };
    u64::try_from(offset).ok().filter(|o| *o <= dt)
}

/// Integrate a constant net `rate` (ml/s) for `dt` microseconds within `[0, capacity]`.
pub(crate) fn integrate(volume: &mut Volume, rate: i64, dt: u64, capacity_ml: u64) -> Integration {
    let mut result = Integration::default();
    if rate == 0 || dt == 0 {
        return result;
    }
    let capacity = i128::from(capacity_ml) * SCALE;
    let start = total(*volume);
    let end = start + i128::from(rate) * i128::from(dt);
    let clamped = if end > capacity {
        result.full_at = crossing(*volume, rate, capacity_ml, dt);
        let spilled = (end - capacity) / SCALE;
        result.spilled_ml = u64::try_from(spilled).unwrap_or(u64::MAX);
        capacity
    } else if end < 0 {
        result.empty_at = crossing(*volume, rate, 0, dt);
        0
    } else {
        end
    };
    volume.ml = u64::try_from(clamped / SCALE).unwrap_or(capacity_ml);
    volume.remainder = u32::try_from(clamped % SCALE).unwrap_or(0);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partitioned_integration_equals_single_step() {
        for (rate, total_us) in [
            (1000, 500_000u64),
            (-800, 500_000),
            (3, 1_000_001),
            (777, 99_999),
        ] {
            let mut whole = Volume {
                ml: 5_000,
                remainder: 0,
            };
            integrate(&mut whole, rate, total_us, 1_000_000);
            for step in [1u64, 3, 1_000, 10_000, 33_333] {
                let mut parts = Volume {
                    ml: 5_000,
                    remainder: 0,
                };
                let mut elapsed = 0;
                while elapsed < total_us {
                    let dt = step.min(total_us - elapsed);
                    integrate(&mut parts, rate, dt, 1_000_000);
                    elapsed += dt;
                }
                assert_eq!(parts, whole, "rate {rate} step {step}");
            }
        }
    }

    #[test]
    fn small_steps_do_not_lose_slow_flows() {
        // 1 ml/s for 0.8 s in 1000 steps of 0.8 ms must move exactly 0.8 ml.
        let mut v = Volume {
            ml: 10,
            remainder: 0,
        };
        for _ in 0..1000 {
            integrate(&mut v, -1, 800, 100);
        }
        assert_eq!(
            v,
            Volume {
                ml: 9,
                remainder: 200_000
            }
        );
    }

    #[test]
    fn clamping_reports_exact_crossings_and_spill() {
        let mut v = Volume {
            ml: 990,
            remainder: 0,
        };
        let r = integrate(&mut v, 100, 200_000, 1000);
        assert_eq!(
            v,
            Volume {
                ml: 1000,
                remainder: 0
            }
        );
        assert_eq!(r.full_at, Some(100_000));
        assert_eq!(r.spilled_ml, 10);
        let mut e = Volume {
            ml: 5,
            remainder: 0,
        };
        let r = integrate(&mut e, -10, 1_000_000, 1000);
        assert_eq!(e, Volume::default());
        assert_eq!(r.empty_at, Some(500_000));
        assert_eq!(
            crossing(
                Volume {
                    ml: 0,
                    remainder: 0
                },
                10,
                1,
                1_000_000
            ),
            Some(100_000)
        );
    }
}
