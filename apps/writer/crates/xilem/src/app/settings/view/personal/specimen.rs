use super::*;
use crate::{
    presentation::{manuscript_text, print},
    reading::Content,
};
use recite_config::WriterPresentationField;
use xilem::masonry::properties::LineBreaking;

pub(super) fn specimen(app: &App, width: f64) -> impl WidgetView<App> + use<> {
    let config = &app.preferences.config.writer;
    let scale = crate::presentation::scale(config);
    let passage = app.writer.as_ref().and_then(|writer| {
        writer.reading.selected_passage().or_else(|| {
            writer
                .reading
                .rows
                .iter()
                .take(crate::reading::PAGE)
                .find_map(|row| match &row.content {
                    Content::Passage { passage, .. } => Some(passage),
                    _ => None,
                })
        })
    });
    let text = passage.map_or(
        "The last tram waits. There is still time to choose another way.",
        |passage| &passage.text,
    );
    let mut chars = text.chars();
    let mut excerpt = chars.by_ref().take(160).collect::<String>();
    if chars.next().is_some() {
        excerpt.push('…');
    }
    let size = f32::from(
        config
            .presentation
            .value(WriterPresentationField::ReadingSize),
    );
    sized_box(
        flex_col((
            print::caps(scale, "Writing preview").color(app.theme.palette.accent),
            passage.and(app.writer.as_ref()).map(|writer| {
                label(
                    scale,
                    format!("{} · read only", writer.session.model().document().key()),
                )
                .prop(LineBreaking::WordWrap)
                .width((width - 32.).max(200.).px())
                .color(app.theme.palette.muted)
            }),
            manuscript_text(size * scale, excerpt).width((width - 32.).max(200.).px()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(12.px()),
    )
    .padding(16.px())
    .width(width.px())
    .background_color(app.theme.palette.inset)
}
