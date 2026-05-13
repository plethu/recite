use recite_core::{ScalarValue, Value};

pub(super) fn parse_value(value: &str) -> Result<Value, ()> {
    if value.starts_with('[') {
        return parse_array(value).map(Value::Array);
    }

    parse_scalar(value).map(Value::Scalar)
}

fn parse_array(value: &str) -> Result<Vec<ScalarValue>, ()> {
    let inner = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or(())?;
    let trimmed = inner.trim();

    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    split_array_items(trimmed)?
        .into_iter()
        .map(|item| parse_scalar(item.trim()))
        .collect()
}

fn split_array_items(value: &str) -> Result<Vec<&str>, ()> {
    let mut items = Vec::new();
    let mut quote = None;
    let mut start = 0;
    let mut cursor = 0;

    while let Some(character) = value[cursor..].chars().next() {
        match character {
            '\\' if quote.is_some() => {
                cursor += character.len_utf8();
                if let Some(escaped) = value[cursor..].chars().next() {
                    cursor += escaped.len_utf8();
                }
            }
            '"' if quote == Some('"') => {
                quote = None;
                cursor += character.len_utf8();
            }
            '"' if quote.is_none() => {
                quote = Some('"');
                cursor += character.len_utf8();
            }
            ',' if quote.is_none() => {
                let item = value[start..cursor].trim();
                if item.is_empty() {
                    return Err(());
                }
                items.push(item);
                cursor += character.len_utf8();
                start = cursor;
            }
            _ => cursor += character.len_utf8(),
        }
    }

    if quote.is_some() {
        return Err(());
    }

    let item = value[start..].trim();
    if item.is_empty() {
        return Err(());
    }
    items.push(item);
    Ok(items)
}

fn parse_scalar(value: &str) -> Result<ScalarValue, ()> {
    if value == "true" {
        return Ok(ScalarValue::Boolean(true));
    }

    if value == "false" {
        return Ok(ScalarValue::Boolean(false));
    }

    if value.starts_with('"') {
        return unquote(value).map(ScalarValue::String);
    }

    if value.starts_with('[') || value.ends_with(']') || value.contains('"') {
        return Err(());
    }

    if let Ok(integer) = value.parse::<i64>() {
        return Ok(ScalarValue::Integer(integer));
    }

    if let Ok(float) = value.parse::<f64>() {
        return Ok(ScalarValue::Float(float));
    }

    Ok(ScalarValue::String(value.to_owned()))
}

fn unquote(value: &str) -> Result<String, ()> {
    let inner = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or(())?;
    let mut output = String::new();
    let mut chars = inner.chars();

    while let Some(character) = chars.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }

        let Some(escaped) = chars.next() else {
            return Err(());
        };
        match escaped {
            '"' => output.push('"'),
            '\\' => output.push('\\'),
            'n' => output.push('\n'),
            't' => output.push('\t'),
            other => output.push(other),
        }
    }

    Ok(output)
}
