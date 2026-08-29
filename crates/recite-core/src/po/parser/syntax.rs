use super::diagnostics::{PoDiagnostic, error};
use super::types::SourceLine;

pub(crate) fn format_field(
    keyword: &str,
    value: &str,
    multiline: bool,
    ending: &str,
    obsolete: bool,
) -> String {
    let prefix = if obsolete { "#~ " } else { "" };
    if multiline {
        format!(
            "{prefix}{keyword} \"\"{ending}{prefix}\"{}\"",
            escape(value)
        )
    } else {
        format!("{prefix}{keyword} \"{}\"", escape(value))
    }
}

pub(super) fn parse_quoted(
    name: &str,
    line: usize,
    input: &str,
) -> Result<String, super::PoParseError> {
    let mut chars = input.chars();
    if chars.next() != Some('"') {
        return Err(error(name, line, PoDiagnostic::ExpectedQuotedString));
    }
    let mut output = String::new();
    let mut escaped = false;
    while let Some(character) = chars.next() {
        if escaped {
            let value = match character {
                'a' => '\x07',
                'b' => '\x08',
                'f' => '\x0c',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                'v' => '\x0b',
                '"' => '"',
                '\\' => '\\',
                '0'..='7' => {
                    let mut digits = String::from(character);
                    for _ in 0..2 {
                        let Some(next) = chars.clone().next() else {
                            break;
                        };
                        if !matches!(next, '0'..='7') {
                            break;
                        }
                        digits.push(next);
                        let _ = chars.next();
                    }
                    char::from(u8::from_str_radix(&digits, 8).map_err(|_| {
                        error(
                            name,
                            line,
                            PoDiagnostic::UnsupportedEscape(format!("\\{digits}")),
                        )
                    })?)
                }
                'x' => {
                    let mut digits = String::new();
                    for _ in 0..2 {
                        let Some(next) = chars.next() else {
                            return Err(error(
                                name,
                                line,
                                PoDiagnostic::UnsupportedEscape("\\x".to_owned()),
                            ));
                        };
                        if !next.is_ascii_hexdigit() {
                            return Err(error(
                                name,
                                line,
                                PoDiagnostic::UnsupportedEscape(format!("\\x{next}")),
                            ));
                        }
                        digits.push(next);
                    }
                    char::from(u8::from_str_radix(&digits, 16).map_err(|_| {
                        error(
                            name,
                            line,
                            PoDiagnostic::UnsupportedEscape(format!("\\x{digits}")),
                        )
                    })?)
                }
                other => {
                    return Err(error(
                        name,
                        line,
                        PoDiagnostic::UnsupportedEscape(format!("\\{other}")),
                    ));
                }
            };
            output.push(value);
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' if chars.as_str().trim().is_empty() => return Ok(output),
            '"' => {
                return Err(error(
                    name,
                    line,
                    PoDiagnostic::UnexpectedTextAfterQuotedString,
                ));
            }
            other => output.push(other),
        }
    }
    Err(error(
        name,
        line,
        if escaped {
            PoDiagnostic::UnsupportedEscape("dangling backslash".to_owned())
        } else {
            PoDiagnostic::UnterminatedQuotedString
        },
    ))
}

pub(super) fn source_lines(source: &str) -> Vec<SourceLine> {
    let mut lines = Vec::new();
    let mut start = 0;
    while start < source.len() {
        let end = source[start..]
            .find('\n')
            .map_or(source.len(), |offset| start + offset + 1);
        let mut content_end = end;
        if source.as_bytes().get(content_end.wrapping_sub(1)) == Some(&b'\n') {
            content_end -= 1;
        }
        if source.as_bytes().get(content_end.wrapping_sub(1)) == Some(&b'\r') {
            content_end -= 1;
        }
        lines.push(SourceLine {
            number: lines.len() + 1,
            start,
            content_end,
            end,
            text: source[start..content_end].to_owned(),
        });
        start = end;
    }
    lines
}

fn escape(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\x07' => output.push_str("\\a"),
            '\x08' => output.push_str("\\b"),
            '\x0c' => output.push_str("\\f"),
            '\x0b' => output.push_str("\\v"),
            character if character.is_control() => {
                use std::fmt::Write as _;
                let _ = write!(output, "\\{:03o}", character as u32);
            }
            character => output.push(character),
        }
    }
    output
}
