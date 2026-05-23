use recite_core::{
    CompiledArgument, CompiledChoice, CompiledChoiceEcho, CompiledEffect, CompiledEffectMode,
    CompiledMetadataEntry, LineIndex, LocaleId, MetadataRange, ScalarValue, SpeakerIndex,
};

use crate::DialogueError;
use crate::event::{
    ChoiceEchoMode, DialogueChoice, DialogueEffectArgument, DialogueEffectMode,
    DialogueEffectRequest, DialogueLine,
};
use crate::locale::{LocaleProvider, TextDomain};

use super::asset::AssetView;

#[derive(Clone, Copy)]
pub(super) struct LocaleLookup<'a> {
    pub(super) locale: Option<&'a LocaleId>,
    pub(super) variant: Option<&'a str>,
    pub(super) provider: Option<&'a dyn LocaleProvider>,
}

impl<'a> LocaleLookup<'a> {
    pub(super) fn source() -> Self {
        Self {
            locale: None,
            variant: None,
            provider: None,
        }
    }
}

pub(super) fn dialogue_line(
    asset: AssetView<'_>,
    line_index: LineIndex,
    default_speaker: Option<SpeakerIndex>,
    locale: LocaleLookup<'_>,
) -> Result<DialogueLine, DialogueError> {
    let line = asset.line_at(line_index)?;
    let text = localise_text(
        line.id.as_str(),
        &line.source_text,
        TextDomain::Line,
        locale,
    );

    Ok(DialogueLine {
        id: line.id.clone(),
        source_text: line.source_text.clone(),
        text,
        speaker: line
            .speaker
            .or(default_speaker)
            .map(|speaker| asset.speaker_at(speaker).map(|speaker| speaker.id.clone()))
            .transpose()?,
        metadata: metadata(asset, line.metadata)?,
    })
}

pub(super) fn dialogue_choice(
    asset: AssetView<'_>,
    choice: &CompiledChoice,
    is_available: bool,
    unavailable_reason: Option<String>,
    locale: LocaleLookup<'_>,
) -> Result<DialogueChoice, DialogueError> {
    let text = localise_text(
        choice.id.as_str(),
        &choice.source_text,
        TextDomain::Choice,
        locale,
    );

    Ok(DialogueChoice {
        id: choice.id.clone(),
        source_text: choice.source_text.clone(),
        text,
        metadata: metadata(asset, choice.metadata)?,
        is_available,
        unavailable_reason,
        echo: choice_echo(&choice.echo),
    })
}

fn localise_text(
    id: &str,
    source_text: &str,
    domain: TextDomain,
    locale: LocaleLookup<'_>,
) -> String {
    let Some((locale_id, provider)) = locale.locale.zip(locale.provider) else {
        return source_text.to_owned();
    };

    provider
        .lookup(id, source_text, domain, locale_id, locale.variant)
        .unwrap_or_else(|| source_text.to_owned())
}

pub(crate) fn dialogue_effect_request(
    asset: AssetView<'_>,
    effect: &CompiledEffect,
) -> Result<DialogueEffectRequest, DialogueError> {
    Ok(DialogueEffectRequest {
        id: effect.id.clone(),
        mode: effect_mode(effect.mode),
        function: effect.function.clone(),
        args: effect.args.iter().map(effect_argument).collect(),
        source_span: asset.source_map_at(effect.source_map)?.span.clone(),
    })
}

pub(super) fn effect_mode(mode: CompiledEffectMode) -> DialogueEffectMode {
    match mode {
        CompiledEffectMode::Deferred => DialogueEffectMode::Deferred,
        CompiledEffectMode::Immediate => DialogueEffectMode::Immediate,
        CompiledEffectMode::Blocking => DialogueEffectMode::Blocking,
    }
}

fn choice_echo(echo: &CompiledChoiceEcho) -> ChoiceEchoMode {
    match echo {
        CompiledChoiceEcho::None => ChoiceEchoMode::None,
        CompiledChoiceEcho::SelectedText => ChoiceEchoMode::SelectedText,
        CompiledChoiceEcho::ExplicitLine(line_id) => ChoiceEchoMode::ExplicitLine(line_id.clone()),
    }
}

fn effect_argument(argument: &CompiledArgument) -> DialogueEffectArgument {
    match argument {
        CompiledArgument::Identifier(value) => DialogueEffectArgument::Identifier(value.clone()),
        CompiledArgument::Value(ScalarValue::String(value)) => {
            DialogueEffectArgument::String(value.clone())
        }
        CompiledArgument::Value(ScalarValue::Integer(value)) => {
            DialogueEffectArgument::Integer(*value)
        }
        CompiledArgument::Value(ScalarValue::Float(value)) => DialogueEffectArgument::Float(*value),
        CompiledArgument::Value(ScalarValue::Boolean(value)) => {
            DialogueEffectArgument::Boolean(*value)
        }
    }
}

fn metadata(
    asset: AssetView<'_>,
    range: MetadataRange,
) -> Result<Vec<recite_core::MetadataEntry>, DialogueError> {
    asset
        .metadata_entries(range)?
        .iter()
        .map(|entry| metadata_entry(asset, entry))
        .collect()
}

fn metadata_entry(
    asset: AssetView<'_>,
    entry: &CompiledMetadataEntry,
) -> Result<recite_core::MetadataEntry, DialogueError> {
    Ok(recite_core::MetadataEntry {
        key: entry.key.clone(),
        value: entry.value.clone(),
        source_span: entry
            .source_map
            .map(|source_map| {
                asset
                    .source_map_at(source_map)
                    .map(|entry| entry.span.clone())
            })
            .transpose()?,
        key_span: None,
        value_span: None,
    })
}
