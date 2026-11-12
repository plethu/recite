//! A small RFC 4180 reader and writer for handoff sheets.

/// Write one record, quoting fields that need it, terminated by CRLF.
pub(super) fn write_record<'a>(output: &mut String, fields: impl IntoIterator<Item = &'a str>) {
    for (index, field) in fields.into_iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let quote = field.contains([',', '"', '\n', '\r'])
            || field.starts_with(' ')
            || field.ends_with(' ');
        if quote {
            output.push('"');
            output.push_str(&field.replace('"', "\"\""));
            output.push('"');
        } else {
            output.push_str(field);
        }
    }
    output.push_str("\r\n");
}

/// Records with the line each starts on, or the line and reason a sheet cannot be read.
type Records = Result<Vec<(usize, Vec<String>)>, (usize, &'static str)>;

/// Read records with the line each one starts on. LF and CRLF endings and a UTF-8 byte order mark
/// are accepted; blank lines are skipped.
pub(super) fn read_records(input: &str) -> Records {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let mut records = Vec::new();
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut line = 1;
    let mut start = 1;
    let mut chars = input.chars().peekable();
    let mut at_field_start = true;
    while let Some(character) = chars.next() {
        if quoted {
            match character {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => {
                    quoted = false;
                    if !matches!(chars.peek(), None | Some(',' | '\r' | '\n')) {
                        return Err((line, "text after a closing quote"));
                    }
                }
                '\n' => {
                    line += 1;
                    field.push('\n');
                }
                _ => field.push(character),
            }
            continue;
        }
        match character {
            '"' if at_field_start => {
                quoted = true;
                at_field_start = false;
            }
            '"' => return Err((line, "quote inside an unquoted field")),
            ',' => {
                fields.push(std::mem::take(&mut field));
                at_field_start = true;
            }
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' | '\r' => {
                fields.push(std::mem::take(&mut field));
                if !(fields.len() == 1 && fields[0].is_empty()) {
                    records.push((start, std::mem::take(&mut fields)));
                }
                fields.clear();
                line += 1;
                start = line;
                at_field_start = true;
            }
            _ => {
                field.push(character);
                at_field_start = false;
            }
        }
    }
    if quoted {
        return Err((start, "unterminated quoted field"));
    }
    if !field.is_empty() || !fields.is_empty() {
        fields.push(field);
        records.push((start, fields));
    }
    Ok(records)
}
