//! Retarget colour fades from the current frame, even during rapid pointer movement.
use freya::{
    animation::{AnimNum, Ease, OnChange, use_animation_with_dependencies},
    prelude::*,
};
use skia_safe::{Color, Paint, PaintStyle, Path, PathEffect};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, PartialEq)]
pub(super) struct Edge {
    pub key: String,
    pub zoom: f32,
    pub route: super::routing::Route,
    pub returning: bool,
    pub dimmed: bool,
    pub highlighted: bool,
    pub conditional: bool,
    pub dark: bool,
    pub reduced_motion: bool,
}
impl Component for Edge {
    fn render(&self) -> impl IntoElement {
        let duration = if self.reduced_motion { 0 } else { 180 };
        let initial = if self.highlighted { 1. } else { 0. };
        let current = use_hook(|| Rc::new(Cell::new(initial)));
        let previous = current.clone();
        let animation =
            use_animation_with_dependencies(&self.highlighted, move |config, active| {
                config.on_change(OnChange::Rerun);
                AnimNum::new(previous.get(), if *active { 1. } else { 0. })
                    .time(duration)
                    .ease(Ease::InOut)
            });
        let initial_opacity = if self.dimmed { 0.65 } else { 1. };
        let current_opacity = use_hook(|| Rc::new(Cell::new(initial_opacity)));
        let previous_opacity = current_opacity.clone();
        let opacity = use_animation_with_dependencies(&self.dimmed, move |config, dimmed| {
            config.on_change(OnChange::Rerun);
            AnimNum::new(previous_opacity.get(), if *dimmed { 0.65 } else { 1. }).time(duration)
        });
        current_opacity.set(opacity.get().value());
        let amount = animation.get().value();
        current.set(amount);
        let zoom = self.zoom;
        let (left, top, width, height) = self.route.bounds;
        let dash: &[f32] = match (self.conditional, self.returning) {
            (true, true) => &[6., 3., 1., 3.],
            (true, false) => &[1., 3.],
            (false, true) => &[4., 3.],
            (false, false) => &[],
        };
        let curve = Path::from_svg(&self.route.path);
        let arrow = Path::from_svg(&self.route.arrow);
        let base = if self.dark {
            Color::from_rgb(146, 152, 143)
        } else {
            Color::from_rgb(146, 153, 142)
        };
        let accent = if self.dark {
            Color::from_rgb(180, 203, 164)
        } else {
            Color::from_rgb(72, 99, 77)
        };
        let layer = |color: Color, thickness: f32| {
            let curve = curve.clone();
            let arrow = arrow.clone();
            canvas(RenderCallback::new(move |context| {
                let canvas = context.canvas;
                canvas.save();
                canvas.scale((zoom, zoom));
                canvas.translate((-left, -top));
                let mut paint = Paint::default();
                paint.set_anti_alias(true);
                paint.set_style(PaintStyle::Stroke);
                paint.set_color(color);
                paint.set_stroke_width(thickness / zoom.max(0.25));
                paint.set_path_effect(PathEffect::dash(dash, 0.));
                if let Some(curve) = &curve {
                    canvas.draw_path(curve, &paint);
                }
                paint.set_path_effect(None);
                if let Some(arrow) = &arrow {
                    canvas.draw_path(arrow, &paint);
                }
                canvas.restore();
            }))
            .key(format!(
                "{color:?}:{}:{}",
                self.route.path, self.route.arrow
            ))
            .width(Size::px(width * zoom))
            .height(Size::px(height * zoom))
        };
        // Draw vector paths directly. Panning never creates or resizes raster assets.
        rect()
            .position(Position::new_absolute().left(left * zoom).top(top * zoom))
            .width(Size::px(width * zoom))
            .height(Size::px(height * zoom))
            .opacity(opacity.get().value())
            .child(layer(base, 1.))
            .child(
                rect()
                    .position(Position::new_absolute().left(0.).top(0.))
                    .opacity(amount)
                    .child(layer(accent, 2.)),
            )
    }
    fn render_key(&self) -> DiffKey {
        DiffKey::from(&self.key)
    }
}
