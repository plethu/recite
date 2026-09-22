//! Scene maps compose topology, personal placement, camera and contextual editing.
use crate::design::Button;
use crate::design::tokens as t;
mod arrangement;
pub(crate) mod camera;
mod card;
mod connections;
mod edge;
mod layout;
mod navigation;
mod placement;
mod routing;
mod scope;
mod topology;
use crate::{editing::Writer, palette};
use freya::prelude::*;
use layout::HEIGHT;

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
    let mut whole_scene = use_state(|| false);
    let selected_scene = scene.clone();
    let mut selected_before = use_state(move || {
        if writer.layout.cameras.peek().contains_key(&selected_scene) {
            writer.selection.peek().clone()
        } else {
            None
        }
    });
    let storage = use_try_consume::<Result<recite_config::UserStateFile, String>>();
    let mut positions = use_state(move || placement::Arrangement::new(storage));
    let initial_scene = scene.clone();
    let mut camera = use_state(move || {
        writer
            .layout
            .cameras
            .peek()
            .get(&initial_scene)
            .copied()
            .unwrap_or_default()
    });
    let mut current_scene = use_state(String::new);
    let mut cameras = writer.layout.cameras;
    use_side_effect(move || {
        cameras
            .write()
            .insert(current_scene.read().clone(), *camera.read());
    });
    let viewport = use_state(|| (900., 650.));
    let pan = use_state(|| None::<(f64, f64)>);
    let mut hovered = use_state(|| None::<String>);
    let mut show_help = use_state(|| false);
    let connections_open = use_state(|| false);
    let mut connection = use_state(|| None::<recite_writer_model::SceneLink>);
    let script = use_memo(move || {
        writer
            .buffers
            .model
            .read()
            .as_ref()
            .ok()
            .and_then(|session| session.document().script_snapshot().ok())
    });
    if *current_scene.peek() != scene {
        positions.write().ensure_scene(&scene);
        let saved = writer.layout.cameras.peek().get(&scene).copied();
        camera.set(saved.unwrap_or_default());
        if saved.is_some() {
            selected_before.set(writer.selection.peek().clone());
        }
        hovered.set(None);
        connection.set(None);
        current_scene.set(scene.clone());
    }
    let graph = use_memo(move || {
        script.read().as_ref().map(|blocks| {
            topology::Topology::new(blocks, positions.read().positions(&current_scene.read()))
        })
    });
    let scope = use_memo(move || {
        if *whole_scene.read() {
            return None;
        }
        let source = script.read();
        let graph = graph.read();
        scope::nearby(
            source.as_ref()?,
            graph.as_ref()?,
            writer.selection.read().as_deref(),
        )
    });
    let scope = scope.read();
    let projected = script.read();
    let Some(blocks) = projected.as_ref() else {
        return rect().into_element();
    };
    let graph = graph.read();
    let Some(graph) = graph.as_ref() else {
        return rect().into_element();
    };
    let (links, nodes, routes) = (&graph.links, &graph.nodes, &graph.routes);
    let document_selection = writer
        .buffers
        .model
        .read()
        .as_ref()
        .ok()
        .and_then(|session| session.selected_block().ok().flatten());
    let mut selected_state = writer.selection;
    let selected = selected_state
        .read()
        .clone()
        .filter(|id| blocks.iter().any(|block| &block.id == id))
        .or_else(|| document_selection.clone());
    if *selected_state.peek() != selected {
        selected_state.set(selected.clone());
    }
    let emphasis = hovered.read().clone().or_else(|| selected.clone());
    let bounds = scope.as_ref().map_or(graph.bounds, |scope| scope.bounds);
    let mut next = *camera.peek();
    match next.framing {
        camera::Framing::Entry => {
            if let Some(entry) = graph.entry {
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
    for (index, (link, route)) in links.iter().zip(routes).enumerate() {
        if scope.as_ref().is_some_and(|scope| {
            !scope.members.contains(&link.origin) || !scope.members.contains(&link.destination)
        }) {
            continue;
        }
        let Some(route) = route else {
            continue;
        };
        if !topology::visible(route.bounds, view, *viewport.read()) {
            continue;
        }
        let incident = connection.read().as_ref().map_or_else(
            || {
                emphasis.as_deref() == Some(&link.origin)
                    || emphasis.as_deref() == Some(&link.destination)
            },
            |active| active == link,
        );
        canvas = canvas.child(edge::Edge {
            key: format!("{scene}:{index}"),
            route: route.clone(),
            zoom: view.zoom,
            returning: link.returning,
            highlighted: incident,
            dimmed: emphasis.is_some() && !incident,
            conditional: link.conditional,
            dark: writer.dark,
            reduced_motion: writer.preferences.read().config.writer.reduced_motion,
        });
    }
    for (block, node) in blocks.iter().zip(nodes.iter()) {
        if scope
            .as_ref()
            .is_some_and(|scope| !scope.members.contains(&block.id))
        {
            continue;
        }
        if selected.as_ref() != Some(&block.id)
            && !topology::visible((node.x, node.y, node.width, HEIGHT), view, *viewport.read())
        {
            continue;
        }
        canvas = canvas.child(card::Card {
            writer,
            block: block.clone(),
            node: node.clone(),
            selected: selected.as_ref() == Some(&block.id),
            related: connection
                .read()
                .as_ref()
                .is_some_and(|link| link.origin == block.id || link.destination == block.id),
            ending: graph.endings.contains(&block.id),
            scene: scene.clone(),
            hovered,
            positions,
            camera,
            zoom: view.zoom,
            pan,
        });
    }
    for (link, route) in links.iter().zip(routes) {
        if view.zoom < card::DIALOGUE_ZOOM
            || (hovered.read().as_deref() != Some(&link.origin)
                && connection.read().as_ref() != Some(link))
        {
            continue;
        }
        if scope.as_ref().is_some_and(|scope| {
            !scope.members.contains(&link.origin) || !scope.members.contains(&link.destination)
        }) {
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
                        .background(palette::Palette::new(writer.dark).canvas)
                        .corner_radius(t::RADIUS)
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
        nodes.clone(),
    );
    let mut pane = writer.pane;
    let error = positions.read().error.clone();
    let actions = rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(t::SPACE_SM)
        .child(
            Button::new()
                .flat()
                .enabled(writer.selection.read().is_some())
                .on_press(move |_| {
                    let selected = writer.selection.peek().clone();
                    if let Some(id) = selected {
                        writer.inspect(&id);
                    }
                })
                .child(crate::messages::text(
                    crate::messages::MsgId::WriterWorkspaceOpenScript,
                )),
        )
        .child(
            Button::new()
                .flat()
                .on_press(move |_| {
                    writer.navigate(recite_writer_model::Workbench::add_beat);
                    pane.set(crate::editing::Pane::Script);
                })
                .child(crate::messages::text(
                    crate::messages::MsgId::WriterGuiAddBeat,
                )),
        )
        .child(
            Button::new()
                .flat()
                .on_press(move |_| {
                    positions.write().reset(&scene);
                    camera.write().framing = camera::Framing::Free;
                })
                .named(crate::messages::text(
                    crate::messages::MsgId::WriterGuiArrangeAutomatically,
                ))
                .child(crate::messages::text(
                    crate::messages::MsgId::WriterGuiArrange,
                )),
        )
        .child(
            Button::new()
                .flat()
                .named(crate::messages::text(
                    crate::messages::MsgId::WriterGuiMapHelp,
                ))
                .expanded(*show_help.read())
                .on_press(move |_| {
                    let next = !*show_help.peek();
                    show_help.set(next);
                })
                .child(crate::messages::text(crate::messages::MsgId::WriterGuiHelp)),
        );
    let navigation = rect()
        .horizontal()
        .cross_align(Alignment::Center)
        .spacing(t::SPACE_SM)
        .width(Size::fill())
        .content(Content::Flex)
        .child(
            rect()
                .width(Size::flex(1.))
                .max_width(Size::px(320.))
                .child(
                    Input::new(writer.search)
                        .a11y_id(writer.search_focus)
                        .width(Size::fill())
                        .height(Size::px(t::control_height()))
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
                ),
        )
        .child(rect().width(Size::flex(1.)))
        .child(crate::controls::IconButton::new(
            "Zoom out",
            crate::controls::Icon::ZoomOut,
            move || {
                let zoom = camera.peek().zoom / 1.2;
                camera.write().zoom_to(zoom, *viewport.peek());
            },
        ))
        .child(
            Button::new()
                .flat()
                .named(crate::messages::text(
                    crate::messages::MsgId::WriterGuiResetZoom,
                ))
                .on_press(move |_| {
                    camera.write().zoom_to(1., *viewport.peek());
                })
                .child(format!("{:.0}%", camera.read().zoom * 100.)),
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
                .on_press(move |_| {
                    camera.write().fit(*viewport.peek(), bounds);
                })
                .child(crate::messages::text(crate::messages::MsgId::WriterGuiFit)),
        );
    rect()
        .key("scene-map")
        .background(palette::Palette::new(writer.dark).canvas)
        .width(Size::flex(1.))
        .height(Size::fill())
        .content(Content::Flex)
        .padding(t::SPACE_MD)
        .spacing(t::SPACE_SM)
        .maybe_child((blocks.len() > 200).then(|| {
            rect()
                .horizontal()
                .spacing(t::SPACE_SM)
                .child(
                    label()
                        .text(scope.as_ref().map_or_else(
                            || format!("Whole scene · {} beats", blocks.len()),
                            |scope| {
                                format!(
                                    "Nearby · {} of {} beats · Select a connection to explore",
                                    scope.members.len(),
                                    blocks.len()
                                )
                            },
                        ))
                        .font_size(t::small()),
                )
                .child(
                    Button::new()
                        .flat()
                        .on_press(move |_| {
                            let next = !*whole_scene.peek();
                            whole_scene.set(next);
                        })
                        .child(if *whole_scene.read() {
                            "Nearby beats"
                        } else {
                            "Whole scene"
                        }),
                )
        }))
        .child(actions)
        .child(navigation)
        .maybe_child((*show_help.read()).then(|| {
            label()
                .text(crate::messages::text(
                    crate::messages::MsgId::WriterGuiMapGestureHelp,
                ))
                .font_size(t::small())
                .color(palette::muted(writer.dark))
        }))
        .maybe_child(error.map(|error| {
            label()
                .text(format!("Map placement could not be saved: {error}"))
                .font_size(t::small())
        }))
        .child(map)
        .maybe_child(
            selected
                .as_ref()
                .map(|id| connections::panel(writer, id, links, connection, connections_open)),
        )
        .into_element()
}
