mod lexer;
mod parser;
mod token;

use recite_core::{ConditionCall, ConditionExpression, SourceSpan};

use self::lexer::Lexer;
use self::parser::Parser;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ParseError {
    pub(crate) span: SourceSpan,
    pub(crate) message: String,
}

impl ParseError {
    pub(super) fn new(span: SourceSpan, message: impl Into<String>) -> Self {
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
