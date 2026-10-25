use super::*;
use crate::attention::{EntryContext, PaneEntry};
use crate::trial::VALUE_PAGE;
use recite_core::ast::{InterpolationBinding, InterpolationType};
use recite_writer_session::preview::parse_value;

pub(super) fn values(state: &Writer, disabled: bool) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let settings = &state.trial.settings;
    let bindings = state.session.model().document().preview_bindings();
    let pages = bindings.len().div_ceil(VALUE_PAGE).max(1);
    let page = settings.binding_page.min(pages - 1);
    let fields = bindings
        .iter()
        .skip(page * VALUE_PAGE)
        .take(VALUE_PAGE)
        .map(|binding| {
            keyed(
                (binding.value.clone(), binding.value_type),
                flex_col((field(state, binding, disabled),))
                    .prop(PaneEntry(
                        state.configuring_trial() && settings.review.targets(binding),
                    ))
                    .prop(EntryContext(true)),
            )
        })
        .collect::<Vec<_>>();
    flex_col((
        (!bindings.is_empty()).then(|| label(scale, "Interpolation values").text_size(20. * scale)),
        flex_col(fields)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(10.px()),
        (pages > 1).then(|| {
            flex_col((
                label(scale, format!("Value page {} of {pages}", page + 1)),
                text_button(
                    scale,
                    "Previous preview values",
                    move |state: &mut Writer| {
                        state.trial.settings.binding_page = page.saturating_sub(1)
                    },
                )
                .disabled(disabled || page == 0),
                text_button(scale, "More preview values", move |state: &mut Writer| {
                    state.trial.settings.binding_page = page + 1
                })
                .disabled(disabled || page + 1 >= pages),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(6.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(10.px())
}

fn field(
    state: &Writer,
    binding: &InterpolationBinding,
    disabled: bool,
) -> impl WidgetView<Writer> + use<> {
    let name = binding.value.trim_start_matches('$').to_owned();
    let value = state
        .trial
        .settings
        .values
        .get(&name)
        .cloned()
        .unwrap_or_default();
    if binding.value_type == InterpolationType::Boolean {
        Either::A(
            checkbox(
                format!("Preview value: {name}"),
                value == "true",
                move |state: &mut Writer, value| {
                    state
                        .trial
                        .settings
                        .values
                        .insert(name.clone(), value.to_string());
                },
            )
            .text_size(14. * state.ui_scale())
            .disabled(disabled),
        )
    } else {
        let kind = match binding.value_type {
            InterpolationType::String => "text",
            InterpolationType::Integer => "whole number",
            InterpolationType::Float => "finite number",
            InterpolationType::Boolean => "true or false",
        };
        let invalid = (!value.is_empty() || state.trial.settings.review.targets(binding))
            && binding.value_type != InterpolationType::String
            && parse_value(binding.value_type, &value).is_err();
        let caption = format!("Preview value: {name}");
        let input = text_input(value, move |state: &mut Writer, value| {
            state.trial.settings.values.insert(name.clone(), value);
        })
        .text_size(16. * state.ui_scale())
        .disabled(disabled)
        .prop(properties::TextAreaLabel::new(caption));
        Either::B(
            flex_col((
                label(state.ui_scale(), format!("{} · {kind}", binding.value)),
                sized_box(input).width(state.script_width().px()),
                invalid.then(|| label(state.ui_scale(), format!("Enter a {kind}."))),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(6.px()),
        )
    }
}
