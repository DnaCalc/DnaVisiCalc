use std::fmt;

use crate::address::{CellRef, SheetBounds, is_cell_reference_token, parse_cell_ref};
use crate::ast::{BinaryOp, Expr, UnaryOp};

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub position: usize,
}

impl ParseError {
    fn new(message: impl Into<String>, position: usize) -> Self {
        Self {
            message: message.into(),
            position,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at position {}", self.message, self.position)
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokenKind,
    position: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Number(f64),
    String(String),
    Bool(bool),
    Cell(CellRef),
    Ident(String),
    LParen,
    RParen,
    Comma,
    Colon,
    Ellipsis,
    Hash,
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Ampersand,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

pub fn parse_formula(input: &str, bounds: SheetBounds) -> Result<Expr, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::new("formula is empty", 0));
    }
    let body = trimmed.strip_prefix('=').unwrap_or(trimmed);
    let tokens = tokenize(body, bounds)?;
    let mut parser = Parser::new(tokens);
    let expr = parser.parse_expression()?;
    if let Some(token) = parser.peek() {
        return Err(ParseError::new("unexpected trailing token", token.position));
    }
    Ok(expr)
}

fn tokenize(input: &str, bounds: SheetBounds) -> Result<Vec<Token>, ParseError> {
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        match c {
            '(' => {
                tokens.push(Token {
                    kind: TokenKind::LParen,
                    position: i,
                });
                i += 1;
            }
            ')' => {
                tokens.push(Token {
                    kind: TokenKind::RParen,
                    position: i,
                });
                i += 1;
            }
            ',' => {
                tokens.push(Token {
                    kind: TokenKind::Comma,
                    position: i,
                });
                i += 1;
            }
            ':' => {
                tokens.push(Token {
                    kind: TokenKind::Colon,
                    position: i,
                });
                i += 1;
            }
            '#' => {
                tokens.push(Token {
                    kind: TokenKind::Hash,
                    position: i,
                });
                i += 1;
            }
            '+' => {
                tokens.push(Token {
                    kind: TokenKind::Plus,
                    position: i,
                });
                i += 1;
            }
            '-' => {
                tokens.push(Token {
                    kind: TokenKind::Minus,
                    position: i,
                });
                i += 1;
            }
            '*' => {
                tokens.push(Token {
                    kind: TokenKind::Star,
                    position: i,
                });
                i += 1;
            }
            '/' => {
                tokens.push(Token {
                    kind: TokenKind::Slash,
                    position: i,
                });
                i += 1;
            }
            '^' => {
                tokens.push(Token {
                    kind: TokenKind::Caret,
                    position: i,
                });
                i += 1;
            }
            '&' => {
                tokens.push(Token {
                    kind: TokenKind::Ampersand,
                    position: i,
                });
                i += 1;
            }
            '=' => {
                tokens.push(Token {
                    kind: TokenKind::Eq,
                    position: i,
                });
                i += 1;
            }
            '"' => {
                let start = i;
                let (value, next_i) = consume_string(input, i)?;
                tokens.push(Token {
                    kind: TokenKind::String(value),
                    position: start,
                });
                i = next_i;
            }
            '<' => {
                if i + 1 < bytes.len() && bytes[i + 1] as char == '=' {
                    tokens.push(Token {
                        kind: TokenKind::Le,
                        position: i,
                    });
                    i += 2;
                } else if i + 1 < bytes.len() && bytes[i + 1] as char == '>' {
                    tokens.push(Token {
                        kind: TokenKind::Ne,
                        position: i,
                    });
                    i += 2;
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Lt,
                        position: i,
                    });
                    i += 1;
                }
            }
            '>' => {
                if i + 1 < bytes.len() && bytes[i + 1] as char == '=' {
                    tokens.push(Token {
                        kind: TokenKind::Ge,
                        position: i,
                    });
                    i += 2;
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Gt,
                        position: i,
                    });
                    i += 1;
                }
            }
            '.' => {
                if i + 2 < bytes.len() && bytes[i + 1] as char == '.' && bytes[i + 2] as char == '.'
                {
                    tokens.push(Token {
                        kind: TokenKind::Ellipsis,
                        position: i,
                    });
                    i += 3;
                } else {
                    return Err(ParseError::new("unexpected '.'", i));
                }
            }
            '@' => {
                let start = i;
                i += 1;
                let ident_start = i;
                while i < bytes.len() {
                    let ch = bytes[i] as char;
                    if ch.is_ascii_alphanumeric() || ch == '_' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                if i == ident_start {
                    return Err(ParseError::new("expected function name after '@'", start));
                }
                let ident = input[ident_start..i].to_ascii_uppercase();
                tokens.push(Token {
                    kind: TokenKind::Ident(ident),
                    position: start,
                });
            }
            ch if ch.is_ascii_digit() => {
                let start = i;
                i = consume_number(bytes, i);
                let raw = &input[start..i];
                let value = raw
                    .parse::<f64>()
                    .map_err(|_| ParseError::new(format!("invalid number '{raw}'"), start))?;
                tokens.push(Token {
                    kind: TokenKind::Number(value),
                    position: start,
                });
            }
            ch if ch.is_ascii_alphabetic() || ch == '_' => {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    let ch2 = bytes[i] as char;
                    if ch2.is_ascii_alphanumeric() || ch2 == '_' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let raw = &input[start..i];
                let upper = raw.to_ascii_uppercase();
                let call_ahead = next_non_whitespace_char(bytes, i) == Some('(');
                if upper == "TRUE" {
                    tokens.push(Token {
                        kind: TokenKind::Bool(true),
                        position: start,
                    });
                    continue;
                }
                if upper == "FALSE" {
                    tokens.push(Token {
                        kind: TokenKind::Bool(false),
                        position: start,
                    });
                    continue;
                }
                if is_cell_reference_token(&upper) && !call_ahead {
                    let cell = parse_cell_ref(&upper, bounds)
                        .map_err(|err| ParseError::new(err.to_string(), start))?;
                    tokens.push(Token {
                        kind: TokenKind::Cell(cell),
                        position: start,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Ident(upper),
                        position: start,
                    });
                }
            }
            _ => {
                return Err(ParseError::new(format!("unexpected character '{c}'"), i));
            }
        }
    }
    Ok(tokens)
}

