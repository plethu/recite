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
        let collapsed = use_state(|| None::<String>);
        let query = use_state(String::new);
        let active = use_state(|| None::<usize>);
        let id = use_a11y();
        let mut scroll = use_scroll_controller(ScrollConfig::default);
        use_after_side_effect(move || {
            if let Some(index) = *active.read() {
                scroll.scroll_to_y(-((index.saturating_sub(2) * 36) as i32));
            }
        });
        let needle = query.read().to_lowercase();
        let mut rows = Vec::new();
        for scene in &self.scenes {
            let expanded = scene.active
                && (!needle.is_empty() || collapsed.read().as_ref() != Some(&scene.caption));
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
                            selected: if writer.localisation.read().active {
                                writer
                                    .buffers
                                    .model
                                    .read()
                                    .as_ref()
                                    .ok()
                                    .and_then(|m| m.selected_block().ok().flatten())
                                    .as_ref()
                                    == Some(id)
                            } else {
                                writer.selection.read().as_ref() == Some(id)
                            },
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
        let destinations = rows.clone();
        rect()
            .width(Size::fill())
            .height(Size::flex(1.))
            .content(Content::Flex)
            .spacing(t::SPACE_XS)
            .child(crate::design::SearchField {
                query,
                id,
                placeholder: "Filter scenes and beats".into(),
                active,
                count,
                vim: writer.preferences.read().config.ui.keymap == recite_config::Keymap::Vim,
                changed: EventHandler::new(|()| {}),
                activate: EventHandler::new(move |index: usize| {
                    if let Some(row) = destinations.get(index) {
                        activate_row(writer, collapsed, row);
                    }
                }),
            })
            .child(if count == 0 {
                label()
                    .text(crate::messages::text(
                        crate::messages::MsgId::WriterGuiNoMatchingScenesOrBeats,
                    ))
                    .into_element()
            } else {
                rect().into_element()
            })
            .child(
                VirtualScrollView::new_with_data(
                    (rows, *active.read()),
                    move |index, (rows, active)| {
                        let content = match &rows[index.index] {
                            Row::Scene(scene, expanded) => {
                                let scene = scene.clone();
                                let expanded = *expanded;
                                let caption = scene.caption.clone();
                                Button::new()
                                    .flat()
                                    .selected(scene.active || *active == Some(index.index))
                                    .expanded(expanded)
                                    .width(Size::fill())
                                    .named(caption.clone())
                                    .on_press(move |_| {
                                        activate_row(
                                            writer,
                                            collapsed,
                                            &Row::Scene(scene.clone(), expanded),
                                        )
                                    })
                                    .child(
                                        rect()
                                            .horizontal()
                                            .width(Size::fill())
                                            .spacing(t::SPACE_SM)
                                            .child(label().text(if expanded {
                                                "▾"
                                            } else {
                                                "▸"
                                            }))
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
                                        *selected || *active == Some(index.index),
                                        writer.dark,
                                        move |_| {
                                            select_beat(writer, &id);
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
                    },
                )
                .scroll_controller(scroll)
                .length(count)
                .item_size(36.)
                .height(Size::flex(1.))
                .width(Size::fill()),
            )
    }
}

fn activate_row(writer: Writer, mut collapsed: State<Option<String>>, row: &Row) {
    match row {
        Row::Scene(scene, expanded) => {
            if scene.active {
                collapsed.set(if *expanded {
                    Some(scene.caption.clone())
                } else {
                    None
                });
            } else {
                collapsed.set(None);
                scene.open.call(());
            }
        }
        Row::Beat { id, .. } => {
            select_beat(writer, id);
        }
    }
}

fn select_beat(mut writer: Writer, id: &str) {
    if writer.localisation.peek().active {
        writer.localisation.write().view = crate::localisation::CatalogueView::Passage;
        writer.inspect(id);
    } else if writer.layout.standalone() {
        writer.inspect(id);
    } else {
        writer.selection.set(Some(id.to_owned()));
        writer.map_focus.request_focus();
    }
}
