use super::escape::escape;

/// Deterministic gettext POT extraction output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotDocument {
    pub entries: Vec<PotEntry>,
}

impl PotDocument {
    #[must_use]
    pub fn to_pot_string(&self) -> String {
        let mut output = String::new();

        for (index, entry) in self.entries.iter().enumerate() {
            if index > 0 {
                output.push('\n');
            }

            for comment in &entry.comments {
                output.push_str("#. ");
                push_po_comment_text(&mut output, comment);
                output.push('\n');
            }

            if let Some(reference) = &entry.reference {
                output.push_str("#: ");
                push_po_reference_text(&mut output, &reference.file);
                output.push(':');
                output.push_str(&reference.line.to_string());
                output.push(':');
                output.push_str(&reference.column.to_string());
                output.push('\n');
            }

            output.push_str("msgctxt ");
            push_po_string(&mut output, &entry.context);
            output.push('\n');
            output.push_str("msgid ");
            push_po_string(&mut output, &entry.source_text);
            if let Some(plural_source_text) = &entry.plural_source_text {
                output.push('\n');
                output.push_str("msgid_plural ");
                push_po_string(&mut output, plural_source_text);
                output.push('\n');
                output.push_str("msgstr[0] \"\"\n");
                output.push_str("msgstr[1] \"\"\n");
            } else {
                output.push('\n');
                output.push_str("msgstr \"\"\n");
            }
        }

        output
    }
}

/// One gettext entry extracted from Recite source or project schema content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotEntry {
    pub context: String,
    pub source_text: String,
    pub plural_source_text: Option<String>,
    pub comments: Vec<String>,
    pub reference: Option<PotReference>,
}

/// Source location attached to a POT entry when available.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotReference {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

fn push_po_string(output: &mut String, value: &str) {
    output.push('"');
    output.push_str(&escape(value));
    output.push('"');
}

fn push_po_comment_text(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '\n' | '\r' | '\t' => output.push(' '),
            character if character.is_control() => output.push(' '),
            character => output.push(character),
        }
    }
}

fn push_po_reference_text(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '\n' | '\r' | '\t' | ':' => output.push('_'),
            character if character.is_control() => output.push('_'),
            character => output.push(character),
        }
    }
}
