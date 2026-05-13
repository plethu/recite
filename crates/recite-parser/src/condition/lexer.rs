use recite_core::SourceSpan;

use crate::source::span_for_text;

use super::ParseError;
use super::token::{Token, TokenKind};

pub(super) struct Lexer<'a> {
    path: &'a str,
    line: u32,
    base_column: usize,
    text: &'a str,
    cursor: usize,
}

impl<'a> Lexer<'a> {
    pub(super) fn new(path: &'a str, line: u32, base_column: usize, text: &'a str) -> Self {
        Self {
            path,
            line,
            base_column,
            text,
            cursor: 0,
        }
    }

    pub(super) fn lex(mut self) -> Result<Vec<Token>, ParseError> {
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

fn is_identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}

fn is_identifier_continue(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
}
