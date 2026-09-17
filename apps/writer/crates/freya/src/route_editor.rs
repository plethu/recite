//! Destination editing beside the reply or continuation it changes.
use crate::design::tokens as t;
use crate::{editing::Writer, palette};
use freya::prelude::*;
use recite_writer_model::View;

#[derive(Clone, PartialEq)]
pub(super) enum RouteOwner {
    Reply(String),
    Beat(String),
}

#[derive(Clone)]
pub(super) struct RouteEditor {
    pub writer: Writer,
    pub owner: RouteOwner,
    pub heading: Element,
}
impl PartialEq for RouteEditor {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner
            && self.heading == other.heading
            && self.writer.dark == other.writer.dark
    }
}
impl Component for RouteEditor {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let open = use_state(|| false);
        let query = use_state(String::new);
        let id = use_a11y();
        let input_id = use_a11y();
        let options = use_memo(move || {
            let needle = query.read().trim().to_lowercase();
            let sections = writer
                .buffers
                .model
                .read()
                .as_ref()
                .map(|model| model.document().sections())
                .unwrap_or_default();
            std::sync::Arc::new(
                sections
                    .into_iter()
                    .chain(std::iter::once("END".into()))
                    .filter_map(|value| {
                        let title = if value == "END" {
                            "End conversation".into()
                        } else {
                            palette::display_name(&value)
                        };
                        (title.to_lowercase().contains(&needle)
                            || value.to_lowercase().contains(&needle))
                        .then(|| crate::design::PickerOption {
                            detail: if value == "END" {
                                "Finish this conversation".into()
                            } else {
                                String::new()
                            },
                            title,
                            value,
                        })
                    })
                    .collect::<Vec<_>>(),
            )
        });
        let owner = self.owner.clone();
        rect()
            .width(Size::fill())
            .spacing(t::SPACE_XS)
            .child(self.heading.clone())
            .child(crate::design::SearchPicker {
                id,
                input_id,
                name: "Change destination…".into(),
                placeholder: "Find a destination…".into(),
                empty_hint: "Choose a beat or end this conversation".into(),
                no_matches: "No matching destinations".into(),
                selected: String::new(),
                query,
                open,
                options: options.read().clone(),
                enabled: true,
                vim: writer.preferences.read().config.ui.keymap == recite_config::Keymap::Vim,
                choose: EventHandler::new(move |option: crate::design::PickerOption| {
                    writer.navigate(|model| match &owner {
                        RouteOwner::Reply(id) => {
                            model.select(View::Passage(id.clone()))?;
                            model.attribute(&option.value)
                        }
                        RouteOwner::Beat(block) => {
                            model.select(View::Block(block.clone()))?;
                            model.set_continuation(&option.value)
                        }
                    });
                }),
            })
    }
}
