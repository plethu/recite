mod lexer;
mod parser;
mod token;

use recite_core::{ConditionCall, ConditionExpression, SourceSpan};
use std::fmt;

use self::lexer::Lexer;
use self::parser::Parser;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ParseError {
    pub(crate) span: SourceSpan,
    pub(crate) kind: ParseErrorKind,
}

impl ParseError {
    pub(super) const fn new(span: SourceSpan, kind: ParseErrorKind) -> Self {
        Self { span, kind }
    }

    pub(crate) fn compatibility_message(&self) -> String {
        self.kind.to_string()
    }
}

/// Stable parser error selectors. The parser keeps these structured until the
/// diagnostic presentation builder chooses typed localisable arguments.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ParseErrorKind {
    UnexpectedCharacter(char),
    UnterminatedString,
    InvalidFloat,
    InvalidInteger,
    ExpectedFunctionCall,
    ExpectedFunctionNameParen,
    ExpectedRightParen,
    ExpectedScalarArgument,
    UnexpectedTrailingTokens,
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedCharacter(character) => {
                write!(formatter, "unexpected character {character:?}")
            }
            Self::UnterminatedString => formatter.write_str("unterminated string literal"),
            Self::InvalidFloat => formatter.write_str("invalid float literal"),
            Self::InvalidInteger => formatter.write_str("invalid integer literal"),
            Self::ExpectedFunctionCall => formatter.write_str("expected function call"),
            Self::ExpectedFunctionNameParen => {
                formatter.write_str("expected '(' after function name")
            }
            Self::ExpectedRightParen => formatter.write_str("expected ')'"),
            Self::ExpectedScalarArgument => formatter.write_str("expected scalar argument"),
            Self::UnexpectedTrailingTokens => formatter.write_str("unexpected trailing tokens"),
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
