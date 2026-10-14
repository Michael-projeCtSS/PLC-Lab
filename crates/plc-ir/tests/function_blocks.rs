#[allow(dead_code)]
mod support;
use plc_ir::{CallBinding, FunctionBlockCall, validate};
use plc_types::{
    PlcType, PlcValue,
    diagnostics::{Diagnostic, DiagnosticCategory, RelatedIdentity, Severity, codes},
    function_blocks::{
        FunctionBlockCatalog, FunctionBlockDeclarationError, FunctionBlockDefinition,
        FunctionBlockInstance, TonParameter,
    },
    identity::{ParameterId, PouId, SymbolId},
};
use support::{entry, ton::*};

fn errors(mut f: TonFixture, bindings: Vec<CallBinding>) -> Vec<Diagnostic> {
    let call = f.call(iid(11), bindings);
    let source = call.source();
    let program = f.finish(vec![call]);
    let before = program.clone();
    let errors = validate(&program).unwrap_err();
    assert_eq!(errors, validate(&program).unwrap_err());
    assert_eq!(program, before);
    for diagnostic in &errors {
        assert_eq!(
            diagnostic.category(),
            DiagnosticCategory::CompilerDiagnostic
        );
        assert_eq!(diagnostic.severity(), Severity::Error);
        assert!(!diagnostic.message().trim().is_empty());
    }
    assert!(
        errors
            .iter()
            .any(|e| e.source().is_some_and(|s| s.reference() == source))
    );
    errors
}
fn assert_code(errors: &[Diagnostic], code: &str) {
    assert!(errors.iter().any(|e| e.code() == code), "{errors:?}");
}
#[test]
fn required_inputs_are_each_mandatory_and_not_retained() {
    for missing in [TonParameter::In, TonParameter::Pt] {
        let mut f = TonFixture::new();
        let mut bindings = f.bindings(None, None);
        bindings.retain(|b| b.parameter() != missing.id());
        let errors = errors(f, bindings);
        assert_eq!(errors.len(), 1);
        assert_code(&errors, codes::MISSING_FB_INPUT);
        assert_eq!(
            errors[0].related(),
            Some(RelatedIdentity::FunctionBlockInstance(iid(11)))
        );
    }
}
#[test]
fn input_types_require_bool_and_time_without_coercion() {
    for (parameter, value) in [
        (TonParameter::In, PlcValue::Int(1)),
        (TonParameter::Pt, PlcValue::Bool(true)),
    ] {
        let mut f = TonFixture::new();
        let mut bindings = f.bindings(None, None);
        bindings.retain(|b| b.parameter() != parameter.id());
        bindings.push(CallBinding::input(parameter.id(), f.f.constant(value)));
        let errors = errors(f, bindings);
        assert_eq!(errors.len(), 1);
        assert_code(&errors, codes::FB_PARAMETER_TYPE);
    }
}
#[test]
fn output_types_require_bool_and_time_without_coercion() {
    for parameter in [TonParameter::Q, TonParameter::Et] {
        let mut f = TonFixture::new();
        let target = if parameter == TonParameter::Q {
            f.et
        } else {
            f.q
        };
        let mut bindings = f.bindings(None, None);
        bindings.push(CallBinding::output(parameter.id(), target));
        let errors = errors(f, bindings);
        assert_eq!(errors.len(), 1);
        assert_code(&errors, codes::FB_PARAMETER_TYPE);
        assert_eq!(errors[0].related(), Some(RelatedIdentity::Symbol(target)));
    }
}
#[test]
fn both_outputs_reject_entry_input_targets() {
    for parameter in [TonParameter::Q, TonParameter::Et] {
        let mut f = TonFixture::new();
        let target = if parameter == TonParameter::Q {
            f.enable
        } else {
            f.preset
        };
        let mut bindings = f.bindings(None, None);
        bindings.push(CallBinding::output(parameter.id(), target));
        let errors = errors(f, bindings);
        assert_eq!(errors.len(), 1);
        assert_code(&errors, codes::INVALID_WRITE_TARGET);
    }
}
#[test]
fn output_target_must_be_declared_and_visible() {
    let mut f = TonFixture::new();
    let mut bindings = f.bindings(None, None);
    bindings.push(CallBinding::output(
        TonParameter::Q.id(),
        SymbolId::new(999).unwrap(),
    ));
    assert_code(&errors(f, bindings), codes::UNKNOWN_SYMBOL);
    let mut f = TonFixture::new();
    let target = SymbolId::new(999).unwrap();
    let owner = PouId::new(2).unwrap();
    let bindings = {
        let mut b = f.bindings(None, None);
        b.push(CallBinding::output(TonParameter::Q.id(), target));
        b
    };
    let call = f.call(iid(11), bindings);
    let p = f.finish(vec![call]);
    let mut symbols = p.symbols().clone();
    symbols
        .declare_pou(
            plc_types::symbols::Pou::new(owner, "Other", plc_types::symbols::PouKind::Program)
                .unwrap(),
        )
        .unwrap();
    symbols
        .declare(
            plc_types::symbols::Symbol::new(
                target,
                "Private",
                PlcType::Bool,
                plc_types::symbols::Scope::Pou(owner),
                plc_types::symbols::VariableCategory::Output,
                None,
            )
            .unwrap(),
        )
        .unwrap();
    let p = plc_ir::Program::new(symbols, p.sources().clone(), p.entry(), p.body().to_vec())
        .with_function_blocks(p.function_blocks().clone());
    assert_code(&validate(&p).unwrap_err(), codes::INVALID_SCOPE);
}
#[test]
fn unknown_instance_is_source_backed() {
    let mut f = TonFixture::new();
    let bindings = f.bindings(None, None);
    let call = f.call(iid(99), bindings);
    let source = call.source();
    let errors = validate(&f.finish(vec![call])).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_code(&errors, codes::UNKNOWN_FB_INSTANCE);
    assert_eq!(errors[0].source().unwrap().reference(), source);
    assert_eq!(
        errors[0].related(),
        Some(RelatedIdentity::FunctionBlockInstance(iid(99)))
    );
}
#[test]
fn missing_definition_is_checked_on_declaration_and_call() {
    let mut f = TonFixture::new();
    let instance_source = f.f.anchor();
    f.catalog = FunctionBlockCatalog::new();
    f.catalog
        .declare_instance(
            FunctionBlockInstance::new(iid(11), did(999), entry(), "Timer", Some(instance_source))
                .unwrap(),
        )
        .unwrap();
    let bindings = f.bindings(None, None);
    let errors = errors(f, bindings);
    assert_eq!(errors.len(), 2);
    assert!(
        errors
            .iter()
            .all(|d| d.code() == codes::UNKNOWN_FB_DEFINITION)
    );
    assert_eq!(errors[0].source().unwrap().reference(), instance_source);
    assert_eq!(
        errors[0].related(),
        Some(RelatedIdentity::FunctionBlockDefinition(did(999)))
    );
}
#[test]
fn catalog_cannot_rebind_an_instance_to_a_competing_definition() {
    let mut f = TonFixture::new();
    let before = f.catalog.clone();
    let error = f
        .catalog
        .declare_instance(
            FunctionBlockInstance::new(iid(11), did(1), entry(), "Other", None).unwrap(),
        )
        .unwrap_err();
    assert_eq!(error, FunctionBlockDeclarationError::DuplicateIdentity);
    assert_eq!(error.diagnostic().code(), codes::DUPLICATE_IDENTITY);
    assert_eq!(f.catalog, before);
    let call = f.associated_call();
    assert!(validate(&f.finish(vec![call])).is_ok());
}
#[test]
fn unknown_parameter_does_not_satisfy_a_required_input() {
    let mut f = TonFixture::new();
    let mut bindings = f.bindings(None, None);
    bindings.remove(0);
    bindings.push(CallBinding::input(
        ParameterId::new(99).unwrap(),
        f.f.constant(PlcValue::Bool(true)),
    ));
    let errors = errors(f, bindings);
    assert_code(&errors, codes::UNKNOWN_FB_PARAMETER);
    assert_code(&errors, codes::MISSING_FB_INPUT);
}
#[test]
fn wrong_directions_and_reserved_inout_are_rejected() {
    for parameter in [
        TonParameter::In,
        TonParameter::Pt,
        TonParameter::Q,
        TonParameter::Et,
    ] {
        let mut f = TonFixture::new();
        let mut bindings = f.bindings(None, None);
        bindings.retain(|b| b.parameter() != parameter.id());
        let wrong = match parameter {
            TonParameter::In | TonParameter::Pt => CallBinding::output(parameter.id(), f.q),
            TonParameter::Q | TonParameter::Et => {
                CallBinding::input(parameter.id(), f.f.constant(PlcValue::Bool(false)))
            }
        };
        bindings.push(wrong);
        assert_code(&errors(f, bindings), codes::FB_PARAMETER_DIRECTION);
    }
    let mut f = TonFixture::new();
    let mut bindings = f.bindings(None, None);
    bindings.push(CallBinding::in_out(TonParameter::Q.id(), f.q));
    assert_code(&errors(f, bindings), codes::FB_PARAMETER_DIRECTION);
}
#[test]
fn duplicate_bindings_are_rejected_across_all_directions() {
    let mut f = TonFixture::new();
    for duplicate in [
        CallBinding::input(TonParameter::In.id(), f.f.constant(PlcValue::Bool(false))),
        CallBinding::output(TonParameter::In.id(), f.q),
        CallBinding::in_out(TonParameter::In.id(), f.q),
    ] {
        let mut bindings = f.bindings(None, None);
        bindings.push(duplicate);
        let error = FunctionBlockCall::new(iid(11), bindings).unwrap_err();
        assert_eq!(error.diagnostic().code(), codes::DUPLICATE_FB_BINDING);
    }
}
#[test]
fn user_kind_is_unsupported_even_with_ton_signature_and_numeric_id_one() {
    let mut f = TonFixture::new();
    f.catalog = FunctionBlockCatalog::new();
    let signature = FunctionBlockDefinition::ton(did(1)).parameters().to_vec();
    f.catalog
        .declare_definition(FunctionBlockDefinition::user_defined(did(1), signature).unwrap())
        .unwrap();
    f.catalog
        .declare_instance(
            FunctionBlockInstance::new(iid(11), did(1), entry(), "Custom", None).unwrap(),
        )
        .unwrap();
    let bindings = f.bindings(None, None);
    let errors = errors(f, bindings);
    assert_eq!(errors.len(), 1);
    assert_code(&errors, codes::UNSUPPORTED_FB_KIND);
}
#[test]
fn instance_owner_and_declaration_sources_are_validated() {
    let mut f = TonFixture::new();
    f.catalog = FunctionBlockCatalog::new();
    f.catalog
        .declare_definition(FunctionBlockDefinition::ton(did(77)))
        .unwrap();
    f.catalog
        .declare_instance(
            FunctionBlockInstance::new(iid(11), did(77), PouId::new(2).unwrap(), "Other", None)
                .unwrap(),
        )
        .unwrap();
    let bindings = f.bindings(None, None);
    assert_code(&errors(f, bindings), codes::INVALID_FB_OWNER);
    let mut f = TonFixture::new();
    let bad_source = support::source(1, 999);
    f.catalog
        .declare_definition(FunctionBlockDefinition::ton(did(88)).with_source(bad_source))
        .unwrap();
    let call = f.associated_call();
    assert_code(
        &validate(&f.finish(vec![call])).unwrap_err(),
        codes::UNKNOWN_SOURCE,
    );
}
#[test]
fn valid_reversed_bindings_and_omitted_outputs_validate() {
    for associate in [false, true] {
        let mut f = TonFixture::new();
        let mut bindings = f.bindings(associate.then_some(f.q), associate.then_some(f.et));
        bindings.reverse();
        let call = f.call(iid(11), bindings);
        assert!(validate(&f.finish(vec![call])).is_ok());
    }
}