fn consume_number(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }
    if i < bytes.len() && bytes[i] as char == '.' {
        if i + 2 < bytes.len() && bytes[i + 1] as char == '.' && bytes[i + 2] as char == '.' {
            return i;
        }
        i += 1;
        while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
            i += 1;
        }
    }
    if i < bytes.len() && ((bytes[i] as char) == 'e' || (bytes[i] as char) == 'E') {
        let exp_start = i;
        i += 1;
        if i < bytes.len() && ((bytes[i] as char) == '+' || (bytes[i] as char) == '-') {
            i += 1;
        }
        let digits_start = i;
        while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
            i += 1;
        }
        if digits_start == i {
            return exp_start;
        }
    }
    i
}

fn next_non_whitespace_char(bytes: &[u8], mut i: usize) -> Option<char> {
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if !ch.is_ascii_whitespace() {
            return Some(ch);
        }
        i += 1;
    }
    None
}

fn consume_string(input: &str, start: usize) -> Result<(String, usize), ParseError> {
    let bytes = input.as_bytes();
    let mut i = start + 1;
    let mut out = String::new();

    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch == '"' {
            if i + 1 < bytes.len() && bytes[i + 1] as char == '"' {
                out.push('"');
                i += 2;
                continue;
            }
            return Ok((out, i + 1));
        }
        out.push(ch);
        i += 1;
    }

    Err(ParseError::new("unterminated string literal", start))
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_concatenation()?;
        while let Some(op) = self.consume_comparison_operator() {
            let rhs = self.parse_concatenation()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
        }
        Ok(expr)
    }

    fn parse_concatenation(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_additive()?;
        while self.match_kind(TokenKind::Ampersand) {
            let rhs = self.parse_additive()?;
            expr = Expr::Binary {
                op: BinaryOp::Concat,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
        }
        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_multiplicative()?;
        loop {
            let op = if self.match_kind(TokenKind::Plus) {
                Some(BinaryOp::Add)
            } else if self.match_kind(TokenKind::Minus) {
                Some(BinaryOp::Sub)
            } else {
                None
            };
            let Some(op) = op else { break };
            let rhs = self.parse_multiplicative()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
        }
        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_power()?;
        loop {
            let op = if self.match_kind(TokenKind::Star) {
                Some(BinaryOp::Mul)
            } else if self.match_kind(TokenKind::Slash) {
                Some(BinaryOp::Div)
            } else {
                None
            };
            let Some(op) = op else { break };
            let rhs = self.parse_power()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(rhs),
            };
        }
        Ok(expr)
    }

    fn parse_power(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.parse_unary()?;
        if self.match_kind(TokenKind::Caret) {
            let rhs = self.parse_power()?;
            return Ok(Expr::Binary {
                op: BinaryOp::Pow,
                left: Box::new(lhs),
                right: Box::new(rhs),
            });
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.match_kind(TokenKind::Plus) {
            return Ok(Expr::Unary {
                op: UnaryOp::Plus,
                expr: Box::new(self.parse_unary()?),
            });
        }
        if self.match_kind(TokenKind::Minus) {
            return Ok(Expr::Unary {
                op: UnaryOp::Minus,
                expr: Box::new(self.parse_unary()?),
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.match_kind(TokenKind::LParen) {
                let mut args = Vec::new();
                if !self.match_kind(TokenKind::RParen) {
                    loop {
                        let arg = self.parse_expression()?;
                        args.push(arg);
                        if self.match_kind(TokenKind::Comma) {
                            continue;
                        }
                        self.expect_kind(TokenKind::RParen)?;
                        break;
                    }
                }
                expr = Expr::Invoke {
                    callee: Box::new(expr),
                    args,
                };
                continue;
            }

            let has_range_op =
                self.match_kind(TokenKind::Colon) || self.match_kind(TokenKind::Ellipsis);
            if !has_range_op {
                break;
            }
            let rhs = self.parse_primary()?;
            let left_cell = extract_cell(expr)?;
            let right_cell = extract_cell(rhs)?;
            expr = Expr::Range(crate::address::CellRange::new(left_cell, right_cell));
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let Some(token) = self.advance() else {
            return Err(ParseError::new("unexpected end of input", self.position()));
        };
        match &token.kind {
            TokenKind::Number(value) => Ok(Expr::Number(*value)),
            TokenKind::String(value) => Ok(Expr::Text(value.clone())),
            TokenKind::Bool(value) => Ok(Expr::Bool(*value)),
            TokenKind::Cell(cell) => {
                if self.match_kind(TokenKind::Hash) {
                    Ok(Expr::SpillRef(*cell))
                } else {
                    Ok(Expr::Cell(*cell))
                }
            }
            TokenKind::Ident(name) => {
                if self.match_kind(TokenKind::LParen) {
                    let mut args = Vec::new();
                    if !self.match_kind(TokenKind::RParen) {
                        loop {
                            let arg = self.parse_expression()?;
                            args.push(arg);
                            if self.match_kind(TokenKind::Comma) {
                                continue;
                            }
                            self.expect_kind(TokenKind::RParen)?;
                            break;
                        }
                    }
                    return Ok(Expr::FunctionCall {
                        name: name.clone(),
                        args,
                    });
                }
                Ok(Expr::Name(name.clone()))
            }
            TokenKind::LParen => {
                let inner = self.parse_expression()?;
                self.expect_kind(TokenKind::RParen)?;
                Ok(inner)
            }
            _ => Err(ParseError::new(
                "expected number, string, cell reference, function call, or '('",
                token.position,
            )),
        }
    }

    fn consume_comparison_operator(&mut self) -> Option<BinaryOp> {
        if self.match_kind(TokenKind::Eq) {
            return Some(BinaryOp::Eq);
        }
        if self.match_kind(TokenKind::Ne) {
            return Some(BinaryOp::Ne);
        }
        if self.match_kind(TokenKind::Le) {
            return Some(BinaryOp::Le);
        }
        if self.match_kind(TokenKind::Lt) {
            return Some(BinaryOp::Lt);
        }
        if self.match_kind(TokenKind::Ge) {
            return Some(BinaryOp::Ge);
        }
        if self.match_kind(TokenKind::Gt) {
            return Some(BinaryOp::Gt);
        }
        None
    }

    fn expect_kind(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        if self.match_kind(kind) {
            Ok(())
        } else {
            let pos = self.peek().map_or(self.position(), |token| token.position);
            Err(ParseError::new("unexpected token", pos))
        }
    }

    fn match_kind(&mut self, kind: TokenKind) -> bool {
        if let Some(token) = self.peek() {
            if token.kind == kind {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos >= self.tokens.len() {
            return None;
        }
        let token = self.tokens[self.pos].clone();
        self.pos += 1;
        Some(token)
    }

    fn position(&self) -> usize {
        self.tokens.last().map_or(0, |token| token.position + 1)
    }
}

fn extract_cell(expr: Expr) -> Result<CellRef, ParseError> {
    match expr {
        Expr::Cell(cell) => Ok(cell),
        _ => Err(ParseError::new(
            "range boundaries must be cell references",
            0,
        )),
    }
}
