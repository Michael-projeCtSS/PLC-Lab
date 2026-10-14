//! Small test-only assembler. No parser, evaluator, or production builder framework.

#[allow(dead_code)]
pub mod ton;

use plc_ir::{Expression, Program, Statement};
use plc_types::{
    PlcType, PlcValue,
    compatibility::{BinaryOperator, UnaryOperator},
    identity::{DocumentId, PouId, SourceElementId, SymbolId},
    source::{SourceLanguage, SourceLocation, SourceMap, SourcePosition, SourceRef, TextSpan},
    symbols::{Pou, PouKind, Scope, Symbol, SymbolTable, VariableCategory},
};

pub struct Fixture {
    symbols: SymbolTable,
    sources: SourceMap,
    next_symbol: u32,
    next_source: u32,
    language: SourceLanguage,
}
pub fn entry() -> PouId {
    PouId::new(1).unwrap()
}
pub fn sid(id: u32) -> SymbolId {
    SymbolId::new(id).unwrap()
}
pub fn source(document: u32, element: u32) -> SourceRef {
    SourceRef::new(
        DocumentId::new(document).unwrap(),
        SourceElementId::new(element).unwrap(),
    )
}
impl Fixture {
    pub fn new() -> Self {
        Self::language(SourceLanguage::St)
    }
    pub fn language(language: SourceLanguage) -> Self {
        let mut symbols = SymbolTable::new();
        symbols
            .declare_pou(Pou::new(entry(), "Main", PouKind::Program).unwrap())
            .unwrap();
        let mut sources = SourceMap::new();
        sources
            .register_document(DocumentId::new(1).unwrap(), language)
            .unwrap();
        Self {
            symbols,
            sources,
            next_symbol: 1,
            next_source: 1,
            language,
        }
    }
    pub fn anchor(&mut self) -> SourceRef {
        let reference = source(1, self.next_source);
        self.next_source += 1;
        let span = matches!(self.language, SourceLanguage::St | SourceLanguage::Stl).then(|| {
            let point = SourcePosition::new(reference.element_id().get(), 1).unwrap();
            TextSpan::new(point, point).unwrap()
        });
        self.sources
            .register_element(SourceLocation::new(reference, self.language, span))
            .unwrap();
        reference
    }
    pub fn declare(&mut self, name: &str, t: PlcType, category: VariableCategory) -> SymbolId {
        let id = sid(self.next_symbol);
        self.next_symbol += 1;
        let source = self.anchor();
        self.symbols
            .declare(Symbol::new(id, name, t, Scope::Pou(entry()), category, Some(source)).unwrap())
            .unwrap();
        id
    }
    pub fn read(&mut self, id: SymbolId) -> Expression {
        Expression::read(self.anchor(), id, self.symbols.get(id).unwrap().plc_type())
    }
    pub fn constant(&mut self, value: PlcValue) -> Expression {
        Expression::constant(self.anchor(), value)
    }
    pub fn not(&mut self, operand: Expression) -> Expression {
        Expression::unary(self.anchor(), PlcType::Bool, UnaryOperator::Not, operand)
    }
    pub fn binary(
        &mut self,
        operator: BinaryOperator,
        t: PlcType,
        left: Expression,
        right: Expression,
    ) -> Expression {
        Expression::binary(self.anchor(), t, operator, left, right)
    }
    pub fn assign(&mut self, target: SymbolId, value: Expression) -> Statement {
        Statement::assign(self.anchor(), target, value)
    }
    pub fn conditional(
        &mut self,
        condition: Expression,
        then_body: Vec<Statement>,
        else_body: Vec<Statement>,
    ) -> Statement {
        Statement::conditional(self.anchor(), condition, then_body, else_body)
    }
    pub fn finish(self, body: Vec<Statement>) -> Program {
        Program::new(self.symbols, self.sources, entry(), body)
    }
}
