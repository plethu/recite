//! Scene accordion: source-order beats belong to their active scene, not an invented tree.
use crate::{
    controls,
    design::{Button, tokens as t},
    editing::Writer,
    palette,
};
use freya::prelude::*;

#[derive(Clone)]
pub(super) struct SceneBranch {
    pub caption: String,
    pub active: bool,
    pub open: EventHandler<()>,
}

impl PartialEq for SceneBranch {
    fn eq(&self, other: &Self) -> bool {
        self.caption == other.caption && self.active == other.active && self.open == other.open
    }
}

#[derive(Clone)]
pub(super) struct SceneNavigation {
    pub writer: Writer,
    pub scenes: Vec<SceneBranch>,
}
#[derive(Clone, PartialEq)]
enum Row {
    Scene(SceneBranch, bool),
    Beat {
        id: String,
        caption: String,
        selected: bool,
    },
}
impl PartialEq for SceneNavigation {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark && self.scenes == other.scenes
    }
}
impl Component for SceneNavigation {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let projected = use_memo(move || {
            writer
                .buffers
                .model
                .read()
                .as_ref()
                .ok()
                .and_then(|m| m.document().script_snapshot().ok())
        });
        let captions = use_memo(move || {
            projected
                .read()
                .as_ref()
                .map(|blocks| {
                    let links = recite_writer_model::scene_links(blocks);
                    let endings: std::collections::BTreeSet<_> = links
                        .iter()
                        .filter(|l| l.destination == "END")
                        .map(|l| l.origin.as_str())
                        .collect();
                    blocks
                        .iter()
                        .map(|block| {
                            (
                                block.id.clone(),
                                format!(
                                    "{}{}{}",
                                    if block.is_default { "Start · " } else { "" },
                                    palette::display_name(&block.id),
                                    if endings.contains(block.id.as_str()) {
                                        " · end"
                                    } else {
                                        ""
                                    }
                                ),
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        });
        let mut collapsed = use_state(|| None::<String>);
        let query = use_state(String::new);
        let needle = query.read().to_lowercase();
        let mut rows = Vec::new();
        for scene in &self.scenes {
            let expanded = scene.active && collapsed.read().as_ref() != Some(&scene.caption);
            let mut beats = Vec::new();
            if expanded {
                for (id, caption) in captions.read().iter() {
                    if needle.is_empty()
                        || caption.to_lowercase().contains(&needle)
                        || scene.caption.to_lowercase().contains(&needle)
                    {
                        beats.push(Row::Beat {
                            id: id.clone(),
                            caption: caption.clone(),
                            selected: writer.selection.read().as_ref() == Some(id),
                        });
                    }
                }
            }
            if needle.is_empty()
                || scene.caption.to_lowercase().contains(&needle)
                || !beats.is_empty()
            {
                rows.push(Row::Scene(scene.clone(), expanded));
                rows.extend(beats);
            }
        }
        let count = rows.len();
        rect()
            .width(Size::fill())
            .height(Size::flex(1.))
            .content(Content::Flex)
            .spacing(t::SPACE_XS)
            .child(
                Input::new(query)
                    .width(Size::fill())
                    .placeholder("Filter scenes and beats")
                    .on_pre_key_down(crate::closing::text_input_key),
            )
            .child(
                VirtualScrollView::new_with_data(rows, move |index, rows| {
                    let content = match &rows[index.index] {
                        Row::Scene(scene, expanded) => {
                            let scene = scene.clone();
                            let expanded = *expanded;
                            let caption = scene.caption.clone();
                            Button::new()
                                .flat()
                                .selected(scene.active)
                                .expanded(expanded)
                                .width(Size::fill())
                                .named(caption.clone())
                                .on_press(move |_| {
                                    if scene.active {
                                        collapsed.set(if expanded {
                                            Some(scene.caption.clone())
                                        } else {
                                            None
                                        });
                                    } else {
                                        collapsed.set(None);
                                        scene.open.call(());
                                    }
                                })
                                .child(
                                    rect()
                                        .horizontal()
                                        .width(Size::fill())
                                        .spacing(t::SPACE_SM)
                                        .child(label().text(if expanded { "▾" } else { "▸" }))
                                        .child(label().text(caption)),
                                )
                                .into_element()
                        }
                        Row::Beat {
                            id,
                            caption,
                            selected,
                        } => {
                            let id = id.clone();
                            rect()
                                .width(Size::fill())
                                .padding((0., 0., 0., t::SPACE_XL))
                                .child(controls::navigation_row(
                                    caption.clone(),
                                    *selected,
                                    writer.dark,
                                    move |_| {
                                        let mut writer = writer;
                                        writer.selection.set(Some(id.clone()));
                                        writer.map_focus.request_focus();
                                    },
                                ))
                                .into_element()
                        }
                    };
                    rect()
                        .key(index.index)
                        .height(Size::px(36.))
                        .width(Size::fill())
                        .overflow(Overflow::Clip)
                        .child(content)
                        .into_element()
                })
                .length(count)
                .item_size(36.)
                .height(Size::flex(1.))
                .width(Size::fill()),
            )
    }
}
