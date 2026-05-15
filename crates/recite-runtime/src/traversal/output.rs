use recite_core::{
    CompiledChoice, CompiledChoiceEcho, CompiledMetadataEntry, LineIndex, MetadataRange,
    SpeakerIndex,
};

use crate::DialogueError;
use crate::event::{ChoiceEchoMode, DialogueChoice, DialogueLine};

use super::asset::AssetView;

pub(super) fn dialogue_line(
    asset: AssetView<'_>,
    line_index: LineIndex,
    default_speaker: Option<SpeakerIndex>,
) -> Result<DialogueLine, DialogueError> {
    let line = asset.line_at(line_index)?;

    Ok(DialogueLine {
        id: line.id.clone(),
        source_text: line.source_text.clone(),
        text: line.source_text.clone(),
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
) -> Result<DialogueChoice, DialogueError> {
    Ok(DialogueChoice {
        id: choice.id.clone(),
        source_text: choice.source_text.clone(),
        text: choice.source_text.clone(),
        metadata: metadata(asset, choice.metadata)?,
        is_available,
        unavailable_reason,
        echo: choice_echo(&choice.echo),
    })
}

fn choice_echo(echo: &CompiledChoiceEcho) -> ChoiceEchoMode {
    match echo {
        CompiledChoiceEcho::None => ChoiceEchoMode::None,
        CompiledChoiceEcho::SelectedText => ChoiceEchoMode::SelectedText,
        CompiledChoiceEcho::ExplicitLine(line_id) => ChoiceEchoMode::ExplicitLine(line_id.clone()),
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
