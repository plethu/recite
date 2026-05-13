use recite_core::{
    BlockId, BlockReference, ChoiceEcho, DivertTarget, EffectMode, LineId, Metadata, MetadataEntry,
    SourceSpan, SpeakerId,
};

use crate::diagnostics::{malformed_divert_target, malformed_header};
use crate::header::{HeaderField, HeaderKeyValue};

use super::Lowerer;

impl Lowerer<'_, '_> {
    pub(super) fn lower_speaker_metadata(
        &mut self,
        fields: &[HeaderField<'_>],
    ) -> (Option<SpeakerId>, Metadata) {
        let mut speaker = None;
        let mut metadata = Metadata::new();

        for field in fields.iter().copied() {
            let Some(kv) = self.valid_key_value(field) else {
                continue;
            };

            if kv.key == "speaker" {
                speaker = self.speaker_from_value(&kv);
                continue;
            }

            if let Some(entry) = self.metadata_entry(kv) {
                metadata.push(entry);
            }
        }

        (speaker, metadata)
    }

    pub(super) fn lower_choice_metadata(
        &mut self,
        fields: &[HeaderField<'_>],
    ) -> (Metadata, ChoiceEcho) {
        let mut metadata = Metadata::new();
        let mut echo = ChoiceEcho::None;

        for field in fields.iter().copied() {
            let Some(kv) = self.valid_key_value(field) else {
                continue;
            };

            if kv.key == "echo" {
                if let Some(parsed) = choice_echo(kv.value) {
                    echo = parsed;
                } else {
                    self.diagnostics.push(malformed_header(kv.value_span));
                }
                continue;
            }

            if let Some(entry) = self.metadata_entry(kv) {
                metadata.push(entry);
            }
        }

        (metadata, echo)
    }

    pub(super) fn valid_key_value<'a>(
        &mut self,
        field: HeaderField<'a>,
    ) -> Option<HeaderKeyValue<'a>> {
        let Some(kv) = field.key_value(self.path) else {
            self.diagnostics
                .push(malformed_header(field.span(self.path)));
            return None;
        };

        if kv.key.is_empty() || kv.value.is_empty() {
            self.diagnostics
                .push(malformed_header(kv.field_span.clone()));
            return None;
        }

        Some(kv)
    }

    pub(super) fn speaker_from_value(&mut self, kv: &HeaderKeyValue<'_>) -> Option<SpeakerId> {
        match SpeakerId::new(kv.value) {
            Ok(speaker) => Some(speaker),
            Err(_) => {
                self.diagnostics
                    .push(malformed_header(kv.value_span.clone()));
                None
            }
        }
    }

    pub(super) fn metadata_entry(&mut self, kv: HeaderKeyValue<'_>) -> Option<MetadataEntry> {
        match metadata_entry(kv) {
            Ok(entry) => Some(entry),
            Err(span) => {
                self.diagnostics.push(malformed_header(span));
                None
            }
        }
    }

    pub(super) fn divert_target(&mut self, field: HeaderField<'_>) -> Option<DivertTarget> {
        if field.text == "END" {
            return Some(DivertTarget::End);
        }

        let reference = if let Some((file, block_id)) = field.text.split_once("::") {
            if file.is_empty() || block_id.is_empty() || block_id.contains("::") {
                self.diagnostics
                    .push(malformed_divert_target(field.span(self.path)));
                return None;
            }

            let Ok(block_id) = BlockId::new(block_id) else {
                self.diagnostics
                    .push(malformed_divert_target(field.span(self.path)));
                return None;
            };

            BlockReference::external(file, block_id)
        } else {
            let Ok(block_id) = BlockId::new(field.text) else {
                self.diagnostics
                    .push(malformed_divert_target(field.span(self.path)));
                return None;
            };

            BlockReference::local(block_id)
        };

        Some(DivertTarget::Block(reference))
    }
}

pub(super) fn effect_mode(value: &str) -> Option<EffectMode> {
    match value {
        "deferred" => Some(EffectMode::Deferred),
        "immediate" => Some(EffectMode::Immediate),
        "blocking" => Some(EffectMode::Blocking),
        _ => None,
    }
}

fn metadata_entry(kv: HeaderKeyValue<'_>) -> Result<MetadataEntry, SourceSpan> {
    let value = kv.parse_value()?;

    Ok(MetadataEntry::new(kv.key, value)
        .with_source_span(kv.field_span)
        .with_key_value_spans(kv.key_span, Some(kv.value_span)))
}

fn choice_echo(value: &str) -> Option<ChoiceEcho> {
    match value {
        "none" => Some(ChoiceEcho::None),
        "selected_text" => Some(ChoiceEcho::SelectedText),
        _ => {
            let line_id = value.strip_prefix("line(")?.strip_suffix(')')?;
            Some(ChoiceEcho::Line(LineId::new(line_id).ok()?))
        }
    }
}
