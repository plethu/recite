//! Clip orthogonal dashed routes before Skia expands their offscreen dash runs.
use skia_safe::{Canvas, Paint, Path, PathBuilder, PathEffect, PathVerb, Point, Rect};
use std::sync::Arc;

#[derive(Clone, PartialEq)]
pub(super) struct EdgePath {
    path: Path,
    lines: Option<Arc<[Line]>>,
}
#[derive(Clone, Copy, PartialEq)]
struct Line {
    from: Point,
    to: Point,
    distance: f32,
}
impl EdgePath {
    pub(super) fn parse(svg: &str) -> Option<Self> {
        let path = Path::from_svg(svg)?;
        let mut distance = 0.;
        let mut moved = false;
        let lines = path
            .iter()
            .map(|part| match part.verb() {
                PathVerb::Move => {
                    if moved {
                        return None;
                    }
                    moved = true;
                    Some(None)
                }
                PathVerb::Line => {
                    let points = part.points();
                    let (from, to) = (points[0], points[1]);
                    if from == to || (from.x != to.x && from.y != to.y) {
                        return None;
                    }
                    let line = Line { from, to, distance };
                    distance += (to.x - from.x).abs() + (to.y - from.y).abs();
                    Some(Some(line))
                }
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
            .map(|lines| lines.into_iter().flatten().collect::<Arc<[_]>>());
        Some(Self { path, lines })
    }

    pub(super) fn draw(&self, canvas: &Canvas, paint: &mut Paint, dash: &[f32]) {
        let Some(lines) = self.lines.as_ref().filter(|_| !dash.is_empty()) else {
            paint.set_path_effect(PathEffect::dash(dash, 0.));
            canvas.draw_path(&self.path, paint);
            return;
        };
        let Some(mut bounds) = canvas.local_clip_bounds() else {
            return;
        };
        // Keep artificial segment ends outside the visible antialiased stroke.
        bounds.outset((paint.stroke_width() + 2., paint.stroke_width() + 2.));
        let period: f32 = dash.iter().sum();
        let visible = clipped_path(lines, bounds, period).unwrap_or_else(|| self.path.clone());
        paint.set_path_effect(PathEffect::dash(dash, 0.));
        canvas.draw_path(&visible, paint);
    }
}

fn clipped_path(lines: &[Line], bounds: Rect, period: f32) -> Option<Path> {
    if !period.is_finite() || period <= 0. {
        return None;
    }
    let mut path = PathBuilder::new();
    let mut end = None;
    for line in lines.iter() {
        let Some(mut visible) = line.clipped(bounds) else {
            end = None;
            continue;
        };
        if end != Some(visible.from) {
            // Each visible contour begins offscreen on a global dash-cycle
            // boundary. One draw preserves coverage where runs overlap.
            let start = line.aligned_start(visible.from, period)?;
            visible.from = start;
            path.move_to(visible.from);
        }
        path.line_to(visible.to);
        end = Some(visible.to);
    }
    Some(path.detach())
}

impl Line {
    fn aligned_start(self, entry: Point, period: f32) -> Option<Point> {
        if self.distance == 0. && entry == self.from {
            return Some(entry);
        }
        let horizontal = self.from.y == self.to.y;
        let (from, to, entry_along) = if horizontal {
            (self.from.x, self.to.x, entry.x)
        } else {
            (self.from.y, self.to.y, entry.y)
        };
        let trimmed = (f64::from(entry_along) - f64::from(from)).abs();
        let distance = f64::from(self.distance);
        let period = f64::from(period);
        let offset = ((distance + trimmed) / period).floor() * period - distance;
        let direction = f64::from((to - from).signum());
        let along = (f64::from(from) + direction * offset) as f32;
        let extension = (f64::from(entry_along) - f64::from(along)) * direction;
        if !along.is_finite() || !(0. ..=period).contains(&extension) {
            return None;
        }
        Some(if horizontal {
            Point::new(along, self.from.y)
        } else {
            Point::new(self.from.x, along)
        })
    }

    fn clipped(self, bounds: Rect) -> Option<Self> {
        let horizontal = self.from.y == self.to.y;
        let (start, end, cross, low, high, cross_low, cross_high) = if horizontal {
            (
                self.from.x,
                self.to.x,
                self.from.y,
                bounds.left,
                bounds.right,
                bounds.top,
                bounds.bottom,
            )
        } else {
            (
                self.from.y,
                self.to.y,
                self.from.x,
                bounds.top,
                bounds.bottom,
                bounds.left,
                bounds.right,
            )
        };
        if cross < cross_low || cross > cross_high || start.min(end) > high || start.max(end) < low
        {
            return None;
        }
        let clipped_start = start.clamp(low, high);
        let clipped_end = end.clamp(low, high);
        if clipped_start == clipped_end {
            return None;
        }
        let point = |along| {
            if horizontal {
                Point::new(along, cross)
            } else {
                Point::new(cross, along)
            }
        };
        Some(Self {
            from: point(clipped_start),
            to: point(clipped_end),
            distance: self.distance + (clipped_start - start).abs(),
        })
    }
}
#[cfg(test)]
mod tests;
