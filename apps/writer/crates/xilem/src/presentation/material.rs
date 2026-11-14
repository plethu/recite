//! Decorative print marks reuse the native canvas recording and do not own input.
use xilem::{
    WidgetView,
    core::memoize,
    masonry::{
        imaging::Painter,
        kurbo::{BezPath, Circle, Rect, Shape},
        layout::AsUnit,
    },
    peniko::Color,
    style::Style,
    view::{canvas, sized_box},
};

mod decor;
mod grain;
pub(crate) use decor::{Decorated, Decoration};
pub(crate) use grain::{desk, grain};

/// Filled controls show keyboard focus as a ring of `color` inside their edge.
pub(crate) fn focus_ring<V>(color: Option<Color>, child: V) -> Decorated<V> {
    Decorated::new(Decoration::Ring(color), child)
}

fn dots(bounds: Rect) -> BezPath {
    let mut dots = BezPath::new();
    let mut y = bounds.y0 + 2.5;
    while y < bounds.y1 {
        let mut x = bounds.x0 + 2.5;
        while x < bounds.x1 {
            dots.extend(Circle::new((x, y), 0.5).path_elements(0.1));
            x += 5.;
        }
        y += 5.;
    }
    dots
}

/// Cache the dot screen by its geometry and ink, independently of camera movement.
/// Dots form one fill operation, rather than hundreds of individual draw commands.
pub(crate) fn halftone<S: 'static>(width: f64, height: f64, ink: Color) -> impl WidgetView<S> {
    memoize((width, height, ink), |&(width, height, ink)| {
        let dots = dots(Rect::new(0., 0., width, height));
        sized_box(
            canvas(move |_: &mut S, _, scene, _| {
                Painter::new(scene).fill(&dots, ink.with_alpha(0.35)).draw();
            })
            .alt_text(""),
        )
        .dims((width.px(), height.px()))
    })
}
