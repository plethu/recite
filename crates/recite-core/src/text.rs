use std::collections::BTreeSet;

/// Placeholder syntax error in localisable project text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlaceholderSyntaxError {
    message: String,
}

impl PlaceholderSyntaxError {
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Placeholder mismatch between source text and a non-empty translation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlaceholderValidationError {
    missing: Vec<String>,
    extra: Vec<String>,
}

impl PlaceholderValidationError {
    #[must_use]
    pub fn missing(&self) -> &[String] {
        &self.missing
    }

    #[must_use]
    pub fn extra(&self) -> &[String] {
        &self.extra
    }

    #[must_use]
    pub fn message(&self) -> String {
        let mut parts = Vec::new();
        if !self.missing.is_empty() {
            parts.push(format!(
                "missing {}",
                self.missing
                    .iter()
                    .map(|name| format!("{{{name}}}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !self.extra.is_empty() {
            parts.push(format!(
                "extra {}",
                self.extra
                    .iter()
                    .map(|name| format!("{{{name}}}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        format!(
            "translation placeholders must match msgid: {}",
            parts.join("; ")
        )
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
                    return Err(placeholder_error("unterminated placeholder"));
                }
                if !is_placeholder_name(&name) {
                    return Err(placeholder_error(format!(
                        "invalid placeholder name '{name}'"
                    )));
                }
                placeholders.insert(name);
            }
            '}' => return Err(placeholder_error("unescaped closing brace")),
            _ => {}
        }
    }

    Ok(placeholders)
}

/// Validate that a non-empty translation preserves source placeholder names.
pub fn validate_translation_placeholders(
    source: &str,
    translation: &str,
) -> Result<(), PlaceholderValidationError> {
    let source_names = extract_placeholder_names(source).unwrap_or_default();
    let translation_names = extract_placeholder_names(translation).unwrap_or_default();
    let missing = source_names
        .difference(&translation_names)
        .cloned()
        .collect::<Vec<_>>();
    let extra = translation_names
        .difference(&source_names)
        .cloned()
        .collect::<Vec<_>>();

    if missing.is_empty() && extra.is_empty() {
        Ok(())
    } else {
        Err(PlaceholderValidationError { missing, extra })
    }
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

fn placeholder_error(message: impl Into<String>) -> PlaceholderSyntaxError {
    PlaceholderSyntaxError {
        message: message.into(),
    }
}
