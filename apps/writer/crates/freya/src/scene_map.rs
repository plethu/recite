//! Scene maps compose topology, personal placement, camera and contextual editing.
mod arrangement;
mod camera;
mod card;
mod edge;
mod layout;
mod navigation;
mod placement;
mod routing;
use crate::{editing::Writer, palette};
use freya::prelude::*;
use layout::{HEIGHT, WIDTH};
use recite_writer_model::scene_links;

#[derive(Clone)]
pub(super) struct SceneMap {
    pub writer: Writer,
    pub scene: String,
}
impl PartialEq for SceneMap {
    fn eq(&self, other: &Self) -> bool {
        self.scene == other.scene && self.writer.dark == other.writer.dark
    }
}
impl Component for SceneMap {
    fn render(&self) -> impl IntoElement {
        render(self.writer, self.scene.clone())
    }
}

fn render(writer: Writer, scene: String) -> Element {
    let mut selected_before = use_state(|| None::<String>);
    let storage = use_try_consume::<Result<recite_config::UserStateFile, String>>();
    let mut positions = use_state(move || placement::Arrangement::new(storage));
    let mut camera = use_state(camera::Camera::default);
    let viewport = use_state(|| (900., 650.));
    let pan = use_state(|| None::<(f64, f64)>);
    let mut current_scene = use_state(String::new);
    let mut hovered = use_state(|| None::<String>);
    let script = use_memo(move || {
        writer
            .buffers
            .model
            .read()
            .as_ref()
            .ok()
            .and_then(|session| {
                Some((
                    session.document().script().ok()?,
                    session.selected_block().ok().flatten(),
                ))
            })
    });
    if *current_scene.peek() != scene {
        positions.write().ensure_scene(&scene);
        camera.set(camera::Camera::default());
        hovered.set(None);
        current_scene.set(scene.clone());
    }
    let projected = script.read();
    let Some((blocks, document_selection)) = projected.as_ref() else {
        return rect().into_element();
    };
    let links = scene_links(blocks);
    let mut nodes = layout::layout(blocks, &links, WIDTH);
    let entry = blocks.iter().position(|b| b.is_default).unwrap_or(0);
    let entry = nodes.get(entry).map(|n| (n.x, n.y, n.width));
    // Loading is done once on scene change. Frame reads never touch disk.
    let saved = positions.read().positions(&scene).clone();
    for (block, node) in blocks.iter().zip(&mut nodes) {
        if let Some((x, y)) = saved.get(&placement::card_key(block)) {
            node.x = *x;
            node.y = *y;
        }
    }
    let mut selected_state = writer.selection;
    let selected = selected_state
        .read()
        .clone()
        .or_else(|| document_selection.clone());
    if selected_state.peek().is_none() {
        selected_state.set(selected.clone());
    }
    let emphasis = hovered.read().clone().or_else(|| selected.clone());
    let routes = routing::routes(blocks, &nodes, &links);
    let left = nodes.iter().map(|n| n.x).fold(WIDTH, f32::min).min(
        routes
            .iter()
            .flatten()
            .map(|r| r.bounds.0)
            .fold(WIDTH, f32::min),
    );
    let top = routes
        .iter()
        .flatten()
        .map(|r| r.bounds.1)
        .fold(0., f32::min);
    let right = routes
        .iter()
        .flatten()
        .map(|r| r.bounds.0 + r.bounds.2)
        .chain(nodes.iter().map(|n| n.x + n.width))
        .fold(1., f32::max);
    let bottom = routes
        .iter()
        .flatten()
        .map(|r| r.bounds.1 + r.bounds.3)
        .chain(nodes.iter().map(|n| n.y + HEIGHT))
        .fold(1., f32::max);
    let bounds = (left, top, right - left, bottom - top);
    let mut next = *camera.peek();
    match next.framing {
        camera::Framing::Entry => {
            if let Some(entry) = entry {
                next.frame_entry(*viewport.read(), entry);
            }
        }
        camera::Framing::Fit => next.fit(*viewport.read(), bounds),
        camera::Framing::Free => {}
    }
    if *selected_before.peek() != selected {
        if let Some(node) = selected
            .as_ref()
            .and_then(|id| blocks.iter().position(|b| &b.id == id))
            .and_then(|i| nodes.get(i))
        {
            navigation::reveal(&mut next, node, *viewport.read());
        }
        selected_before.set(selected.clone());
    }
    camera.set_if_modified(next);
    let view = *camera.read();
    let mut canvas = rect()
        .width(Size::px(viewport.read().0))
        .height(Size::px(viewport.read().1));
    for (index, (link, route)) in links.iter().zip(&routes).enumerate() {
        let Some(route) = route else {
            continue;
        };
        let incident = emphasis.as_deref() == Some(&link.origin)
            || emphasis.as_deref() == Some(&link.destination);
        canvas = canvas.child(edge::Edge {
            key: format!("{scene}:{index}"),
            route: route.clone(),
            zoom: view.zoom,
            returning: link.returning,
            highlighted: incident,
            dimmed: hovered.read().is_some() && !incident,
            conditional: link.conditional,
            dark: writer.dark,
            reduced_motion: writer.preferences.read().config.writer.reduced_motion,
        });
    }
    for (block, node) in blocks.iter().zip(&nodes) {
        canvas = canvas.child(card::Card {
            writer,
            block: block.clone(),
            node: node.clone(),
            selected: selected.as_ref() == Some(&block.id),
            ending: links
                .iter()
                .any(|l| l.origin == block.id && l.destination == "END"),
            scene: scene.clone(),
            hovered,
            positions,
            camera,
            zoom: view.zoom,
            pan,
        });
    }
    for (link, route) in links.iter().zip(&routes) {
        if view.zoom < card::DIALOGUE_ZOOM || emphasis.as_deref() != Some(&link.origin) {
            continue;
        }
        let Some(route) = route else {
            continue;
        };
        canvas = canvas.child(
            rect()
                .position(
                    Position::new_absolute()
                        .left((route.label.0 - 80.) * view.zoom)
                        .top(route.label.1 * view.zoom - (72. * view.zoom).max(48.) / 2.),
                )
                // Above both nested connection drawing layers.
                .layer(4)
                .width(Size::px(160. * view.zoom))
                .height(Size::px((72. * view.zoom).max(48.)))
                .main_align(Alignment::Center)
                .child(
                    rect()
                        .width(Size::fill())
                        .padding((3. * view.zoom, 6. * view.zoom))
                        .background(palette::reading(writer.dark))
                        .child(
                            label()
                                .text(link.label.clone())
                                .font_size((14. * view.zoom).max(12.))
                                .text_align(TextAlign::Center)
                                .max_lines(4),
                        ),
                ),
        );
    }
    let map = camera::viewport(
        camera,
        viewport,
        pan,
        canvas.into_element(),
        writer,
        blocks.clone(),
        nodes,
    );
    let mut pane = writer.pane;
    let error = positions.read().error.clone();
    rect()
        .key("scene-map")
        .width(Size::flex(1.))
        .height(Size::fill())
        .content(Content::Flex)
        .padding(12.)
        .spacing(8.)
        .child(
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(6.)
                .child(
                    Button::new()
                        .flat()
                        .compact()
                        .on_press(move |_| {
                            writer.navigate(recite_writer_model::Workbench::add_beat);
                            pane.set(crate::editing::Pane::Script);
                        })
                        .child("Add beat"),
                )
                .child(
                    Button::new()
                        .flat()
                        .compact()
                        .on_press(move |_| {
                            positions.write().reset(&scene);
                            camera.set(camera::Camera::default());
                        })
                        .child("Arrange automatically"),
                ),
        )
        .child(
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(6.)
                .child(crate::controls::IconButton::new(
                    "Zoom out",
                    crate::controls::Icon::ZoomOut,
                    move || {
                        let zoom = camera.peek().zoom / 1.2;
                        camera.write().zoom_to(zoom, *viewport.peek());
                    },
                ))
                .child(
                    label()
                        .text(format!("{:.0}%", camera.read().zoom * 100.))
                        .font_size(12.),
                )
                .child(crate::controls::IconButton::new(
                    "Zoom in",
                    crate::controls::Icon::ZoomIn,
                    move || {
                        let zoom = camera.peek().zoom * 1.2;
                        camera.write().zoom_to(zoom, *viewport.peek());
                    },
                ))
                .child(
                    Button::new()
                        .flat()
                        .compact()
                        .on_press(move |_| {
                            camera.write().zoom_to(1., *viewport.peek());
                        })
                        .child("100%"),
                )
                .child(
                    Button::new()
                        .flat()
                        .compact()
                        .on_press(move |_| {
                            camera.write().fit(*viewport.peek(), bounds);
                        })
                        .child("Fit"),
                ),
        )
        .child(
            label()
                .text("Dashed: return · Dotted: condition · Drag empty space to pan")
                .font_size(11.)
                .color(palette::muted(writer.dark)),
        )
        .maybe_child(error.map(|error| {
            label()
                .text(format!("Map placement could not be saved: {error}"))
                .font_size(12.)
        }))
        .child(
            Input::new(writer.search)
                .a11y_id(writer.search_focus)
                .placeholder("Find a beat…")
                .on_pre_key_down(move |event: Event<KeyboardEventData>| {
                    if event.code == Code::Escape {
                        writer.map_focus.request_focus();
                        event.stop_propagation();
                        return false;
                    }
                    if event.code == Code::Enter {
                        let query = writer.search.peek().to_lowercase();
                        if let Ok(m) = writer.buffers.model.peek().as_ref()
                            && let Ok(blocks) = m.document().script()
                            && let Some(block) = blocks.iter().find(|b| {
                                palette::display_name(&b.id).to_lowercase().contains(&query)
                            })
                        {
                            selected_state.set(Some(block.id.clone()));
                            writer.map_focus.request_focus();
                        }
                        event.stop_propagation();
                        return false;
                    }
                    crate::closing::text_input_key(event)
                }),
        )
        .child(map)
        .maybe_child(
            selected
                .as_ref()
                .map(|id| navigation::connections(writer, id, &links)),
        )
        .into_element()
}
