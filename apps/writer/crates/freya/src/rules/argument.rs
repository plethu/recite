use crate::design::tokens as t;
use crate::messages::{MsgId, text};
use freya::prelude::*;
use recite_writer_model::RuleArgument;
#[derive(Clone, PartialEq)]
pub(super) struct ArgumentField {
    pub argument: RuleArgument,
    pub owner: String,
    pub change: EventHandler<String>,
    pub vim: bool,
}
impl Component for ArgumentField {
    fn render(&self) -> impl IntoElement {
        let colors = t::colors();
        let error = self
            .argument
            .validate()
            .err()
            .map(|error| format!("{}: {error}", crate::palette::display_name(&self.owner)));
        let initial = self.argument.value.clone();
        let mut value = use_state(move || initial);
        let current = self.argument.value.clone();
        use_after_side_effect(move || value.set_if_modified(current.clone()));
        let change = self.change.clone();
        let caption = format!(
            "{} · {}",
            crate::palette::display_name(&self.argument.label),
            self.argument.type_name
        );
        let query = use_state(String::new);
        let open = use_state(|| false);
        let picker_id = use_a11y();
        let input_id = use_a11y();
        let field = if self.argument.choices.is_empty() {
            Input::new(value)
                .width(Size::fill())
                .a11y_id(input_id)
                .placeholder(caption.clone())
                .on_pre_key_down(crate::closing::text_input_key)
                .on_validate(move |input: InputValidator| change.call(input.text().clone()))
                .into_element()
        } else {
            crate::design::SearchPicker {
                id: picker_id,
                input_id,
                name: caption.clone(),
                placeholder: text(MsgId::WriterRulesSearchValues),
                empty_hint: text(MsgId::WriterRulesChooseValue),
                no_matches: text(MsgId::WriterRulesNoValues),
                selected: self.argument.value.clone(),
                query,
                open,
                options: std::sync::Arc::new(
                    self.argument
                        .choices
                        .iter()
                        .map(|value| crate::design::PickerOption {
                            annotation: String::new(),
                            value: value.clone(),
                            title: value.clone(),
                            detail: String::new(),
                        })
                        .collect(),
                ),
                choose: EventHandler::new(move |option: crate::design::PickerOption| {
                    change.call(option.value)
                }),
                vim: self.vim,
                enabled: true,
            }
            .into_element()
        };
        rect()
            .width(Size::fill())
            .max_width(Size::px(420.))
            .spacing(t::SPACE_XS)
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::Flex)
                    .child(
                        label()
                            .text(crate::palette::display_name(&self.argument.label))
                            .width(Size::flex(1.))
                            .font_size(t::body()),
                    )
                    .child(
                        label()
                            .text(self.argument.type_name.clone())
                            .font_size(t::small())
                            .color(colors.muted),
                    ),
            )
            .child(field)
            .maybe_child(error.map(|message| {
                label()
                    .text(message)
                    .font_size(t::small())
                    .color(colors.error)
            }))
    }
}
