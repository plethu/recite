use recite_core::{Argument, ConditionCall, ConditionExpression, ScalarValue, SourceSpan};

use crate::source::span_for_text;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ParseError {
    pub(crate) span: SourceSpan,
    pub(crate) message: String,
}

impl ParseError {
    fn new(span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

pub(crate) fn parse_condition_expression(
    path: &str,
    line: u32,
    base_column: usize,
    text: &str,
) -> Result<ConditionExpression, ParseError> {
    let tokens = Lexer::new(path, line, base_column, text).lex()?;
    let mut parser = Parser::new(path, tokens);
    let expression = parser.parse_or()?;
    parser.expect_end()?;
    Ok(expression)
}

pub(crate) fn parse_condition_call(
    path: &str,
    line: u32,
    base_column: usize,
    text: &str,
) -> Result<ConditionCall, ParseError> {
    let tokens = Lexer::new(path, line, base_column, text).lex()?;
    let mut parser = Parser::new(path, tokens);
    let call = parser.parse_call()?;
    parser.expect_end()?;
    Ok(call)
}

#[derive(Clone, Debug, PartialEq)]
enum TokenKind {
    Ident(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    And,
    Or,
    Not,
    LeftParen,
    RightParen,
    Comma,
    End,
}

#[derive(Clone, Debug, PartialEq)]
struct Token {
    kind: TokenKind,
    span: SourceSpan,
}

struct Lexer<'a> {
    path: &'a str,
    line: u32,
    base_column: usize,
    text: &'a str,
    cursor: usize,
}

impl<'a> Lexer<'a> {
    fn new(path: &'a str, line: u32, base_column: usize, text: &'a str) -> Self {
        Self {
            path,
            line,
            base_column,
            text,
            cursor: 0,
        }
    }

    fn lex(mut self) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();

        while self.cursor < self.text.len() {
            let Some(character) = self.current_char() else {
                break;
            };

            match character {
                ' ' | '\t' => self.advance_char(),
                '(' => tokens.push(self.single_char(TokenKind::LeftParen)),
                ')' => tokens.push(self.single_char(TokenKind::RightParen)),
                ',' => tokens.push(self.single_char(TokenKind::Comma)),
                '"' => tokens.push(self.string()?),
                '-' | '0'..='9' if self.starts_number() => tokens.push(self.number()?),
                _ if is_identifier_start(character) => tokens.push(self.identifier()),
                _ => {
                    return Err(ParseError::new(
                        self.span_at_current(),
                        format!("unexpected character {character:?}"),
                    ));
                }
            }
        }

        tokens.push(Token {
            kind: TokenKind::End,
            span: self.span_at_current(),
        });
        Ok(tokens)
    }

    fn current_char(&self) -> Option<char> {
        self.text[self.cursor..].chars().next()
    }

    fn advance_char(&mut self) {
        let Some(character) = self.current_char() else {
            return;
        };
        self.cursor += character.len_utf8();
    }

    fn single_char(&mut self, kind: TokenKind) -> Token {
        let start = self.cursor;
        self.advance_char();
        self.token(kind, start, self.cursor)
    }

    fn identifier(&mut self) -> Token {
        let start = self.cursor;
        self.advance_char();
        while let Some(character) = self.current_char() {
            if !is_identifier_continue(character) {
                break;
            }
            self.advance_char();
        }

        let text = &self.text[start..self.cursor];
        let kind = match text {
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "true" => TokenKind::Boolean(true),
            "false" => TokenKind::Boolean(false),
            _ => TokenKind::Ident(text.to_owned()),
        };

        self.token(kind, start, self.cursor)
    }

    fn string(&mut self) -> Result<Token, ParseError> {
        let start = self.cursor;
        self.advance_char();
        let mut value = String::new();

        while let Some(character) = self.current_char() {
            match character {
                '"' => {
                    self.advance_char();
                    return Ok(self.token(TokenKind::String(value), start, self.cursor));
                }
                '\\' => {
                    self.advance_char();
                    let Some(escaped) = self.current_char() else {
                        return Err(ParseError::new(
                            self.span_for_range(start, self.cursor),
                            "unterminated string literal",
                        ));
                    };
                    match escaped {
                        '"' => value.push('"'),
                        '\\' => value.push('\\'),
                        'n' => value.push('\n'),
                        't' => value.push('\t'),
                        other => value.push(other),
                    }
                    self.advance_char();
                }
                other => {
                    value.push(other);
                    self.advance_char();
                }
            }
        }

        Err(ParseError::new(
            self.span_for_range(start, self.cursor),
            "unterminated string literal",
        ))
    }

    fn starts_number(&self) -> bool {
        let remaining = &self.text[self.cursor..];
        if let Some(after_minus) = remaining.strip_prefix('-') {
            return after_minus
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit());
        }

        remaining
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_digit())
    }

    fn number(&mut self) -> Result<Token, ParseError> {
        let start = self.cursor;
        if self.current_char() == Some('-') {
            self.advance_char();
        }

        while self
            .current_char()
            .is_some_and(|character| character.is_ascii_digit())
        {
            self.advance_char();
        }

        let mut is_float = false;
        if self.current_char() == Some('.') {
            is_float = true;
            self.advance_char();
            while self
                .current_char()
                .is_some_and(|character| character.is_ascii_digit())
            {
                self.advance_char();
            }
        }

        let text = &self.text[start..self.cursor];
        if is_float {
            let value = text.parse::<f64>().map_err(|_| {
                ParseError::new(
                    self.span_for_range(start, self.cursor),
                    "invalid float literal",
                )
            })?;
            return Ok(self.token(TokenKind::Float(value), start, self.cursor));
        }

        let value = text.parse::<i64>().map_err(|_| {
            ParseError::new(
                self.span_for_range(start, self.cursor),
                "invalid integer literal",
            )
        })?;
        Ok(self.token(TokenKind::Integer(value), start, self.cursor))
    }

    fn token(&self, kind: TokenKind, start: usize, end: usize) -> Token {
        Token {
            kind,
            span: self.span_for_range(start, end),
        }
    }

    fn span_at_current(&self) -> SourceSpan {
        self.span_for_range(self.cursor, self.cursor)
    }

    fn span_for_range(&self, start: usize, end: usize) -> SourceSpan {
        let column = self.base_column + self.text[..start].chars().count();
        span_for_text(self.path, self.line, column, &self.text[start..end])
    }
}

