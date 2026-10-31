//! Viewport composition and route painting share one coordinate transform.
use super::*;
use crate::{presentation::group, view::workspace};
use xilem::masonry::imaging::record;
use xilem::masonry::kurbo::{self, BezPath, Circle, CubicBez, Line, Point, Shape, Vec2};
use xilem::masonry::properties;
pub(super) fn viewport(state: &Writer, scene: &Scene) -> impl WidgetView<Writer> + use<> {
    let camera = scene.camera;
    let zoom = f64::from(camera.zoom);
    let cards = scene
        .mounted_indices()
        .map(|index| {
            let node = scene.node(index);
            let transform = Affine::scale(zoom).then_translate(
                (
                    f64::from(node.x * camera.zoom + camera.x),
                    f64::from(node.y * camera.zoom + camera.y),
                )
                    .into(),
            );
            (
                super::cards::key(&scene.projection.graph, index),
                placed::placed(transform, card::card(state, scene, index)),
            )
        })
        .collect::<Vec<_>>();
    let width = workspace::Layout::resolve(state).map;
    let drawing = super::routes::routes(width.px(), draw);
    let host = input::input(
        input::Kind::Viewport(scene.detail()),
        state.config.ui.keymap == recite_config::Keymap::Vim,
        // Cards own the final layer; variable children only append at its end.
        // Full reply captions remain in the explicit Connections list.
        zstack((
            drawing,
            group::list("Scene beats", super::cards::cards(cards)),
        ))
        .alignment(UnitPoint::TOP_LEFT),
        |state: &mut Writer, action| state.map_input(Source::Viewport, action),
    );
    let host = host
        .camera(
            scene.camera,
            matches!(scene.gesture, Gesture::Panning { .. }),
        )
        .focus(focus_request(scene))
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
        .width(width.px()),
    )
}
/// Routes, the dot grid and the overview share viewport coordinates.
fn draw(state: &Writer, recording: &mut record::Scene) {
    if let Some(scene) = state.map.scene(state) {
        draw_edges(state, scene, recording);
        if scene.detail() == Detail::Overview {
            super::overview::draw(state, scene, recording);
        }
    }
}
pub(super) fn focus_request(scene: &Scene) -> Option<input::FocusRequest> {
    let focus = scene.focus.as_ref()?;
    let index = scene.projection.graph.ids.get(&focus.beat)?;
    let node = scene.node(*index);
    let zoom = f64::from(scene.camera.zoom);
    let origin = Point::new(
        f64::from(node.x * scene.camera.zoom + scene.camera.x),
        f64::from(node.y * scene.camera.zoom + scene.camera.y),
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
    let transform = Affine::scale(f64::from(scene.camera.zoom))
        .then_translate((f64::from(scene.camera.x), f64::from(scene.camera.y)).into());
    let mut painter = Painter::new(recording);
    draw_grid(state, scene, &mut painter);
    for index in scene.edge_indices() {
        let link = &scene.projection.graph.links[index];
        let active = [scene.selected.as_deref(), scene.hovered.as_deref()]
            .into_iter()
            .flatten()
            .any(|id| link.origin == id || link.destination == id);
        let color = if link.returning {
            state.palette.teal
        } else {
            state.palette.accent
        };
        // Match screen-space ink weights at every zoom level.
        let weight = if link.returning {
            1.
        } else if active {
            2.
        } else {
            1.5
        };
        let stroke = Stroke::new(weight / f64::from(scene.camera.zoom))
            .with_caps(kurbo::Cap::Round)
            .with_join(kurbo::Join::Round);
        let dashes: &[f64] = match (link.conditional, link.returning) {
            (true, true) => &[6., 3., 1., 3.],
            (true, false) => &[1., 3.],
            (false, true) => &[4., 3.],
            (false, false) => &[],
        };
        let route_stroke = stroke.clone().with_dashes(
            0.,
            dashes
                .iter()
                .map(|length| length / f64::from(scene.camera.zoom)),
        );
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
                .stroke(path, &route_stroke, color)
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
/// Dots cover the overscan that the viewport may reveal before committing a pan.
fn draw_grid(state: &Writer, scene: &Scene, painter: &mut Painter<'_, record::Scene>) {
    let step = (24. * f64::from(scene.camera.zoom)).max(8.);
    let color = state.palette.canvas.lerp_rect(state.palette.ink, 0.16);
    let mut dots = BezPath::new();
    let margin = (f64::from(topology::OVERSCAN) / step).ceil() * step;
    let first = |origin: f32| f64::from(origin).rem_euclid(step) - margin;
    let width = f64::from(scene.viewport.0) + margin;
    let height = f64::from(scene.viewport.1) + margin;
    let mut x = first(scene.camera.x);
    while x < width {
        let mut y = first(scene.camera.y);
        while y < height {
            dots.extend(Circle::new((x, y), 0.75).path_elements(0.1));
            y += step;
        }
        x += step;
    }
    // Non-overlapping dots share one fill, rather than one renderer command per dot.
    painter.fill(dots, color).draw();
}
