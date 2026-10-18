//! Restrained graph-card lighting, inset search-field depth, paper grain and print marks.
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
        .color(Color::from_af32rgb(0.12 * amount, 0, 0, 0))
}

/// A hard offset plate in a second ink: the print register shifted, never blurred.
pub(crate) fn offset(color: Color, distance: f32) -> Shadow {
    Shadow::new().x(distance).y(distance).color(color)
}

thread_local! {
    static GRAIN: Option<skia_safe::Image> = skia_safe::Image::from_encoded(
        skia_safe::Data::new_copy(include_bytes!("../../../../../../assets/identity/grain.png")),
    );
}

/// Paper grain for canvas and sidebar surfaces, tinted with the current ink. Place it as the
/// first child of a surface; it never sits on the reading surface or behind prose and code.
pub(crate) fn grain(ink: Color) -> impl IntoElement {
    let tint = super::palette::skia_opaque(ink);
    rect()
        .background(RenderCallback::new(move |context| {
            GRAIN.with(|image| {
                let Some(image) = image else { return };
                // The canvas is in logical pixels; keep one grain texel per device pixel.
                let scale = context.canvas.local_to_device_as_3x3().scale_x().max(1.);
                let matrix = skia_safe::Matrix::scale((1. / scale, 1. / scale));
                let Some(shader) = image.to_shader(
                    Some((skia_safe::TileMode::Repeat, skia_safe::TileMode::Repeat)),
                    skia_safe::SamplingOptions::default(),
                    Some(&matrix),
                ) else {
                    return;
                };
                let mut paint = skia_safe::Paint::default();
                paint.set_shader(shader);
                paint.set_color_filter(skia_safe::color_filters::blend(
                    tint,
                    skia_safe::BlendMode::SrcIn,
                ));
                context.canvas.draw_rect(
                    skia_safe::Rect::from_wh(context.size.width, context.size.height),
                    &paint,
                );
            });
        }))
        .key(format!("grain:{ink:?}"))
        .position(Position::new_absolute().left(0.).top(0.))
        .width(Size::fill())
        .height(Size::fill())
}

/// The printer's registration mark: a ring with a crosshair, drawn at `size` logical pixels.
pub(crate) fn registration(color: Color, size: f32) -> impl IntoElement {
    let ink = super::palette::skia_opaque(color);
    rect()
        .background(RenderCallback::new(move |context| {
            let mut paint = skia_safe::Paint::default();
            paint.set_anti_alias(true);
            paint.set_style(skia_safe::PaintStyle::Stroke);
            paint.set_stroke_width(1.);
            paint.set_color(ink);
            let c = size / 2.;
            context.canvas.draw_circle((c, c), size * 0.3, &paint);
            context.canvas.draw_line((c, 0.5), (c, size - 0.5), &paint);
            context.canvas.draw_line((0.5, c), (size - 0.5, c), &paint);
        }))
        .key(format!("registration:{ink:?}"))
        .width(Size::px(size))
        .height(Size::px(size))
}

/// The map's dot grid, in screen space but offset with the camera so it pans with the cards.
pub(crate) fn dot_grid(color: Color, spacing: f32, offset: (f32, f32)) -> impl IntoElement {
    let ink = super::palette::skia_opaque(color);
    let spacing = spacing.max(8.);
    rect()
        .background(RenderCallback::new(move |context| {
            let mut paint = skia_safe::Paint::default();
            paint.set_anti_alias(true);
            paint.set_color(ink);
            let (width, height) = (context.size.width, context.size.height);
            let mut y = offset.1.rem_euclid(spacing);
            while y < height {
                let mut x = offset.0.rem_euclid(spacing);
                while x < width {
                    context.canvas.draw_circle((x, y), 1., &paint);
                    x += spacing;
                }
                y += spacing;
            }
        }))
        .key(format!("dot_grid:{ink:?}"))
        .position(Position::new_absolute().left(0.).top(0.))
        .width(Size::fill())
        .height(Size::fill())
}

/// The colon device as one square turned 45°: selection markers, end marks and loading.
pub(crate) fn diamond(color: Color, size: f32) -> Rect {
    rect()
        .width(Size::px(size))
        .height(Size::px(size))
        .background(color)
        .rotate(45.)
}

