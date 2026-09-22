//! Restrained graph-card lighting and inset search-field depth.
use freya::prelude::*;

pub(super) fn face(base: Color, depression: f32) -> Fill {
    let [top, bottom] = face_edges(base, depression);
    LinearGradient::new()
        .angle(0.)
        .stop((top, 0.))
        .stop((base, 48.))
        .stop((bottom, 100.))
        .into()
}

pub(super) fn face_edges(base: Color, depression: f32) -> [Color; 2] {
    [
        Color::lerp(base, Color::WHITE, 0.04 * (1. - depression)),
        Color::lerp(base, Color::BLACK, 0.012),
    ]
}

pub(super) fn inset(amount: f32) -> Shadow {
    Shadow::new()
        .inset()
        .y(1.)
        .blur(3.)
        .color(Color::from_af32rgb(0.12 * amount, 0, 0, 0))
}
