//! Handwritten scanner. Columns count Unicode scalars; CRLF is one line break.
use crate::{
    ast::Literal,
    diagnostics::{CompilerConfig, Diagnostics, codes, error},
    literals,
};
use plc_types::{
    PlcType,
    function_blocks::StandardFunctionBlock,
    diagnostics::Diagnostic,
    source::{SourcePosition, TextSpan},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Program,
    EndProgram,
    VarInput,
    VarOutput,
    Var,
    EndVar,
    If,
    Then,
    Else,
    EndIf,
    Elsif,
    Case,
    Of,
    EndCase,
    Not,
    And,
    Or,
    Xor,
    Mod,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Keyword(Keyword),
    Type(PlcType),
    FunctionBlockType(StandardFunctionBlock),
    Identifier(String),
    Literal(Literal),
    Unsupported(String),
    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Assign,
    Output,
    LeftParen,
    RightParen,
    Comma,
    Dot,
    Range,
    Semicolon,
    Colon,
    Eof,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: TextSpan,
}

pub fn lex(source: &str, config: CompilerConfig) -> Result<Vec<Token>, Vec<Diagnostic>> {
    if source.len() > config.source_bytes {
        return Err(vec![error(
            codes::LIMIT,
            "Source exceeds the configured byte limit.",
            None,
        )]);
    }
    Scanner {
        chars: source.chars().collect(),
        cursor: 0,
        line: 1,
        column: 1,
        tokens: Vec::new(),
        errors: Diagnostics::new(config.diagnostics),
        config,
    }
    .scan()
}
struct Scanner {
    chars: Vec<char>,
    cursor: usize,
    line: u32,
    column: u32,
    tokens: Vec<Token>,
    errors: Diagnostics,
    config: CompilerConfig,
}
impl Scanner {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.cursor).copied()
    }
    fn next(&self) -> Option<char> {
        self.chars.get(self.cursor + 1).copied()
    }
    fn position(&self) -> SourcePosition {
        SourcePosition::new(self.line, self.column).expect("bounded positive scanner coordinates")
    }
    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.cursor += 1;
        if ch == '\r' {
            if self.peek() == Some('\n') {
                self.cursor += 1;
            }
            self.line += 1;
            self.column = 1;
        } else if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }
    fn span(&self, start: SourcePosition) -> TextSpan {
        TextSpan::new(start, self.position()).expect("scanner order")
    }
    fn text(&self, start: usize) -> String {
        self.chars[start..self.cursor].iter().collect()
    }
    fn scan(mut self) -> Result<Vec<Token>, Vec<Diagnostic>> {
        while let Some(ch) = self.peek() {
            if self.errors.stopped {
                break;
            }
            if ch.is_whitespace() {
                self.bump();
                continue;
            }
            let start = self.position();
            if ch == '/' && self.next() == Some('/') {
                while self.peek().is_some_and(|c| c != '\r' && c != '\n') {
                    self.bump();
                }
                continue;
            }
            if ch == '(' && self.next() == Some('*') {
                self.bump();
                self.bump();
                let mut closed = false;
                while self.peek().is_some() {
                    if self.peek() == Some('*') && self.next() == Some(')') {
                        self.bump();
                        self.bump();
                        closed = true;
                        break;
                    }
                    if self.peek() == Some('(') && self.next() == Some('*') {
                        self.errors.push(error(
                            codes::LEXICAL,
                            "Nested block comments are unsupported.",
                            Some(self.span(start)),
                        ));
                    }
                    self.bump();
                }
                if !closed {
                    self.errors.push(error(
                        codes::LEXICAL,
                        "Unterminated block comment; expected '*)'.",
                        Some(self.span(start)),
                    ));
                }
                continue;
            }
            if self.tokens.len() + 1 >= self.config.tokens {
                self.errors.push(error(
                    codes::LIMIT,
                    "Token limit exceeded (including EOF).",
                    Some(self.span(start)),
                ));
                break;
            }
            let offset = self.cursor;
            let kind = if ch.is_ascii_alphabetic() || ch == '_' {
                while self
                    .peek()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    self.bump();
                }
                let name = self.text(offset);
                let upper = name.to_ascii_uppercase();
                if matches!(upper.as_str(), "T" | "TIME") && self.peek() == Some('#') {
                    self.bump();
                    let payload = self.cursor;
                    if self.peek() == Some('-') {
                        self.bump();
                    }
                    while self
                        .peek()
                        .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '#'))
                    {
                        self.bump();
                    }
                    let raw = self.text(payload);
                    if let Err(message) = literals::time(&raw) {
                        self.errors
                            .push(error(codes::LITERAL, message, Some(self.span(start))));
                    }
                    TokenKind::Literal(Literal::Time(raw))
                } else {
                    word(name, &upper)
                }
            } else if ch.is_ascii_digit() {
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    self.bump();
                }
                let mut real = false;
                let mut malformed = false;
                if self.peek() == Some('.') && self.next() != Some('.') {
                    real = true;
                    self.bump();
                    let first = self.cursor;
                    while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                        self.bump();
                    }
                    malformed = self.cursor == first;
                }
                if self.peek().is_some_and(|c| c == 'e' || c == 'E') {
                    malformed |= !real;
                    self.bump();
                    if self.peek().is_some_and(|c| c == '+' || c == '-') {
                        self.bump();
                    }
                    let first = self.cursor;
                    while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                        self.bump();
                    }
                    malformed |= self.cursor == first;
                }
                if self
                    .peek()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                {
                    malformed = true;
                    while self
                        .peek()
                        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
                    {
                        self.bump();
                    }
                }
                if malformed {
                    self.errors.push(error(codes::LITERAL, "Malformed numeric literal; REAL requires digits on both sides of a decimal point.", Some(self.span(start))));
                }
                let raw = self.text(offset);
                TokenKind::Literal(if real {
                    Literal::Real(raw)
                } else {
                    Literal::Integer(raw)
                })
            } else {
                self.bump();
                match ch {
                    '+' => TokenKind::Plus,
                    '-' => TokenKind::Minus,
                    '*' => TokenKind::Star,
                    '/' => TokenKind::Slash,
                    '(' => TokenKind::LeftParen,
                    ')' => TokenKind::RightParen,
                    ',' => TokenKind::Comma,
                    '.' if self.peek() == Some('.') => {
                        self.bump();
                        TokenKind::Range
                    }
                    '.' => TokenKind::Dot,
                    ';' => TokenKind::Semicolon,
                    ':' if self.peek() == Some('=') => {
                        self.bump();
                        TokenKind::Assign
                    }
                    ':' => TokenKind::Colon,
                    '=' if self.peek() == Some('>') => {
                        self.bump();
                        TokenKind::Output
                    }
                    '=' => TokenKind::Equal,
                    '<' if self.peek() == Some('=') => {
                        self.bump();
                        TokenKind::LessEqual
                    }
                    '<' if self.peek() == Some('>') => {
                        self.bump();
                        TokenKind::NotEqual
                    }
                    '<' => TokenKind::Less,
                    '>' if self.peek() == Some('=') => {
                        self.bump();
                        TokenKind::GreaterEqual
                    }
                    '>' => TokenKind::Greater,
                    _ => {
                        self.errors.push(error(codes::LEXICAL, format!("Invalid character '{ch}'; Alpha names use ASCII letters, digits and underscore."), Some(self.span(start))));
                        continue;
                    }
                }
            };
            self.tokens.push(Token {
                kind,
                span: self.span(start),
            });
        }
        if !self.errors.items.is_empty() {
            return Err(self.errors.items);
        }
        let end = self.position();
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: TextSpan::new(end, end).expect("EOF point"),
        });
        Ok(self.tokens)
    }
}
fn word(name: String, upper: &str) -> TokenKind {
    use Keyword::*;
    let keyword = match upper {
        "PROGRAM" => Program,
        "END_PROGRAM" => EndProgram,
        "VAR_INPUT" => VarInput,
        "VAR_OUTPUT" => VarOutput,
        "VAR" => Var,
        "END_VAR" => EndVar,
        "IF" => If,
        "THEN" => Then,
        "ELSE" => Else,
        "END_IF" => EndIf,
        "ELSIF" => Elsif,
        "CASE" => Case,
        "OF" => Of,
        "END_CASE" => EndCase,
        "XOR" => Xor,
        "MOD" => Mod,
        "NOT" => Not,
        "AND" => And,
        "OR" => Or,
        "BOOL" => return TokenKind::Type(PlcType::Bool),
        "INT" => return TokenKind::Type(PlcType::Int),
        "REAL" => return TokenKind::Type(PlcType::Real),
        "TIME" => return TokenKind::Type(PlcType::Time),
        "TON" | "TOF" | "TP" | "R_TRIG" | "F_TRIG" | "CTU" | "CTD" => {
            return TokenKind::FunctionBlockType(
                StandardFunctionBlock::from_name(upper).expect("listed standard block name"),
            );
        }
        "TRUE" => return TokenKind::Literal(Literal::Bool(true)),
        "FALSE" => return TokenKind::Literal(Literal::Bool(false)),
        "FOR" | "WHILE" | "REPEAT" | "RETURN" | "ARRAY" | "STRUCT"
        | "STRING" | "FUNCTION" | "FUNCTION_BLOCK" | "METHOD" | "CLASS" | "NAMESPACE"
        | "VAR_IN_OUT" | "VAR_TEMP" | "RETAIN" | "AT" | "POINTER" | "REFERENCE" | "INCLUDE"
        | "IMPORT" | "NAN" | "INF" | "INFINITY" => return TokenKind::Unsupported(name),
        _ => return TokenKind::Identifier(name),
    };
    TokenKind::Keyword(keyword)
}
