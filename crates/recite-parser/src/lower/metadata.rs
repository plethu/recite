use recite_core::{
    BlockId, BlockReference, ChoiceEcho, DivertTarget, EffectMode, InterpolationBinding,
    InterpolationType, SourceMetadata, SourceMetadataEntry, SourceRecoveryClass, SpeakerId,
};

use crate::diagnostics::{malformed_divert_target, malformed_header};
use crate::header::{HeaderField, HeaderKeyValue};
use crate::source::span_for_text;

use super::Lowerer;
use super::metadata_values::{choice_echo, is_placeholder_name, metadata_entry};

impl Lowerer<'_, '_> {
    pub(super) fn lower_speaker_metadata(
        &mut self,
        fields: &[HeaderField<'_>],
    ) -> (Option<SpeakerId>, SourceMetadata, Vec<InterpolationBinding>) {
        let mut speaker = None;
        let mut metadata = SourceMetadata::new();
        let mut bindings = Vec::new();

        for field in fields.iter().copied() {
            let Some(kv) = self.valid_key_value(field) else {
                continue;
            };

            if kv.key == "speaker" {
                speaker = self.speaker_from_value(&kv);
                continue;
            }

            if kv.key == "bind" {
                if let Some(binding) = self.interpolation_binding(&kv) {
                    bindings.push(binding);
                }
                continue;
            }

            if let Some(entry) = self.metadata_entry(kv) {
                metadata.push(entry);
            }
        }

        (speaker, metadata, bindings)
    }

    pub(super) fn lower_choice_metadata(
        &mut self,
        fields: &[HeaderField<'_>],
    ) -> (SourceMetadata, ChoiceEcho, Vec<InterpolationBinding>) {
        let mut metadata = SourceMetadata::new();
        let mut echo = ChoiceEcho::None;
        let mut bindings = Vec::new();

        for field in fields.iter().copied() {
            let Some(kv) = self.valid_key_value(field) else {
                continue;
            };

            if kv.key == "echo" {
                if let Some(parsed) = choice_echo(kv.value) {
                    echo = parsed;
                } else {
                    self.mark(SourceRecoveryClass::Metadata);
                    self.diagnostics.push(malformed_header(kv.value_span));
                }
                continue;
            }

            if kv.key == "bind" {
                if let Some(binding) = self.interpolation_binding(&kv) {
                    bindings.push(binding);
                }
                continue;
            }

            if let Some(entry) = self.metadata_entry(kv) {
                metadata.push(entry);
            }
        }

        (metadata, echo, bindings)
    }

    fn interpolation_binding(&mut self, kv: &HeaderKeyValue<'_>) -> Option<InterpolationBinding> {
        let Some(value) = kv
            .value
            .strip_prefix("(")
            .and_then(|value| value.strip_suffix(')'))
        else {
            self.diagnostics
                .push(malformed_header(kv.value_span.clone()));
            return None;
        };
        let Some((name, value)) = value.split_once(':') else {
            self.diagnostics
                .push(malformed_header(kv.value_span.clone()));
            return None;
        };
        let Some((value_type, value)) = value.split_once("=$") else {
            self.diagnostics
                .push(malformed_header(kv.value_span.clone()));
            return None;
        };
        let value_type = match value_type {
            "string" => InterpolationType::String,
            "int" => InterpolationType::Integer,
            "float" => InterpolationType::Float,
            "bool" => InterpolationType::Boolean,
            _ => {
                let type_column =
                    kv.value_span.start.column() as usize + 1 + name.chars().count() + 1;
                self.diagnostics
                    .push(malformed_header(crate::source::span_for_text(
                        self.path,
                        kv.value_span.start.line(),
                        type_column,
                        value_type,
                    )));
                return None;
            }
        };
        if !is_placeholder_name(name) || !is_placeholder_name(value) {
            self.diagnostics
                .push(malformed_header(kv.value_span.clone()));
            return None;
        }
        Some(InterpolationBinding::new(name, value, value_type))
    }

    pub(super) fn valid_key_value<'a>(
        &mut self,
        field: HeaderField<'a>,
    ) -> Option<HeaderKeyValue<'a>> {
        let Some(kv) = field.key_value(self.path) else {
            self.mark(SourceRecoveryClass::Metadata);
            self.diagnostics
                .push(malformed_header(field.span(self.path)));
            return None;
        };

        if kv.key.is_empty() || kv.value.is_empty() {
            self.mark(SourceRecoveryClass::Metadata);
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
                self.mark(SourceRecoveryClass::Metadata);
                self.diagnostics
                    .push(malformed_header(kv.value_span.clone()));
                None
            }
        }
    }

    pub(super) fn metadata_entry(&mut self, kv: HeaderKeyValue<'_>) -> Option<SourceMetadataEntry> {
        match metadata_entry(kv) {
            Ok(entry) => Some(entry),
            Err(span) => {
                self.mark(SourceRecoveryClass::Metadata);
                self.diagnostics.push(malformed_header(span));
                None
            }
        }
    }

    pub(super) fn divert_target(&mut self, field: HeaderField<'_>) -> Option<DivertTarget> {
        if field.text == recite_core::END_DIVERT_TARGET {
            return Some(DivertTarget::End);
        }

        let reference = if let Some((file, block_id)) = field.text.split_once("::") {
            if file.is_empty() || block_id.is_empty() || block_id.contains("::") {
                self.mark(SourceRecoveryClass::BlockReferences);
                self.diagnostics
                    .push(malformed_divert_target(field.span(self.path)));
                return None;
            }

            let Ok(block_id) = BlockId::new(block_id) else {
                self.mark(SourceRecoveryClass::BlockReferences);
                self.diagnostics
                    .push(malformed_divert_target(field.span(self.path)));
                return None;
            };

            let block_id_span = span_for_text(
                self.path,
                field.line,
                field.column + file.chars().count() + 2,
                block_id.as_str(),
            );
            BlockReference::external(file, block_id).with_spans(
                Some(span_for_text(self.path, field.line, field.column, file)),
                block_id_span,
            )
        } else {
            let Ok(block_id) = BlockId::new(field.text) else {
                self.mark(SourceRecoveryClass::BlockReferences);
                self.diagnostics
                    .push(malformed_divert_target(field.span(self.path)));
                return None;
            };

            BlockReference::local(block_id).with_spans(
                None,
                span_for_text(self.path, field.line, field.column, field.text),
            )
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
