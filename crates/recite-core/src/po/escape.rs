pub(super) fn escape(value: &str) -> String {
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
