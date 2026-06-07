use recite_core::{Block, Diagnostic, ProjectSchema, SourceFile, SourceSpan, SpeakerId, Statement};
use recite_parser::parse;

use crate::compile::CompileInput;
use crate::validation::{
    project::{sort_diagnostics_by_source, source_files_in_project_order},
    validate_source_files, validate_source_files_with_schema,
};

/// Result of extracting gettext POT entries from raw Recite inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotExtractionReport {
    pub diagnostics: Vec<Diagnostic>,
    pub catalog: Option<PotDocument>,
}

impl PotExtractionReport {
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.diagnostics.is_empty() && self.catalog.is_some()
    }
}

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
            output.push('\n');
            output.push_str("msgstr \"\"\n");
        }

        output
    }
}

/// One gettext entry extracted from Recite source or project schema content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotEntry {
    pub context: String,
    pub source_text: String,
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

/// Extract localisable line and choice entries from raw source inputs.
#[must_use]
pub fn extract_pot(inputs: impl IntoIterator<Item = CompileInput>) -> PotExtractionReport {
    extract_pot_with_optional_schema(inputs, None)
}

/// Extract localisable line, choice, and schema speaker display-name entries
/// from raw source inputs.
#[must_use]
pub fn extract_pot_with_schema(
    inputs: impl IntoIterator<Item = CompileInput>,
    schema: &ProjectSchema,
) -> PotExtractionReport {
    extract_pot_with_optional_schema(inputs, Some(schema))
}

fn extract_pot_with_optional_schema(
    inputs: impl IntoIterator<Item = CompileInput>,
    schema: Option<&ProjectSchema>,
) -> PotExtractionReport {
    let mut source_files = Vec::new();
    let mut diagnostics = Vec::new();

    for input in inputs {
        let parse = parse(&input.path, &input.source);
        let lowered = parse.lower_source_file();
        diagnostics.extend(lowered.diagnostics);
        source_files.push(lowered.source_file);
    }

    sort_diagnostics_by_source(&mut diagnostics);
    if !diagnostics.is_empty() {
        return PotExtractionReport {
            diagnostics,
            catalog: None,
        };
    }

    let validation = if let Some(schema) = schema {
        validate_source_files_with_schema(&source_files, schema)
    } else {
        validate_source_files(&source_files)
    };
    if !validation.is_ok() {
        return PotExtractionReport {
            diagnostics: validation.diagnostics,
            catalog: None,
        };
    }

    PotExtractionReport {
        diagnostics: Vec::new(),
        catalog: Some(collect_pot(&source_files, schema)),
    }
}

fn collect_pot(source_files: &[SourceFile], schema: Option<&ProjectSchema>) -> PotDocument {
    let mut entries = Vec::new();
    let ordered_files = source_files_in_project_order(source_files);

    for source_file in ordered_files {
        for block in &source_file.blocks {
            extract_block_entries(&mut entries, source_file, block);
        }
    }

    if let Some(schema) = schema {
        for (speaker_id, speaker) in &schema.speakers {
            if let Some(display_name) = &speaker.display_name {
                entries.push(PotEntry {
                    context: format!("dialogue_speaker:{speaker_id}"),
                    source_text: display_name.clone(),
                    comments: vec!["speaker display name".to_owned()],
                    reference: None,
                });
            }
        }
        for (reason_id, reason) in &schema.availability_reasons {
            entries.push(PotEntry {
                context: format!("availability_reason:{reason_id}"),
                source_text: reason.template.clone(),
                comments: vec!["availability reason template".to_owned()],
                reference: None,
            });
        }
    }

    PotDocument { entries }
}

fn extract_block_entries(entries: &mut Vec<PotEntry>, source_file: &SourceFile, block: &Block) {
    for statement in &block.statements {
        extract_statement_entries(
            entries,
            source_file,
            block,
            statement,
            block.default_speaker.as_ref(),
        );
    }
}

fn extract_statement_entries(
    entries: &mut Vec<PotEntry>,
    source_file: &SourceFile,
    block: &Block,
    statement: &Statement,
    speaker_context: Option<&SpeakerId>,
) {
    match statement {
        Statement::Line(line) => {
            let speaker = line.speaker.as_ref().or(speaker_context);
            if let Some(id) = &line.id {
                entries.push(source_entry(
                    id.as_str(),
                    line.source_id.display_text().as_deref(),
                    &line.source_text.text,
                    source_file,
                    block,
                    speaker,
                    &line.source_text.span,
                ));
            }

            for statement in &line.statements {
                extract_statement_entries(entries, source_file, block, statement, speaker);
            }
        }
        Statement::Choice(choice) => {
            if let Some(id) = &choice.id {
                entries.push(source_entry(
                    id.as_str(),
                    choice.source_id.display_text().as_deref(),
                    &choice.source_text.text,
                    source_file,
                    block,
                    speaker_context,
                    &choice.source_text.span,
                ));
            }

            for statement in &choice.statements {
                extract_statement_entries(entries, source_file, block, statement, speaker_context);
            }
        }
        Statement::If(branch) => {
            for statement in &branch.then_statements {
                extract_statement_entries(entries, source_file, block, statement, speaker_context);
            }
            for statement in &branch.else_statements {
                extract_statement_entries(entries, source_file, block, statement, speaker_context);
            }
        }
        Statement::Match(branch) => {
            for arm in &branch.arms {
                for statement in &arm.statements {
                    extract_statement_entries(
                        entries,
                        source_file,
                        block,
                        statement,
                        speaker_context,
                    );
                }
            }
        }
        Statement::Divert(_) | Statement::Effect(_) | Statement::Comment(_) => {}
    }
}

fn source_entry(
    context: &str,
    source_id_display: Option<&str>,
    source_text: &str,
    source_file: &SourceFile,
    block: &Block,
    speaker: Option<&SpeakerId>,
    span: &SourceSpan,
) -> PotEntry {
    let mut comments = vec![
        format!("file: {}", source_file.path),
        format!("block: {}", block.id),
    ];
    if let Some(speaker) = speaker {
        comments.push(format!("speaker: {speaker}"));
    }
    if let Some(source_id_display) = source_id_display {
        comments.push(format!("source id: {source_id_display}"));
    }

    PotEntry {
        context: context.to_owned(),
        source_text: source_text.to_owned(),
        comments,
        reference: Some(PotReference {
            file: span.file.clone(),
            line: span.start.line(),
            column: span.start.column(),
        }),
    }
}

fn push_po_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character => output.push(character),
        }
    }
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
