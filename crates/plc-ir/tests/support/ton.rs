//! Shared test data only; uses the real catalog, validator and runtime consumers.
use super::{Fixture, entry};
use plc_ir::{CallBinding, Expression, FunctionBlockCall, Program, Statement};
use plc_types::{
    PlcTime, PlcType, PlcValue,
    function_blocks::{
        FunctionBlockCatalog, FunctionBlockDefinition, FunctionBlockInstance, TonParameter,
    },
    identity::{FunctionBlockId, FunctionBlockInstanceId, SymbolId},
    symbols::VariableCategory::{Input, Output},
};

pub fn iid(n: u32) -> FunctionBlockInstanceId {
    FunctionBlockInstanceId::new(n).unwrap()
}
pub fn did(n: u32) -> FunctionBlockId {
    FunctionBlockId::new(n).unwrap()
}
pub fn ms(n: u64) -> PlcTime {
    PlcTime::from_micros(n * 1_000)
}
pub struct TonFixture {
    pub f: Fixture,
    pub catalog: FunctionBlockCatalog,
    pub enable: SymbolId,
    pub preset: SymbolId,
    pub invoke: SymbolId,
    pub fail: SymbolId,
    pub q: SymbolId,
    pub et: SymbolId,
    pub seen_q: SymbolId,
    pub seen_et: SymbolId,
    pub result: SymbolId,
}
impl TonFixture {
    pub fn new() -> Self {
        let mut f = Fixture::new();
        let enable = f.declare("Enable", PlcType::Bool, Input);
        let preset = f.declare("Preset", PlcType::Time, Input);
        let invoke = f.declare("Invoke", PlcType::Bool, Input);
        let fail = f.declare("Fail", PlcType::Bool, Input);
        let q = f.declare("Q", PlcType::Bool, Output);
        let et = f.declare("ET", PlcType::Time, Output);
        let seen_q = f.declare("SeenQ", PlcType::Bool, Output);
        let seen_et = f.declare("SeenET", PlcType::Time, Output);
        let result = f.declare("Result", PlcType::Int, Output);
        let mut catalog = FunctionBlockCatalog::new();
        catalog
            .declare_definition(FunctionBlockDefinition::ton(did(77)))
            .unwrap();
        catalog
            .declare_instance(
                FunctionBlockInstance::new(iid(11), did(77), entry(), "Timer", Some(f.anchor()))
                    .unwrap(),
            )
            .unwrap();
        Self {
            f,
            catalog,
            enable,
            preset,
            invoke,
            fail,
            q,
            et,
            seen_q,
            seen_et,
            result,
        }
    }
    pub fn bindings(&mut self, q: Option<SymbolId>, et: Option<SymbolId>) -> Vec<CallBinding> {
        let mut bindings = vec![
            CallBinding::input(TonParameter::In.id(), self.f.read(self.enable)),
            CallBinding::input(TonParameter::Pt.id(), self.f.read(self.preset)),
        ];
        if let Some(q) = q {
            bindings.push(CallBinding::output(TonParameter::Q.id(), q));
        }
        if let Some(et) = et {
            bindings.push(CallBinding::output(TonParameter::Et.id(), et));
        }
        bindings
    }
    pub fn call(
        &mut self,
        instance: FunctionBlockInstanceId,
        bindings: Vec<CallBinding>,
    ) -> Statement {
        Statement::function_block_call(
            self.f.anchor(),
            FunctionBlockCall::new(instance, bindings).unwrap(),
        )
    }
    pub fn associated_call(&mut self) -> Statement {
        let bindings = self.bindings(Some(self.q), Some(self.et));
        self.call(iid(11), bindings)
    }
    pub fn constant_call(
        &mut self,
        input: bool,
        preset: PlcTime,
        q: Option<SymbolId>,
        et: Option<SymbolId>,
    ) -> Statement {
        let mut call = vec![
            CallBinding::input(
                TonParameter::In.id(),
                self.f.constant(PlcValue::Bool(input)),
            ),
            CallBinding::input(
                TonParameter::Pt.id(),
                self.f.constant(PlcValue::Time(preset)),
            ),
        ];
        if let Some(q) = q {
            call.push(CallBinding::output(TonParameter::Q.id(), q));
        }
        if let Some(et) = et {
            call.push(CallBinding::output(TonParameter::Et.id(), et));
        }
        self.call(iid(11), call)
    }
    pub fn finish(self, body: Vec<Statement>) -> Program {
        self.f.finish(body).with_function_blocks(self.catalog)
    }
    pub fn call_with(&mut self, input: Expression, preset: Expression) -> Statement {
        self.call(
            iid(11),
            vec![
                CallBinding::input(TonParameter::In.id(), input),
                CallBinding::input(TonParameter::Pt.id(), preset),
            ],
        )
    }
}
