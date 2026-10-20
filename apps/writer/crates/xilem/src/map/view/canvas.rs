//! Viewport composition and route painting share one coordinate transform.
use super::*;
use crate::view::workspace;
use xilem::masonry::imaging::record;
use xilem::masonry::kurbo::{self, CubicBez, Line, Point, Vec2};
use xilem::masonry::properties;
pub(super) fn viewport(state: &Writer, scene: &Scene) -> impl WidgetView<Writer> + use<> {
    let scale = f64::from(state.ui_scale());
    let camera = scene.camera;
    let zoom = f64::from(camera.zoom) * scale;
    let cards = scene
        .mounted_indices()
        .map(|index| {
            let node = scene.node(index);
            let transform = Affine::scale(zoom).then_translate(
                (
                    f64::from(node.x * camera.zoom + camera.x) * scale,
                    f64::from(node.y * camera.zoom + camera.y) * scale,
                )
                    .into(),
            );
            placed::placed(
                transform,
                crate::keyed::keyed(
                    (
                        state.session.model().document().key().clone(),
                        scene.projection.graph.blocks[index].id.clone(),
                    ),
                    card::card(state, scene, index),
                ),
            )
        })
        .collect::<Vec<_>>();
    let width = workspace::Layout::resolve(state).map;
    let drawing = sized_box(
        canvas(|state: &mut Writer, _, recording, _| {
            if let Some(scene) = state.map.scene(state) {
                draw_edges(state, scene, recording);
                if scene.detail() == Detail::Overview {
                    super::overview::draw(state, scene, recording);
                }
            }
        })
        .alt_text("Dialogue routes; use Connections below for their accessible list."),
    )
    .dims((width.px(), 500.px()));
    let host = input::input(
        input::Kind::Viewport(scene.detail()),
        state.config.ui.keymap == recite_config::Keymap::Vim,
        zstack((drawing, cards, route_labels(state, scene))).alignment(UnitPoint::TOP_LEFT),
        |state: &mut Writer, action| state.map_input(Source::Viewport, action),
    );
    let host = host
        .focus(focus_request(state, scene))
        .disabled(state.phase != crate::Phase::Writing)
        .prop(properties::BorderColor {
            color: state.palette.accent,
        });
    resize_observer(
        Writer::resize_map,
        sized_box(crate::keyed::keyed(
            state.session.model().document().key().clone(),
            host,
        ))
        .dims((width.px(), 500.px())),
    )
}
pub(super) fn focus_request(state: &Writer, scene: &Scene) -> Option<input::FocusRequest> {
    let focus = scene.focus.as_ref()?;
    let index = scene.projection.graph.ids.get(&focus.beat)?;
    let node = scene.node(*index);
    let scale = f64::from(state.ui_scale());
    let zoom = f64::from(scene.camera.zoom) * scale;
    let origin = Point::new(
        f64::from(node.x * scene.camera.zoom + scene.camera.x) * scale,
        f64::from(node.y * scene.camera.zoom + scene.camera.y) * scale,
    );
    Some(input::FocusRequest {
        token: focus.token.clone(),
        bounds: kurbo::Rect::from_origin_size(
            origin,
            (f64::from(node.width) * zoom, f64::from(HEIGHT) * zoom),
        ),
    })
}

