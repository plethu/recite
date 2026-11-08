//! Reply components consume one resolved editing session.
use super::*;
use crate::WriterLayout;
use crate::presentation::{label, paragraph, text_button};
use xilem::masonry::kurbo;
use xilem::{
    AnyWidgetView, WidgetView,
    masonry::layout::AsUnit,
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row, sized_box},
};
mod argument;
mod structure;
pub(crate) use argument::ArgumentField;
const PAGE: usize = 8;

pub(crate) fn panel(state: &Writer, reply: &ReplySession) -> impl WidgetView<Writer> + use<> {
    sized_box(
        flex_col((
            heading(state, &reply.projection),
            additions(state, &reply.projection),
            draft_actions(state),
            reply_actions(state, &reply.projection),
            fields(state, reply),
            pagination(state, reply),
            (!state.work.busy()
                && !state.navigation.busy()
                && !reply
                    .projection
                    .belongs_to(state.session.model().document()))
            .then(|| {
                text_button(
                    state.ui_scale(),
                    "Refresh reply controls",
                    Writer::refresh_rules,
                )
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(12.px()),
    )
    .width(state.script_width().px())
}
fn heading(state: &Writer, rules: &ReplyRules) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let width = state.script_width();
    flex_col((
        label(scale, "Reply rules").text_size(22. * scale),
        paragraph(scale, format!("{} · Destination: {}", rules.text,
            rules.destination.as_deref().unwrap_or("No destination")), width),
        paragraph(scale, "Conditions control when this reply is available. Effects belong to its destination and can also run when another reply reaches that beat.", width),
        rules.has_other_statements.then(|| paragraph(scale,
            "Other statements are preserved. Effect order can change only when the destination contains direct effects alone.", width)),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(12.px())
}
fn additions(state: &Writer, rules: &ReplyRules) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let disabled = !state.rules_editable();
    flex_col((
        flex_row((
            text_button(scale, "Add condition…", |state: &mut Writer| {
                state.pick_rule_choice(selection::Target::AddCondition(None))
            })
            .disabled(disabled || rules.available_conditions.is_empty()),
            text_button(scale, "Add effect…", |state: &mut Writer| {
                state.pick_rule_choice(selection::Target::AddEffect)
            })
            .disabled(
                disabled || !rules.effect_order_editable || rules.available_effects.is_empty(),
            ),
        ))
        .gap(8.px()),
        rules
            .condition
            .is_none()
            .then(|| label(scale, "Available without a condition")),
        rules
            .effects
            .is_empty()
            .then(|| label(scale, "No direct effects at this destination")),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}
fn draft_actions(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let disabled = !state.rules_editable() || !state.session.model().has_draft();
    flex_row((
        text_button(state.ui_scale(), "Apply reply", |state: &mut Writer| {
            state.navigate(crate::navigation::Target::Edit(
                recite_writer_model::EditOperation::Apply,
            ))
        })
        .disabled(disabled),
        text_button(state.ui_scale(), "Discard reply", Writer::discard_rules).disabled(disabled),
    ))
    .gap(8.px())
}
fn reply_actions(state: &Writer, rules: &ReplyRules) -> impl WidgetView<Writer> + use<> {
    let passage = rules.passage.clone();
    flex_col((
        text_button(state.ui_scale(), "Edit reply text", Writer::edit_reply_text)
            .disabled(!state.rules_editable()),
        text_button(
            state.ui_scale(),
            "Change reply destination…",
            move |state: &mut Writer| {
                state.choose_destination(recite_writer_model::RouteOwner::Reply(passage.clone()))
            },
        )
        .disabled(!state.rules_editable()),
    ))
    .direction(
        if state.script_width() >= 350. * f64::from(state.ui_scale()) {
            kurbo::Axis::Horizontal
        } else {
            kurbo::Axis::Vertical
        },
    )
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
fn fields(state: &Writer, reply: &ReplySession) -> impl WidgetView<Writer> + use<> {
    let pages = reply.controls.rows.len().div_ceil(PAGE).max(1);
    let page = reply.controls.page.min(pages - 1);
    let rows = reply
        .controls
        .rows
        .iter()
        .skip(page * PAGE)
        .take(PAGE)
        .map(|row| row_view(state, reply, row))
        .collect::<Vec<_>>();
    flex_col(rows)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(16.px())
}
fn row_view(state: &Writer, reply: &ReplySession, row: &Row) -> Box<AnyWidgetView<Writer>> {
    match row.resolve(&reply.projection) {
        Some(rows::Resolved::Condition { path, title, node }) => {
            structure::condition(state, reply, path, title, node).boxed()
        }
        Some(rows::Resolved::Effect { index, effect }) => {
            structure::effect(state, reply, index, effect).boxed()
        }
        Some(rows::Resolved::Argument {
            target,
            owner,
            argument,
        }) => argument::field(state, reply, target, owner, argument).boxed(),
        None => paragraph(
            state.ui_scale(),
            "Reply controls changed. Refresh them to continue.",
            state.script_width(),
        )
        .boxed(),
    }
}
fn pagination(state: &Writer, reply: &ReplySession) -> Option<impl WidgetView<Writer> + use<>> {
    let pages = reply.controls.rows.len().div_ceil(PAGE).max(1);
    let page = reply.controls.page.min(pages - 1);
    (pages > 1).then(|| {
        flex_col((
            label(
                state.ui_scale(),
                format!("Rule fields · {} of {pages}", page + 1),
            ),
            flex_row((
                text_button(
                    state.ui_scale(),
                    "Previous fields",
                    move |state: &mut Writer| {
                        if let Some(reply) = &mut state.rules {
                            reply.controls.page = page.saturating_sub(1);
                        }
                    },
                )
                .disabled(page == 0),
                text_button(
                    state.ui_scale(),
                    "Next fields",
                    move |state: &mut Writer| {
                        if let Some(reply) = &mut state.rules {
                            reply.controls.page = (page + 1).min(pages - 1);
                        }
                    },
                )
                .disabled(page + 1 >= pages),
            ))
            .gap(8.px()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px())
    })
}
