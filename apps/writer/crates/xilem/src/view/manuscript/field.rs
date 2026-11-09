//! Shared native field construction for accepted inline passages and the fallback editor.
use super::*;
use crate::attention::{EditingTarget, PassageField};
use crate::presentation::{
    control_style::{ControlStyle, styled},
    print,
};
use crate::reading::PassageNumber;
use recite_writer_model::Passage;
use recite_writer_session::typography::FONT_PROSE;
use xilem::masonry::properties;
pub(in crate::view) fn prose_editor(
    state: &Writer,
    passage: &str,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    field(
        state,
        passage,
        state.session.model().draft().to_owned(),
        "Manuscript".into(),
        width,
    )
}
pub(in crate::view) fn passage_editor(
    state: &Writer,
    passage: &Passage,
    number: PassageNumber,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let active = state.session.model().view() == &View::Passage(passage.id.clone());
    field(
        state,
        &passage.id,
        if active {
            state.session.model().draft().to_owned()
        } else {
            passage.text.clone()
        },
        if active {
            "Manuscript".into()
        } else {
            format!("Edit passage {}", number.ordinal())
        },
        width,
    )
}
fn field(
    state: &Writer,
    passage: &str,
    text: String,
    name: String,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let target = EditingTarget::passage(state, passage.to_owned());
    keyed(
        (
            state.session.model().document().key().clone(),
            passage.to_owned(),
        ),
        input::guard(
            target.clone(),
            text.clone(),
            styled(
                ControlStyle::Manuscript,
                text_input(text, |state: &mut Writer, text| {
                    state.change(|model| {
                        model.set_draft(text);
                        Ok(())
                    });
                })
                .insert_newline(InsertNewline::OnEnter)
                .font(print::font(FONT_PROSE))
                .text_size(state.reading_size())
                .placeholder(name.clone())
                .disabled(state.phase() != Phase::Writing || state.session.read_only())
                .prop(properties::TextAreaLabel::new(name))
                .prop(properties::PlaceholderTextSize {
                    size: state.reading_size(),
                })
                .prop(Manuscript(true))
                .prop(PassageField(Some(target)))
                .padding(0.px())
                .background_color(state.palette.surface),
            ),
        ),
    )
    .width(width.px())
}
