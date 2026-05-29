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
        let first_span = first.span().clone();
        let mut last_span = first_span.clone();
        let mut expressions = vec![first];

        while self.at(TokenKindDiscriminant::Or) {
            self.bump();
            let expression = self.parse_and()?;
            last_span = expression.span().clone();
            expressions.push(expression);
        }

        if expressions.len() == 1 {
            return Ok(expressions.remove(0));
        }

        let span = join_spans(self.path, &first_span, &last_span);
        Ok(ConditionExpression::or(expressions, span))
    }

    fn parse_and(&mut self) -> Result<ConditionExpression, ParseError> {
        let first = self.parse_unary()?;
        let first_span = first.span().clone();
        let mut last_span = first_span.clone();
        let mut expressions = vec![first];

        while self.at(TokenKindDiscriminant::And) {
            self.bump();
            let expression = self.parse_unary()?;
            last_span = expression.span().clone();
            expressions.push(expression);
        }

        if expressions.len() == 1 {
            return Ok(expressions.remove(0));
        }

        let span = join_spans(self.path, &first_span, &last_span);
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
        let mut arg_spans = Vec::new();
        if !self.at(TokenKindDiscriminant::RightParen) {
            loop {
                let (argument, span) = self.parse_argument()?;
                args.push(argument);
                arg_spans.push(span);
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
        Ok(ConditionCall::new(function, args, span).with_source_spans(function_span, arg_spans))
    }

    fn parse_argument(&mut self) -> Result<(Argument, SourceSpan), ParseError> {
        let token = self.current().clone();
        let span = token.span;
        match token.kind {
            TokenKind::Ident(value) => {
                self.bump();
                Ok((Argument::identifier(value), span))
            }
            TokenKind::String(value) => {
                self.bump();
                Ok((ScalarValue::from(value).into(), span))
            }
            TokenKind::Integer(value) => {
                self.bump();
                Ok((ScalarValue::from(value).into(), span))
            }
            TokenKind::Float(value) => {
                self.bump();
                Ok((ScalarValue::from(value).into(), span))
            }
            TokenKind::Boolean(value) => {
                self.bump();
                Ok((ScalarValue::from(value).into(), span))
            }
            _ => Err(ParseError::new(span, "expected scalar argument")),
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
