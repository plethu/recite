use recite_core::{Argument, ConditionCall, ConditionExpression, ScalarValue, SourceSpan};

use super::ParseError;
use super::token::{Token, TokenKind, TokenKindDiscriminant};

pub(super) struct Parser<'a> {
    path: &'a str,
    tokens: Vec<Token>,
    cursor: usize,
}

impl<'a> Parser<'a> {
    pub(super) fn new(path: &'a str, tokens: Vec<Token>) -> Self {
        Self {
            path,
            tokens,
            cursor: 0,
        }
    }

    pub(super) fn parse_or(&mut self) -> Result<ConditionExpression, ParseError> {
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

    pub(super) fn parse_call(&mut self) -> Result<ConditionCall, ParseError> {
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

    pub(super) fn expect_end(&mut self) -> Result<(), ParseError> {
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

fn join_spans(path: &str, start: &SourceSpan, end: &SourceSpan) -> SourceSpan {
    let end_position = end.end.unwrap_or(end.start);
    SourceSpan::new(path, start.start, Some(end_position))
}