/// A halftone dot screen in one ink. With `fade`, dots grow from nothing at the top-left to full
/// strength in the bottom-right corner, like a corner of a printed tint. Decorative only: it never
/// sits behind prose or text smaller than a title.
pub(crate) fn halftone(
    color: Color,
    alpha: f32,
    dot: f32,
    grid: f32,
    fade: bool,
) -> impl IntoElement {
    let ink = super::palette::skia_opaque(color);
    rect()
        .background(RenderCallback::new(move |context| {
            let mut paint = skia_safe::Paint::default();
            paint.set_anti_alias(true);
            let (width, height) = (context.size.width, context.size.height);

            let mut y = grid / 2.;
            while y < height {
                let mut x = grid / 2.;
                while x < width {
                    let strength = if fade {
                        // Distance from the bottom-right corner, normalised per axis.
                        let dx = (width - x) / width.max(1.);
                        let dy = (height - y) / height.max(1.);
                        (1. - (dx * dx + dy * dy).sqrt() * 1.25).clamp(0., 1.)
                    } else {
                        1.
                    };
                    if strength > 0. {
                        paint.set_color(ink.with_a((alpha * strength * 255.).round() as u8));
                        context
                            .canvas
                            .draw_circle((x, y), dot / 2. * strength.sqrt(), &paint);
                    }
                    x += grid;
                }
                y += grid;
            }
        }))
        .key(format!("halftone:{ink:?}"))
        .position(Position::new_absolute().left(0.).top(0.))
        .width(Size::fill())
        .height(Size::fill())
}

/// A dashed rounded frame, as for a placeholder printed on the sheet.
pub(crate) fn dashed_frame(color: Color, stroke: f32, radius: f32) -> impl IntoElement {
    let ink = super::palette::skia_opaque(color);
    rect()
        .background(RenderCallback::new(move |context| {
            let mut paint = skia_safe::Paint::default();
            paint.set_anti_alias(true);
            paint.set_style(skia_safe::PaintStyle::Stroke);
            paint.set_stroke_width(stroke);
            paint.set_color(ink);
            paint.set_path_effect(skia_safe::PathEffect::dash(&[5., 4.], 0.));
            let half = stroke / 2.;
            let rect = skia_safe::Rect::from_xywh(
                half,
                half,
                context.size.width - stroke,
                context.size.height - stroke,
            );
            context
                .canvas
                .draw_rrect(skia_safe::RRect::new_rect_xy(rect, radius, radius), &paint);
        }))
        .key(format!("dashed_frame:{ink:?}"))
        .position(Position::new_absolute().left(0.).top(0.))
        .width(Size::fill())
        .height(Size::fill())
}

/// A dotted hairline across the available width.
pub(crate) fn dotted_rule(color: Color, slot: &str) -> impl IntoElement {
    let ink = super::palette::skia_opaque(color);
    rect()
        .background(RenderCallback::new(move |context| {
            let mut paint = skia_safe::Paint::default();
            paint.set_anti_alias(true);
            paint.set_style(skia_safe::PaintStyle::Stroke);
            paint.set_stroke_width(1.);
            paint.set_color(ink);
            paint.set_path_effect(skia_safe::PathEffect::dash(&[1., 3.], 0.));
            let y = (context.size.height / 2.).floor() + 0.5;
            context
                .canvas
                .draw_line((0., y), (context.size.width, y), &paint);
        }))
        .key(format!("dotted_rule:{slot}:{ink:?}"))
        .width(Size::flex(1.))
        .height(Size::px(3.))
}

/// Loading: the colon's two diamonds trade plum and teal on each `phase`. The caller holds the
/// phase still under reduced motion.
pub(crate) fn loading_mark(swapped: bool) -> Rect {
    let p = super::palette::current();
    let (first, second) = if swapped {
        (p.teal, p.accent)
    } else {
        (p.accent, p.teal)
    };
    rect()
        .spacing(super::tokens::SPACE_XS)
        .padding(2.)
        .a11y_role(AccessibilityRole::ProgressIndicator)
        .child(diamond(first, 6.))
        .child(diamond(second, 6.))
}
