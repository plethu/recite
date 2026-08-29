use std::collections::BTreeSet;

/// Placeholder syntax error in localisable project text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlaceholderSyntaxError {
    message: String,
    kind: PlaceholderSyntaxKind,
}

/// The finite syntax-error taxonomy used by compiler diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum PlaceholderSyntaxKind {
    Unterminated,
    InvalidName(String),
    UnescapedClosingBrace,
}

impl PlaceholderSyntaxError {
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub fn kind(&self) -> &PlaceholderSyntaxKind {
        &self.kind
    }
}

/// Extract Recite interpolation placeholders from localisable text.
pub fn extract_placeholder_names(source: &str) -> Result<BTreeSet<String>, PlaceholderSyntaxError> {
    let mut placeholders = BTreeSet::new();
    let mut chars = source.char_indices().peekable();

    while let Some((_, character)) = chars.next() {
        match character {
            '\\' => {
                if matches!(chars.peek(), Some((_, '{' | '}'))) {
                    chars.next();
                }
            }
            '{' => {
                let mut name = String::new();
                let mut closed = false;
                for (_, inner) in chars.by_ref() {
                    if inner == '}' {
                        closed = true;
                        break;
                    }
                    name.push(inner);
                }
                if !closed {
                    return Err(placeholder_error(
                        PlaceholderSyntaxKind::Unterminated,
                        "unterminated placeholder",
                    ));
                }
                if !is_placeholder_name(&name) {
                    return Err(placeholder_error(
                        PlaceholderSyntaxKind::InvalidName(name.clone()),
                        format!("invalid placeholder name '{name}'"),
                    ));
                }
                placeholders.insert(name);
            }
            '}' => {
                return Err(placeholder_error(
                    PlaceholderSyntaxKind::UnescapedClosingBrace,
                    "unescaped closing brace",
                ));
            }
            _ => {}
        }
    }

    Ok(placeholders)
}

/// Extract placeholder names in source order, retaining repeated occurrences.
pub fn extract_placeholder_occurrences(
    source: &str,
) -> Result<Vec<String>, PlaceholderSyntaxError> {
    let mut occurrences = Vec::new();
    let mut chars = source.char_indices().peekable();
    while let Some((_, character)) = chars.next() {
        match character {
            '\\' => {
                if matches!(chars.peek(), Some((_, '{' | '}'))) {
                    chars.next();
                }
            }
            '{' => {
                let mut name = String::new();
                let mut closed = false;
                for (_, inner) in chars.by_ref() {
                    if inner == '}' {
                        closed = true;
                        break;
                    }
                    name.push(inner);
                }
                if !closed {
                    return Err(placeholder_error(
                        PlaceholderSyntaxKind::Unterminated,
                        "unterminated placeholder",
                    ));
                }
                if !is_placeholder_name(&name) {
                    return Err(placeholder_error(
                        PlaceholderSyntaxKind::InvalidName(name.clone()),
                        format!("invalid placeholder name '{name}'"),
                    ));
                }
                occurrences.push(name);
            }
            '}' => {
                return Err(placeholder_error(
                    PlaceholderSyntaxKind::UnescapedClosingBrace,
                    "unescaped closing brace",
                ));
            }
            _ => {}
        }
    }
    Ok(occurrences)
}

/// Decode escaped literal braces for runtime use while preserving other text.
pub fn decode_interpolation_text(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut chars = source.chars();
    while let Some(character) = chars.next() {
        if character == '\\' {
            if let Some(next) = chars.next() {
                if matches!(next, '{' | '}') {
                    output.push(next);
                } else {
                    output.push(character);
                    output.push(next);
                }
            } else {
                output.push(character);
            }
        } else {
            output.push(character);
        }
    }
    output
}

fn is_placeholder_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    first.is_ascii_lowercase()
        && chars.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn placeholder_error(
    kind: PlaceholderSyntaxKind,
    message: impl Into<String>,
) -> PlaceholderSyntaxError {
    PlaceholderSyntaxError {
        message: message.into(),
        kind,
    }
}