struct Parser<'a> {
    path: &'a str,
    tokens: Vec<Token>,
    cursor: usize,
}

impl<'a> Parser<'a> {
    fn new(path: &'a str, tokens: Vec<Token>) -> Self {
        Self {
            path,
            tokens,
            cursor: 0,
        }
    }

    fn parse_or(&mut self) -> Result<ConditionExpression, ParseError> {
        let first = self.parse_and()?;
        let mut expressions = vec![first];

        while self.at(TokenKindDiscriminant::Or) {
            self.bump();
            expressions.push(self.parse_and()?);
        }

        if expressions.len() == 1 {
            return Ok(expressions.remove(0));
        }

        let span = join_spans(
            self.path,
            expressions.first().unwrap().span(),
            expressions.last().unwrap().span(),
        );
        Ok(ConditionExpression::or(expressions, span))
    }

    fn parse_and(&mut self) -> Result<ConditionExpression, ParseError> {
        let first = self.parse_unary()?;
        let mut expressions = vec![first];

        while self.at(TokenKindDiscriminant::And) {
            self.bump();
            expressions.push(self.parse_unary()?);
        }

        if expressions.len() == 1 {
            return Ok(expressions.remove(0));
        }

        let span = join_spans(
            self.path,
            expressions.first().unwrap().span(),
            expressions.last().unwrap().span(),
        );
        Ok(ConditionExpression::and(expressions, span))
    }

