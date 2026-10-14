//! ST syntax and provenance, distinct from canonical semantic IR.
use plc_types::{
    PlcType,
    compatibility::{BinaryOperator, UnaryOperator},
    source::TextSpan,
    symbols::VariableCategory,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    pub text: String,
    pub span: TextSpan,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclaredType {
    Scalar(PlcType),
    Ton,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub name: Name,
    pub role: VariableCategory,
    pub kind: DeclaredType,
    pub span: TextSpan,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramAst {
    pub name: Name,
    pub declarations: Vec<Declaration>,
    pub body: Vec<Statement>,
    pub span: TextSpan,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    Bool(bool),
    Integer(String),
    Real(String),
    Time(String),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    pub span: TextSpan,
    pub kind: ExpressionKind,
    pub(crate) depth: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionKind {
    Literal(Literal),
    Identifier(Name),
    Parentheses(Box<Expression>),
    Unary(UnaryOperator, Box<Expression>),
    Binary(BinaryOperator, Box<Expression>, Box<Expression>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub span: TextSpan,
    pub kind: StatementKind,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementKind {
    Assignment(Name, Expression),
    If(Expression, Vec<Statement>, Vec<Statement>),
    Call(Name, Vec<Association>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Association {
    pub parameter: Name,
    pub span: TextSpan,
    pub value: AssociationValue,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssociationValue {
    Input(Expression),
    Output(Name),
}
