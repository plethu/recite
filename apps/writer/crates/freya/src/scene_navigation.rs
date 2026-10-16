//! Scene accordion: source-order beats belong to their active scene, not an invented tree.
use crate::{
    controls,
    design::{Button, tokens as t},
    editing::Writer,
};
use freya::prelude::*;
use std::{cell::RefCell, rc::Rc, sync::Arc};

mod captions;
mod rows;
use captions::Caption;

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
#[derive(Clone, Debug, PartialEq)]
enum Row {
    Scene { scene_index: usize, expanded: bool },
    Beat { caption_index: usize },
}
// Beat indices resolve only against the caption snapshot captured with these rows.
#[derive(Clone)]
struct NavigationRows {
    selected: Option<String>,
    active: Option<usize>,
    items: Arc<[Row]>,
    captions: Arc<[Caption]>,
    scenes: Arc<[SceneBranch]>,
}
impl PartialEq for NavigationRows {
    fn eq(&self, other: &Self) -> bool {
        self.selected == other.selected
            && self.active == other.active
            && Arc::ptr_eq(&self.items, &other.items)
            && Arc::ptr_eq(&self.captions, &other.captions)
            && self.scenes == other.scenes
    }
}
impl PartialEq for SceneNavigation {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark && self.scenes == other.scenes
    }
}
impl Component for SceneNavigation {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        use_drop(move || {
            let mut matches = writer.vim.matches;
            matches.set(Vec::new());
        });
        let projected = use_memo(move || {
            writer
                .buffers
                .model
                .read()
                .as_ref()
                .ok()
                .and_then(|m| m.document().script_snapshot().ok())
        });
        let mut previous = None;
        let captions = use_memo(move || {
            let next = projected
                .read()
                .as_ref()
                .map(|blocks| captions::refresh(blocks, previous.as_ref()))
                .unwrap_or_default();
            previous = Some(next.clone());
            next
        });
        let collapsed = use_state(|| None::<String>);
        let query = writer.vim.query;
        let active = use_state(|| None::<usize>);
        let id = writer.vim.search_id;
        let scroll = use_scroll_controller(ScrollConfig::default);
        crate::design::use_list_reveal(Some(query), active, scroll, 36., 2);
        let cache = use_hook(|| Rc::new(RefCell::new(rows::Cache::default())));
        let open_block = || {
            writer
                .buffers
                .model
                .read()
                .as_ref()
                .ok()
                .and_then(|m| m.selected_block().ok().flatten())
        };
        // The rail's one plate marks the beat being edited when nothing is selected on the map.
        let selected = if writer.localisation.read().active {
            open_block()
        } else {
            writer.selection.read().clone().or_else(open_block)
        };
        let captions = captions.read().clone();
        let projection = cache.borrow_mut().refresh(
            &self.scenes,
            &captions,
            &query.read(),
            collapsed.read().as_deref(),
        );
        if let Some(next) = projection.matches {
            let mut matches = writer.vim.matches;
            matches.set_if_modified(next);
        }
        let rows = projection.items;
        let count = rows.len();
        let rows = NavigationRows {
            selected,
            active: *active.read(),
            items: rows,
            captions,
            scenes: self.scenes.clone().into(),
        };
        let destinations = rows.clone();
        rect()
            .width(Size::fill())
            .height(Size::flex(1.))
            .content(Content::Flex)
            .spacing(t::SPACE_XS)
            .child(
                rect()
                    .width(Size::fill())
                    .padding((0., t::RAIL_PADDING))
                    .child(crate::design::SearchField {
                        insert_request: Some(writer.vim.insert_requested),
                        query,
                        id,
                        placeholder: "Find scene or beat…".into(),
                        active,
                        count,
                        vim: writer.preferences.read().config.ui.keymap
                            == recite_config::Keymap::Vim,
                        changed: EventHandler::new(|()| {}),
                        activate: EventHandler::new(move |index: usize| {
                            activate_row(writer, collapsed, &destinations, index);
                        }),
                    }),
            )
            .child(if count == 0 {
                rect()
                    .padding((0., t::RAIL_PADDING))
                    .child(label().text(crate::messages::text(
                        crate::messages::MsgId::WriterGuiNoMatchingScenesOrBeats,
                    )))
                    .into_element()
            } else {
                rect().into_element()
            })
            .child(
                VirtualScrollView::new_with_data(rows, move |index, rows| {
                    let content = match &rows.items[index.index] {
                        Row::Scene {
                            scene_index,
                            expanded,
                        } => {
                            let scene = &rows.scenes[*scene_index];
                            let expanded = *expanded;
                            let caption = scene.caption.clone();
                            let destinations = rows.clone();
                            let active_scene = scene.active;
                            // One plate in the rail: the owning scene is plum text, not a plate.
                            Button::new()
                                .flat()
                                .toggle(active_scene)
                                .highlighted(rows.active == Some(index.index))
                                .plate_bleed(t::RAIL_PADDING)
                                .expanded(expanded)
                                .width(Size::fill())
                                .named(caption.clone())
                                .on_press(move |_| {
                                    activate_row(writer, collapsed, &destinations, index.index)
                                })
                                .child(
                                    rect()
                                        .horizontal()
                                        .width(Size::fill())
                                        .spacing(t::SPACE_SM)
                                        .child(label().text(if expanded { "▾" } else { "▸" }))
                                        .child(label().text(caption).maybe(
                                            active_scene,
                                            |caption| {
                                                caption
                                                    .color(t::colors().accent)
                                                    .font_weight(FontWeight::SEMI_BOLD)
                                            },
                                        )),
                                )
                                .into_element()
                        }
                        Row::Beat { caption_index } => {
                            let caption = &rows.captions[*caption_index];
                            let id = &caption.id;
                            let selected = rows.selected.as_ref() == Some(id);
                            let id = id.clone();
                            rect()
                                .width(Size::fill())
                                .padding((0., 0., 0., t::SPACE_XL))
                                .child(controls::navigation_row(
                                    caption.label.clone(),
                                    selected || rows.active == Some(index.index),
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
                        .padding((0., t::RAIL_PADDING))
                        .overflow(Overflow::Clip)
                        .child(content)
                        .into_element()
                })
                .scroll_controller(scroll)
                .length(count)
                .item_size(36.)
                .height(Size::flex(1.))
                .width(Size::fill()),
            )
    }
}

fn activate_row(
    writer: Writer,
    mut collapsed: State<Option<String>>,
    rows: &NavigationRows,
    index: usize,
) {
    match rows.items.get(index) {
        Some(Row::Scene {
            scene_index,
            expanded,
        }) => {
            let scene = &rows.scenes[*scene_index];
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
        Some(Row::Beat { caption_index }) => {
            select_beat(writer, &rows.captions[*caption_index].id);
        }
        None => {}
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
