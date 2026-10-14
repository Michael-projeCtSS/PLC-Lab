//! Private-state assertions complement the immutable public Q/ET observation tests.
use super::*;
use crate::{function_blocks::InstanceState, lower};
use plc_ir::validate;
use plc_types::{
    PlcType,
    compatibility::BinaryOperator as B,
    diagnostics::{DiagnosticCategory, codes},
};
#[allow(dead_code)]
#[path = "../../plc-ir/tests/support/mod.rs"]
mod support;
use support::ton::*;

enum Tail {
    None,
    Arithmetic,
    Budget,
}
fn timer(period: PlcTime, budget: u64, tail: Tail) -> (Runtime, [SymbolId; 5]) {
    let mut f = TonFixture::new();
    let pins = [f.enable, f.preset, f.fail, f.q, f.et];
    let call = f.associated_call();
    let mut body = vec![call];
    if !matches!(tail, Tail::None) {
        let value = if matches!(tail, Tail::Arithmetic) {
            let one = f.f.constant(PlcValue::Int(1));
            let zero = f.f.constant(PlcValue::Int(0));
            f.f.binary(B::Divide, PlcType::Int, one, zero)
        } else {
            f.f.constant(PlcValue::Int(1))
        };
        let write = f.f.assign(f.result, value);
        let condition = f.f.read(f.fail);
        body.push(f.f.conditional(condition, vec![write], vec![]));
    }
    let p = f.finish(body);
    let plan = lower(&validate(&p).unwrap()).unwrap();
    (
        Runtime::new(plan, RuntimeConfig::new(period, budget).unwrap()),
        pins,
    )
}
fn preserved(r: &Runtime, before: &RuntimeSnapshot, instances: &InstanceStates) {
    let after = r.snapshot();
    assert_eq!(&r.instances, instances); // Includes activation epoch, not just public Q/ET.
    assert_eq!(after.function_blocks(), before.function_blocks());
    assert_eq!(after.committed(), before.committed());
    assert_eq!(after.outputs(), before.outputs());
    assert_eq!(after.time(), before.time());
    assert_eq!(after.completed_scans(), before.completed_scans());
    assert_eq!(after.state(), RuntimeState::Faulted);
}
#[test]
fn later_arithmetic_and_budget_faults_preserve_committed_timer_epoch_and_memory() {
    for (tail, budget, code) in [
        (Tail::Arithmetic, 20, codes::DIVISION_BY_ZERO),
        (Tail::Budget, 7, codes::EXECUTION_BUDGET),
    ] {
        let (mut r, pins) = timer(ms(100), budget, tail);
        r.single_scan().unwrap(); // Inactive at 0 ms.
        r.set_input(pins[0], PlcValue::Bool(true)).unwrap();
        r.set_input(pins[1], PlcValue::Time(PlcTime::ZERO)).unwrap();
        r.single_scan().unwrap(); // Activate at 100 ms; Q true.
        let before = r.snapshot();
        let instances = r.instances.clone();
        assert!(
            matches!(instances[&iid(11)], InstanceState::Ton { active_since: Some(epoch), q: true, .. } if epoch == ms(100))
        );
        r.set_input(pins[0], PlcValue::Bool(false)).unwrap(); // Working call clears epoch and outputs.
        r.set_input(pins[2], PlcValue::Bool(true)).unwrap();
        let error = r.single_scan().unwrap_err();
        assert_eq!(error.diagnostic().unwrap().code(), code);
        assert!(error.diagnostic().unwrap().source().is_some());
        preserved(&r, &before, &instances);
        assert_eq!(r.snapshot().input_image()[&pins[0]], PlcValue::Bool(false));
        assert_eq!(r.snapshot().input_image()[&pins[2]], PlcValue::Bool(true));
        assert!(matches!(r.single_scan(), Err(ScanError::Faulted)));
        r.reset();
        assert!(matches!(
            r.instances[&iid(11)],
            InstanceState::Ton {
                active_since: None,
                q: false,
                et: PlcTime::ZERO
            }
        ));
        assert_eq!(r.snapshot().time(), PlcTime::ZERO);
        assert_eq!(r.snapshot().completed_scans(), 0);
        assert_eq!(r.state(), RuntimeState::Ready);
    }
}
#[test]
fn budget_failure_between_state_and_each_output_write_rolls_back_the_whole_scan() {
    for budget in [5, 6] {
        let mut f = TonFixture::new();
        let (enable, fail, q, et) = (f.enable, f.fail, f.q, f.et);
        let bindings = f.bindings(None, None);
        let unassociated = f.call(iid(11), bindings);
        let associated = f.associated_call();
        let condition = f.f.read(fail);
        let branch =
            f.f.conditional(condition, vec![associated], vec![unassociated]);
        let p = f.finish(vec![branch]);
        let mut r = Runtime::new(
            lower(&validate(&p).unwrap()).unwrap(),
            RuntimeConfig::new(ms(100), budget).unwrap(),
        );
        r.single_scan().unwrap();
        r.set_input(enable, PlcValue::Bool(true)).unwrap();
        r.single_scan().unwrap(); // Nondefault committed epoch + Q without associations (5 operations).
        let before = r.snapshot();
        let instances = r.instances.clone();
        r.set_input(enable, PlcValue::Bool(false)).unwrap();
        r.set_input(fail, PlcValue::Bool(true)).unwrap();
        let error = r.single_scan().unwrap_err();
        assert_eq!(error.diagnostic().unwrap().code(), codes::EXECUTION_BUDGET);
        assert_eq!(
            error.diagnostic().unwrap().related(),
            Some(plc_types::diagnostics::RelatedIdentity::Symbol(
                if budget == 5 { q } else { et }
            ))
        );
        preserved(&r, &before, &instances);
    }
}
#[test]
fn clock_and_counter_completion_failures_rollback_timer_mutation() {
    for counter in [false, true] {
        let period = if counter {
            ms(100)
        } else {
            PlcTime::from_micros(u64::MAX)
        };
        let (mut r, pins) = timer(period, 5, Tail::None);
        r.set_input(pins[0], PlcValue::Bool(true)).unwrap();
        r.single_scan().unwrap();
        if counter {
            r.completed_scans = u64::MAX;
        }
        let before = r.snapshot();
        let instances = r.instances.clone();
        r.set_input(pins[0], PlcValue::Bool(false)).unwrap();
        let error = r.single_scan().unwrap_err();
        assert_eq!(
            error.diagnostic().unwrap().code(),
            if counter {
                codes::SCAN_COUNTER_OVERFLOW
            } else {
                codes::CLOCK_OVERFLOW
            }
        );
        preserved(&r, &before, &instances);
    }
}
#[test]
fn impossible_runtime_timer_epoch_is_a_source_backed_internal_error() {
    let (mut r, pins) = timer(ms(100), 5, Tail::None);
    r.set_input(pins[0], PlcValue::Bool(true)).unwrap();
    let InstanceState::Ton { active_since, .. } = r.instances.get_mut(&iid(11)).unwrap();
    *active_since = Some(ms(100)); // Private invariant injection; no production clock/state setter.
    let instances = r.instances.clone();
    let before = r.snapshot();
    let error = r.single_scan().unwrap_err();
    let diagnostic = error.diagnostic().unwrap();
    assert_eq!(
        diagnostic.category(),
        DiagnosticCategory::InternalApplicationError
    );
    assert_eq!(diagnostic.code(), codes::INTERNAL_INVARIANT);
    assert!(diagnostic.source().is_some());
    preserved(&r, &before, &instances);
}
