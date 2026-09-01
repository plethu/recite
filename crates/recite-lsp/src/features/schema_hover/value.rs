use recite_core::SpeakerDefinition;
use recite_ui::{MsgId, UiCatalog};

pub(crate) fn speaker_hover_text(
    word: &str,
    definition: &SpeakerDefinition,
    catalog: &UiCatalog,
) -> String {
    definition.display_name.as_ref().map_or_else(
        || catalog.format_pairs(MsgId::LspHoverSpeaker, [("name", word)]),
        |display_name| {
            catalog.format_pairs(
                MsgId::LspHoverSpeakerWithDisplayName,
                [("name", word), ("display_name", display_name)],
            )
        },
    )
}
