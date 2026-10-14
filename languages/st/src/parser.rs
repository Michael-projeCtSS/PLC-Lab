use crate::{
    ast::*,
    diagnostics::{CompilerConfig, Diagnostics, codes, error},
    lexer::{Keyword, Token, TokenKind},
};
use plc_types::{
    compatibility::{BinaryOperator, UnaryOperator},
    diagnostics::Diagnostic,
    source::TextSpan,
    symbols::VariableCategory,
};

pub(crate) fn parse(
    tokens: &[Token],
    config: CompilerConfig,
) -> Result<ProgramAst, Vec<Diagnostic>> {
    let mut parser = Parser {
        tokens,
        cursor: 0,
        config,
        errors: Diagnostics::new(config.diagnostics),
    };
    let result = parser.program();
    match result {
        Ok(ast) if parser.errors.items.is_empty() => Ok(ast),
        Ok(_) => Err(parser.errors.items),
        Err(diagnostic) => {
            parser.errors.push(diagnostic);
            Err(parser.errors.items)
        }
    }
}
struct Parser<'a> {
    tokens: &'a [Token],
    cursor: usize,
    config: CompilerConfig,
    errors: Diagnostics,
}
fn joined(first: TextSpan, last: TextSpan) -> TextSpan {
    TextSpan::new(first.start(), last.end()).expect("ordered parsed spans")
}
impl Parser<'_> {
    fn peek(&self) -> &Token {
        &self.tokens[self.cursor.min(self.tokens.len() - 1)]
    }
    fn at(&self, kind: &TokenKind) -> bool {
        &self.peek().kind == kind
    }
    fn keyword(&self, word: Keyword) -> bool {
        self.at(&TokenKind::Keyword(word))
    }
    fn take(&mut self) -> Token {
        let token = self.peek().clone();
        if token.kind != TokenKind::Eof {
            self.cursor += 1;
        }
        token
    }
    fn syntax(&self, message: &str) -> Diagnostic {
        match &self.peek().kind {
            TokenKind::Unsupported(name) => error(
                codes::UNSUPPORTED,
                format!("'{name}' is unsupported in Alpha Structured Text."),
                Some(self.peek().span),
            ),
            _ => error(codes::SYNTAX, message, Some(self.peek().span)),
        }
    }
    fn expect(&mut self, kind: TokenKind, message: &str) -> Result<Token, Diagnostic> {
        if self.at(&kind) {
            Ok(self.take())
        } else {
            Err(self.syntax(message))
        }
    }
    fn name(&mut self) -> Result<Name, Diagnostic> {
        if let TokenKind::Identifier(text) = self.peek().kind.clone() {
            let span = self.take().span;
            Ok(Name { text, span })
        } else {
            Err(self.syntax("Expected an ASCII identifier: [A-Za-z_][A-Za-z0-9_]*."))
        }
    }
    fn program(&mut self) -> Result<ProgramAst, Diagnostic> {
        let start = self
            .expect(
                TokenKind::Keyword(Keyword::Program),
                "Expected PROGRAM followed by its name.",
            )?
            .span;
        let name = self.name()?;
        let mut declarations = Vec::new();
        while matches!(
            self.peek().kind,
            TokenKind::Keyword(Keyword::Var | Keyword::VarInput | Keyword::VarOutput)
        ) && !self.errors.stopped
        {
            let section = self.take();
            let role = match section.kind {
                TokenKind::Keyword(Keyword::VarInput) => VariableCategory::Input,
                TokenKind::Keyword(Keyword::VarOutput) => VariableCategory::Output,
                _ => VariableCategory::Local,
            };
            while !self.keyword(Keyword::EndVar)
                && !self.at(&TokenKind::Eof)
                && !self.errors.stopped
            {
                if matches!(
                    self.peek().kind,
                    TokenKind::Keyword(
                        Keyword::EndProgram
                            | Keyword::If
                            | Keyword::EndIf
                            | Keyword::Else
                            | Keyword::Var
                            | Keyword::VarInput
                            | Keyword::VarOutput
                    )
                ) || self
                    .tokens
                    .get(self.cursor + 1)
                    .is_some_and(|t| matches!(t.kind, TokenKind::Assign | TokenKind::LeftParen))
                {
                    self.errors
                        .push(self.syntax("Expected END_VAR before program statements."));
                    break;
                }
                let before = self.cursor;
                match self.declaration(role) {
                    Ok(declaration) => declarations.push(declaration),
                    Err(diagnostic) => {
                        self.errors.push(diagnostic);
                        self.synchronize(before, true);
                    }
                }
            }
            if self.keyword(Keyword::EndVar) {
                self.take();
            } else if self.at(&TokenKind::Eof) {
                self.errors
                    .push(self.syntax("Expected END_VAR after declarations."));
            }
        }
        let body = self.body(0);
        let end = self
            .expect(
                TokenKind::Keyword(Keyword::EndProgram),
                "Expected END_PROGRAM.",
            )?
            .span;
        if !self.at(&TokenKind::Eof) {
            return Err(error(
                codes::UNSUPPORTED,
                if self.keyword(Keyword::Program) {
                    "Only one PROGRAM is supported per source document."
                } else {
                    "Unexpected source after END_PROGRAM; only one PROGRAM is supported."
                },
                Some(self.peek().span),
            ));
        }
        Ok(ProgramAst {
            name,
            declarations,
            body,
            span: joined(start, end),
        })
    }
    fn declaration(&mut self, role: VariableCategory) -> Result<Declaration, Diagnostic> {
        let name = self.name()?;
        self.expect(
            TokenKind::Colon,
            "Expected ':' between a declaration name and its type.",
        )?;
        let kind = match self.peek().kind.clone() {
            TokenKind::Type(t) => {
                self.take();
                DeclaredType::Scalar(t)
            }
            TokenKind::Ton => {
                self.take();
                DeclaredType::Ton
            }
            TokenKind::Identifier(type_name) => {
                return Err(error(
                    codes::UNSUPPORTED,
                    format!(
                        "Type '{type_name}' is unsupported; use BOOL, INT, REAL, TIME or a local TON instance."
                    ),
                    Some(self.peek().span),
                ));
            }
            _ => return Err(self.syntax("Expected BOOL, INT, REAL, TIME or TON.")),
        };
        let end = self
            .expect(TokenKind::Semicolon, "Expected ';' after declaration.")?
            .span;
        Ok(Declaration {
            span: joined(name.span, end),
            name,
            role,
            kind,
        })
    }
    fn body(&mut self, depth: usize) -> Vec<Statement> {
        let mut statements = Vec::new();
        while !self.at(&TokenKind::Eof)
            && !matches!(
                self.peek().kind,
                TokenKind::Keyword(Keyword::Else | Keyword::EndIf | Keyword::EndProgram)
            )
            && !self.errors.stopped
        {
            let before = self.cursor;
            match self.statement(depth) {
                Ok(statement) => statements.push(statement),
                Err(diagnostic) => {
                    self.errors.push(diagnostic);
                    self.synchronize(before, false);
                }
            }
        }
        statements
    }
    fn synchronize(&mut self, before: usize, declaration: bool) {
        while !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Semicolon) {
                self.take();
                return;
            }
            if matches!(
                self.peek().kind,
                TokenKind::Keyword(
                    Keyword::EndVar | Keyword::EndIf | Keyword::Else | Keyword::EndProgram
                )
            ) {
                break;
            }
            // A missing semicolon need not discard the following recognizable statement.
            if self.cursor > before
                && (self.keyword(Keyword::If)
                    || self.tokens.get(self.cursor + 1).is_some_and(|t| {
                        t.kind
                            == if declaration {
                                TokenKind::Colon
                            } else {
                                TokenKind::Assign
                            }
                    }))
            {
                return;
            }
            self.take();
        }
        if self.cursor == before
            && !self.at(&TokenKind::Eof)
            && !matches!(
                self.peek().kind,
                TokenKind::Keyword(Keyword::EndIf | Keyword::Else | Keyword::EndProgram)
            )
        {
            self.take();
        }
    }
    fn statement(&mut self, depth: usize) -> Result<Statement, Diagnostic> {
        if depth >= self.config.nesting {
            return Err(error(
                codes::LIMIT,
                "Parser nesting limit exceeded.",
                Some(self.peek().span),
            ));
        }
        if self.keyword(Keyword::If) {
            let start = self.take().span;
            let condition = self.expression(1, depth + 1)?;
            self.expect(
                TokenKind::Keyword(Keyword::Then),
                "Expected THEN after IF condition.",
            )?;
            let then_body = self.body(depth + 1);
            let else_body = if self.keyword(Keyword::Else) {
                self.take();
                self.body(depth + 1)
            } else {
                Vec::new()
            };
            self.expect(TokenKind::Keyword(Keyword::EndIf), "Expected END_IF.")?;
            let end = self
                .expect(TokenKind::Semicolon, "Expected ';' after END_IF.")?
                .span;
            return Ok(Statement {
                span: joined(start, end),
                kind: StatementKind::If(condition, then_body, else_body),
            });
        }
        let name = self.name()?;
        let start = name.span;
        if self.at(&TokenKind::Assign) {
            self.take();
            let value = self.expression(1, depth + 1)?;
            let end = self
                .expect(TokenKind::Semicolon, "Expected ';' after assignment.")?
                .span;
            return Ok(Statement {
                span: joined(start, end),
                kind: StatementKind::Assignment(name, value),
            });
        }
        self.expect(
            TokenKind::LeftParen,
            "Expected ':=' for assignment or '(' for a TON call.",
        )?;
        let mut associations = Vec::new();
        if !self.at(&TokenKind::RightParen) {
            loop {
                let parameter = self.name()?;
                let span = parameter.span;
                let value = if self.at(&TokenKind::Assign) {
                    self.take();
                    AssociationValue::Input(self.expression(1, depth + 1)?)
                } else {
                    self.expect(
                        TokenKind::Output,
                        "Expected ':=' for an input or '=>' for an output association.",
                    )?;
                    AssociationValue::Output(self.name()?)
                };
                let end = match &value {
                    AssociationValue::Input(e) => e.span,
                    AssociationValue::Output(n) => n.span,
                };
                associations.push(Association {
                    parameter,
                    value,
                    span: joined(span, end),
                });
                if !self.at(&TokenKind::Comma) {
                    break;
                }
                self.take();
            }
        }
        self.expect(TokenKind::RightParen, "Expected ')' after TON arguments.")?;
        let end = self
            .expect(TokenKind::Semicolon, "Expected ';' after TON call.")?
            .span;
        Ok(Statement {
            span: joined(start, end),
            kind: StatementKind::Call(name, associations),
        })
    }
    fn expression(&mut self, minimum: u8, level: usize) -> Result<Expression, Diagnostic> {
        if level >= self.config.nesting {
            return Err(error(
                codes::LIMIT,
                "Expression nesting limit exceeded.",
                Some(self.peek().span),
            ));
        }
        let token = self.take();
        let mut left = match token.kind {
            TokenKind::Literal(literal) => Expression {
                span: token.span,
                kind: ExpressionKind::Literal(literal),
                depth: 1,
            },
            TokenKind::Identifier(text) => Expression {
                span: token.span,
                kind: ExpressionKind::Identifier(Name {
                    text,
                    span: token.span,
                }),
                depth: 1,
            },
            TokenKind::Minus | TokenKind::Keyword(Keyword::Not) => {
                let op = if token.kind == TokenKind::Minus {
                    UnaryOperator::Negate
                } else {
                    UnaryOperator::Not
                };
                let operand = self.expression(6, level + 1)?;
                Expression {
                    span: joined(token.span, operand.span),
                    depth: operand.depth + 1,
                    kind: ExpressionKind::Unary(op, Box::new(operand)),
                }
            }
            TokenKind::LeftParen => {
                let operand = self.expression(1, level + 1)?;
                let end = self
                    .expect(TokenKind::RightParen, "Expected ')' after expression.")?
                    .span;
                Expression {
                    span: joined(token.span, end),
                    depth: operand.depth + 1,
                    kind: ExpressionKind::Parentheses(Box::new(operand)),
                }
            }
            TokenKind::Unsupported(name) => {
                return Err(error(
                    codes::UNSUPPORTED,
                    format!("'{name}' is unsupported in Alpha Structured Text."),
                    Some(token.span),
                ));
            }
            _ => {
                return Err(error(
                    codes::SYNTAX,
                    "Expected a literal, identifier, unary operator or parenthesized expression.",
                    Some(token.span),
                ));
            }
        };
        while let Some((operator, precedence)) = binary(&self.peek().kind) {
            if precedence < minimum {
                break;
            }
            self.take();
            let right = self.expression(precedence + 1, level + 1)?;
            let depth = left.depth.max(right.depth) + 1;
            if depth + level >= self.config.nesting {
                return Err(error(
                    codes::LIMIT,
                    "Expression tree limit exceeded.",
                    Some(joined(left.span, right.span)),
                ));
            }
            left = Expression {
                span: joined(left.span, right.span),
                depth,
                kind: ExpressionKind::Binary(operator, Box::new(left), Box::new(right)),
            };
        }
        if left.depth + level >= self.config.nesting {
            return Err(error(
                codes::LIMIT,
                "Expression tree limit exceeded.",
                Some(left.span),
            ));
        }
        Ok(left)
    }
}
fn binary(token: &TokenKind) -> Option<(BinaryOperator, u8)> {
    use BinaryOperator::*;
    Some(match token {
        TokenKind::Keyword(Keyword::Or) => (Or, 1),
        TokenKind::Keyword(Keyword::And) => (And, 2),
        TokenKind::Equal => (Equal, 3),
        TokenKind::NotEqual => (NotEqual, 3),
        TokenKind::Less => (Less, 3),
        TokenKind::LessEqual => (LessEqual, 3),
        TokenKind::Greater => (Greater, 3),
        TokenKind::GreaterEqual => (GreaterEqual, 3),
        TokenKind::Plus => (Add, 4),
        TokenKind::Minus => (Subtract, 4),
        TokenKind::Star => (Multiply, 5),
        TokenKind::Slash => (Divide, 5),
        _ => return None,
    })
}
