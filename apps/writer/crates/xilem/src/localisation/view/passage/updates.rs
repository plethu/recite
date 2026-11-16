//! Source updates as a banner at the top of the bilingual sheet: passages whose source wording
//! has no catalogue entry yet, and the review that brings the catalogue up to date.
use super::*;
use crate::presentation::Type;
use crate::presentation::banner::banner;
use xilem::masonry::{parley::style::FontWeight, properties::LineBreaking};

/// The banner, while the open catalogue lacks an entry for some passage on this page.
pub(super) fn updates(
    writer: &Writer,
    beat: &beat::Beat,
    width: f64,
) -> Option<impl WidgetView<Writer> + use<>> {
    let unmatched = unmatched(writer, beat);
    (writer.localisation.has_catalogue() && unmatched > 0).then(|| {
        let scale = writer.ui_scale();
        let state = &writer.localisation;
        banner(
            &writer.palette,
            (
                Type::Control
                    .label(
                        scale,
                        format!(
                            "Source changed · {unmatched} {} without a catalogue entry",
                            if unmatched == 1 {
                                "passage"
                            } else {
                                "passages"
                            }
                        ),
                    )
                    .weight(FontWeight::SEMI_BOLD)
                    .prop(LineBreaking::WordWrap),
                text_button(
                    scale,
                    "Review source updates",
                    Writer::review_catalogue_sources,
                )
                .disabled(
                    writer.project_action_blocked() || state.slot.read_only() || state.dirty(),
                ),
            ),
            width < 640. * f64::from(scale),
            width,
        )
    })
}

/// Passages on this page whose source wording no open catalogue entry matches.
pub(super) fn unmatched(writer: &Writer, beat: &beat::Beat) -> usize {
    beat.rows
        .iter()
        .filter_map(beat::Row::passage)
        .filter(|row| {
            row.translation.as_ref().is_none_or(|translation| {
                writer.localisation.slot.identity() != Some(&translation.key.catalogue)
            })
        })
        .count()
}
