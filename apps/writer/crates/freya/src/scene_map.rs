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
mod toolbar;
mod topology;
use crate::{editing::Writer, palette};
use freya::prelude::*;
use layout::HEIGHT;
use std::sync::Arc;

// Scope indices and source blocks resolve against the same captured topology.
#[derive(Clone)]
struct MapView {
    graph: Arc<topology::Topology>,
    scope: Option<scope::Scope>,
}
impl PartialEq for MapView {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.graph, &other.graph) && self.scope == other.scope
    }
}

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
    let show_help = use_state(|| false);
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
            Arc::new(topology::Topology::new(
                blocks.clone(),
                positions.read().positions(&current_scene.read()),
            ))
        })
    });
    let map_view = use_memo(move || {
        let graph = graph.read();
        let graph = graph.as_ref()?;
        let scope = if *whole_scene.read() {
            None
        } else {
            scope::nearby(graph, writer.selection.read().as_deref())
        };
        Some(MapView {
            graph: graph.clone(),
            scope,
        })
    });
    let map_view = map_view.read();
    let Some(map_view) = map_view.as_ref() else {
        return rect().into_element();
    };
    let graph = &map_view.graph;
    let blocks = &graph.blocks;
    let scope = &map_view.scope;
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
    for offset in 0..scope.as_ref().map_or(links.len(), |s| s.edge_indices.len()) {
        let index = scope.as_ref().map_or(offset, |s| s.edge_indices[offset]);
        let (link, route) = (&links[index], &routes[index]);
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
            highlighted: incident && (connection.read().is_some() || !link.returning),
            dimmed: connection.read().is_some() && !incident,
            conditional: link.conditional,
            dark: writer.dark,
            reduced_motion: writer.preferences.read().config.writer.reduced_motion,
        });
    }
    for offset in 0..scope
        .as_ref()
        .map_or(blocks.len(), |s| s.node_indices.len())
    {
        let index = scope.as_ref().map_or(offset, |s| s.node_indices[offset]);
        let (block, node) = (&blocks[index], &nodes[index]);
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
    for offset in 0..scope.as_ref().map_or(links.len(), |s| s.edge_indices.len()) {
        let index = scope.as_ref().map_or(offset, |s| s.edge_indices[offset]);
        let (link, route) = (&links[index], &routes[index]);
        if view.zoom < card::DIALOGUE_ZOOM
            || (hovered.read().as_deref() != Some(&link.origin)
                && hovered.read().as_deref() != Some(&link.destination)
                && connection.read().as_ref() != Some(link))
        {
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
                        .background(palette::current().canvas)
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
    let error = positions.read().error.clone();
    rect()
        .key("scene-map")
        .background(palette::current().canvas)
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
        .child(
            toolbar::Toolbar {
                writer,
                scene,
                camera,
                viewport,
                positions,
                bounds,
                show_help,
            }
            .render(),
        )
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