    fn parse_unary(&mut self) -> Result<ConditionExpression, ParseError> {
        if self.at(TokenKindDiscriminant::Not) {
            let not = self.bump().clone();
            let expression = self.parse_unary()?;
            let span = join_spans(self.path, &not.span, expression.span());
            return Ok(ConditionExpression::not(expression, span));
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<ConditionExpression, ParseError> {
        if self.at(TokenKindDiscriminant::LeftParen) {
            let left = self.bump().clone();
            let expression = self.parse_or()?;
            let right = self
                .expect(TokenKindDiscriminant::RightParen, "expected ')'")?
                .clone();
            let span = join_spans(self.path, &left.span, &right.span);
            return Ok(ConditionExpression::grouped(expression, span));
        }

        let call = self.parse_call()?;
        Ok(ConditionExpression::Call(call))
    }

    fn parse_call(&mut self) -> Result<ConditionCall, ParseError> {
        let function = match &self.current().kind {
            TokenKind::Ident(function) => function.clone(),
            TokenKind::End => {
                return Err(ParseError::new(
                    self.current().span.clone(),
                    "expected function call",
                ));
            }
            _ => {
                return Err(ParseError::new(
                    self.current().span.clone(),
                    "expected function call",
                ));
            }
        };
        let function_span = self.bump().span.clone();
        self.expect(
            TokenKindDiscriminant::LeftParen,
            "expected '(' after function name",
        )?;

        let mut args = Vec::new();
        if !self.at(TokenKindDiscriminant::RightParen) {
            loop {
                args.push(self.parse_argument()?);
                if !self.at(TokenKindDiscriminant::Comma) {
                    break;
                }
                self.bump();
            }
        }

        let right = self
            .expect(TokenKindDiscriminant::RightParen, "expected ')'")?
            .clone();
        let span = join_spans(self.path, &function_span, &right.span);
        Ok(ConditionCall::new(function, args, span))
    }

    fn parse_argument(&mut self) -> Result<Argument, ParseError> {
        let token = self.current().clone();
        match token.kind {
            TokenKind::Ident(value) => {
                self.bump();
                Ok(Argument::identifier(value))
            }
            TokenKind::String(value) => {
                self.bump();
                Ok(ScalarValue::from(value).into())
            }
            TokenKind::Integer(value) => {
                self.bump();
                Ok(ScalarValue::from(value).into())
            }
            TokenKind::Float(value) => {
                self.bump();
                Ok(ScalarValue::from(value).into())
            }
            TokenKind::Boolean(value) => {
                self.bump();
                Ok(ScalarValue::from(value).into())
            }
            _ => Err(ParseError::new(token.span, "expected scalar argument")),
        }
    }

    fn expect_end(&mut self) -> Result<(), ParseError> {
        if self.at(TokenKindDiscriminant::End) {
            return Ok(());
        }

        Err(ParseError::new(
            self.current().span.clone(),
            "unexpected trailing tokens",
        ))
    }

    fn expect(
        &mut self,
        kind: TokenKindDiscriminant,
        message: &'static str,
    ) -> Result<&Token, ParseError> {
        if self.at(kind) {
            return Ok(self.bump());
        }

        Err(ParseError::new(self.current().span.clone(), message))
    }

    fn at(&self, kind: TokenKindDiscriminant) -> bool {
        self.current().kind.discriminant() == kind
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn bump(&mut self) -> &Token {
        let token = &self.tokens[self.cursor];
        self.cursor += 1;
        token
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TokenKindDiscriminant {
    Ident,
    String,
    Integer,
    Float,
    Boolean,
    And,
    Or,
    Not,
    LeftParen,
    RightParen,
    Comma,
    End,
}

impl TokenKind {
    fn discriminant(&self) -> TokenKindDiscriminant {
        match self {
            Self::Ident(_) => TokenKindDiscriminant::Ident,
            Self::String(_) => TokenKindDiscriminant::String,
            Self::Integer(_) => TokenKindDiscriminant::Integer,
            Self::Float(_) => TokenKindDiscriminant::Float,
            Self::Boolean(_) => TokenKindDiscriminant::Boolean,
            Self::And => TokenKindDiscriminant::And,
            Self::Or => TokenKindDiscriminant::Or,
            Self::Not => TokenKindDiscriminant::Not,
            Self::LeftParen => TokenKindDiscriminant::LeftParen,
            Self::RightParen => TokenKindDiscriminant::RightParen,
            Self::Comma => TokenKindDiscriminant::Comma,
            Self::End => TokenKindDiscriminant::End,
        }
    }
}

fn is_identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}

fn is_identifier_continue(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
}

fn join_spans(path: &str, start: &SourceSpan, end: &SourceSpan) -> SourceSpan {
    let end_position = end.end.unwrap_or(end.start);
    SourceSpan::new(path, start.start, Some(end_position))
}