fn draw_edges(state: &Writer, scene: &Scene, recording: &mut record::Scene) {
    let scale = f64::from(state.ui_scale());
    let transform = Affine::scale(scale * f64::from(scene.camera.zoom)).then_translate(
        (
            scale * f64::from(scene.camera.x),
            scale * f64::from(scene.camera.y),
        )
            .into(),
    );
    let mut painter = Painter::new(recording);
    draw_grid(state, scene, &mut painter);
    let stroke = Stroke::new(1.5);
    for index in scene.edge_indices() {
        let link = &scene.projection.graph.links[index];
        let active = [scene.selected.as_deref(), scene.hovered.as_deref()]
            .into_iter()
            .flatten()
            .any(|id| link.origin == id || link.destination == id);
        let color = if active {
            state.palette.accent
        } else {
            state.palette.muted
        };
        if let Some(curve) = provisional_edge(scene, index) {
            painter
                .stroke(curve, &stroke, color)
                .transform(transform)
                .draw();
            for dx in [-4., 4.] {
                painter
                    .stroke(
                        Line::new(curve.p3 + Vec2::new(dx, -6.), curve.p3),
                        &stroke,
                        color,
                    )
                    .transform(transform)
                    .draw();
            }
            continue;
        }
        let Some(route) = &scene.projection.graph.routes[index] else {
            continue;
        };
        if !topology::visible(route.bounds, scene.camera, scene.viewport) {
            continue;
        }
        if let Some((path, arrow)) = &scene.projection.paths[index] {
            painter
                .stroke(path, &stroke, color)
                .transform(transform)
                .draw();
            painter
                .stroke(arrow, &stroke, color)
                .transform(transform)
                .draw();
        }
    }
}
fn provisional_edge(scene: &Scene, index: usize) -> Option<CubicBez> {
    let moved = scene.moved_beat()?;
    let link = &scene.projection.graph.links[index];
    if link.origin != moved && link.destination != moved {
        return None;
    }
    let from = scene.node(*scene.projection.graph.ids.get(&link.origin)?);
    let to = scene.node(*scene.projection.graph.ids.get(&link.destination)?);
    let start = Point::new(
        f64::from(from.x + from.width / 2.),
        f64::from(from.y + HEIGHT),
    );
    let end = Point::new(f64::from(to.x + to.width / 2.), f64::from(to.y));
    Some(CubicBez::new(
        start,
        start + Vec2::new(0., 32.),
        end - Vec2::new(0., 32.),
        end,
    ))
}
fn route_labels(state: &Writer, scene: &Scene) -> Vec<impl WidgetView<Writer> + use<>> {
    let scale = f64::from(state.ui_scale());
    scene
        .edge_indices()
        .filter(|index| {
            let link = &scene.projection.graph.links[*index];
            scene
                .selected
                .as_deref()
                .is_some_and(|id| link.origin == id || link.destination == id)
        })
        .filter_map(|index| {
            scene.projection.graph.routes[index]
                .as_ref()
                .map(|route| (index, route))
        })
        .filter(|(_, route)| topology::visible(route.bounds, scene.camera, scene.viewport))
        .take(if scene.detail() == Detail::Overview {
            0
        } else {
            8
        })
        .map(|(index, route)| {
            let link = &scene.projection.graph.links[index];
            let text = format!(
                "{}{}{}",
                link.label.chars().take(40).collect::<String>(),
                if link.conditional {
                    " · conditional"
                } else {
                    ""
                },
                if link.returning { " · return" } else { "" }
            );
            let affine = Affine::scale(scale * f64::from(scene.camera.zoom)).then_translate(
                (
                    scale * f64::from(route.label.0 * scene.camera.zoom + scene.camera.x + 8.),
                    scale * f64::from(route.label.1 * scene.camera.zoom + scene.camera.y),
                )
                    .into(),
            );
            placed::placed(
                affine,
                label(1., text)
                    .text_size(11.)
                    .background_color(state.palette.canvas),
            )
        })
        .collect::<Vec<_>>()
}
fn draw_grid(state: &Writer, scene: &Scene, painter: &mut Painter<'_, record::Scene>) {
    let scale = f64::from(state.ui_scale());
    let step = (40. * f64::from(scene.camera.zoom) * scale).max(16.);
    let stroke = Stroke::new(0.5);
    let mut x = (f64::from(scene.camera.x) * scale).rem_euclid(step);
    let width = f64::from(scene.viewport.0) * scale;
    let height = f64::from(scene.viewport.1) * scale;
    while x < width {
        painter
            .stroke(Line::new((x, 0.), (x, height)), &stroke, state.palette.rule)
            .draw();
        x += step;
    }
    let mut y = (f64::from(scene.camera.y) * scale).rem_euclid(step);
    while y < height {
        painter
            .stroke(Line::new((0., y), (width, y)), &stroke, state.palette.rule)
            .draw();
        y += step;
    }
}
