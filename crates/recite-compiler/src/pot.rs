use recite_core::{
    Diagnostic, SourceSpan, SpeakerId,
    ast::{Block, SourceFile, Statement},
    po::{PotDocument, PotEntry, PotReference},
    schema::ProjectSchema,
};
use recite_parser::parse;

use crate::compile::CompileInput;
use crate::validation::{
    project::{sort_diagnostics_by_source, source_files_in_project_order},
    validate_inputs, validate_source_files,
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
        validate_inputs(
            source_files
                .iter()
                .map(crate::validation::ValidationInput::all_complete),
            Some(schema),
            crate::validation::ProjectCompleteness::Complete,
        )
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
                    plural_source_text: None,
                    comments: vec!["speaker display name".to_owned()],
                    reference: None,
                });
            }
        }
        for (reason_id, reason) in &schema.availability_reasons {
            entries.push(PotEntry {
                context: format!("availability_reason:{reason_id}"),
                source_text: reason.template.clone(),
                plural_source_text: None,
                comments: vec!["availability reason template".to_owned()],
                reference: None,
            });
        }
        for projector in schema.presentation_projectors.values() {
            for (output_id, output) in &projector.outputs {
                if let Some(label) = &output.label {
                    entries.push(PotEntry {
                        context: format!("presentation_label:{}", label.template_id),
                        source_text: label.source_text.clone(),
                        plural_source_text: None,
                        comments: vec![format!("presentation label template: {output_id}")],
                        reference: None,
                    });
                }
            }
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
                    PotEntryInput {
                        context: id.as_str(),
                        source_id_display: line.source_id.display_text().as_deref(),
                        source_text: &line.source_text.text,
                        plural_source_text: line
                            .plural_source_text
                            .as_ref()
                            .map(|text| text.text.as_str()),
                    },
                    PotEntryContext {
                        source_file,
                        block,
                        speaker,
                        span: &line.source_text.span,
                    },
                ));
            }

            for statement in &line.statements {
                extract_statement_entries(entries, source_file, block, statement, speaker);
            }
        }
        Statement::Choice(choice) => {
            if let Some(id) = &choice.id {
                entries.push(source_entry(
                    PotEntryInput {
                        context: id.as_str(),
                        source_id_display: choice.source_id.display_text().as_deref(),
                        source_text: &choice.source_text.text,
                        plural_source_text: None,
                    },
                    PotEntryContext {
                        source_file,
                        block,
                        speaker: speaker_context,
                        span: &choice.source_text.span,
                    },
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

struct PotEntryInput<'a> {
    context: &'a str,
    source_id_display: Option<&'a str>,
    source_text: &'a str,
    plural_source_text: Option<&'a str>,
}

struct PotEntryContext<'a> {
    source_file: &'a SourceFile,
    block: &'a Block,
    speaker: Option<&'a SpeakerId>,
    span: &'a SourceSpan,
}

fn source_entry(input: PotEntryInput<'_>, context: PotEntryContext<'_>) -> PotEntry {
    let mut comments = vec![
        format!("file: {}", context.source_file.path),
        format!("block: {}", context.block.id),
    ];
    if let Some(speaker) = context.speaker {
        comments.push(format!("speaker: {speaker}"));
    }
    if let Some(source_id_display) = input.source_id_display {
        comments.push(format!("source id: {source_id_display}"));
    }

    PotEntry {
        context: input.context.to_owned(),
        source_text: input.source_text.to_owned(),
        plural_source_text: input.plural_source_text.map(str::to_owned),
        comments,
        reference: Some(PotReference {
            file: context.span.file.clone(),
            line: context.span.start.line(),
            column: context.span.start.column(),
        }),
    }
}
