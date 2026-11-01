//! The Map's background dots repeat one rasterized tile instead of encoding every dot.
//!
//! Texels carry the ink colour with antialiased coverage as alpha, so whatever lies beneath
//! the grid, including the viewport frame, stays visible.
use std::sync::Arc;
use xilem::masonry::{
    imaging::Painter,
    kurbo::{Affine, Point, Rect, Vec2},
    peniko::{
        Blob, Brush, Color, Extend, ImageAlphaType, ImageBrush, ImageData, ImageFormat,
        ImageQuality,
    },
};

/// Dot radius in logical pixels, unchanged by zoom. The previous vector grid drew 0.75 px
/// circles that its renderer covered at about 0.29 of each touched pixel; this radius keeps
/// that established printed weight with exact coverage.
const RADIUS: f64 = 0.6;
/// Coverage samples per tile pixel along each axis.
const SUPERSAMPLING: u32 = 16;

/// Dots `step` apart with one centred on `origin`, the camera's world origin.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::map) struct Grid {
    pub step: f64,
    pub origin: Point,
    pub ink: Color,
}

/// A tile depends on spacing, colour and device scale, never on the camera position, so
/// panning reuses it.
pub(super) struct Tile {
    step: f64,
    ink: Color,
    scale: f64,
    pixels: u32,
    brush: Brush,
}
impl Tile {
    fn new(grid: Grid, scale: f64) -> Self {
        let pixels = (grid.step * scale).round().max(1.) as u32;
        let radius = RADIUS * f64::from(pixels) / grid.step;
        let centre = f64::from(pixels) / 2.;
        let [red, green, blue, alpha] = grid.ink.to_rgba8().to_u8_array();
        let mut data = Vec::with_capacity((pixels * pixels * 4) as usize);
        for y in 0..pixels {
            for x in 0..pixels {
                let coverage = coverage(f64::from(x), f64::from(y), centre, radius);
                let alpha = (f64::from(alpha) * coverage).round() as u8;
                data.extend([red, green, blue, alpha]);
            }
        }
        let image = ImageData {
            data: Blob::new(Arc::new(data)),
            format: ImageFormat::Rgba8,
            alpha_type: ImageAlphaType::Alpha,
            width: pixels,
            height: pixels,
        };
        Self {
            step: grid.step,
            ink: grid.ink,
            scale,
            pixels,
            brush: Brush::Image(
                ImageBrush::new(image)
                    .with_extend(Extend::Repeat)
                    .with_quality(ImageQuality::Medium),
            ),
        }
    }
    fn serves(&self, grid: Grid, scale: f64) -> bool {
        self.step == grid.step && self.ink == grid.ink && self.scale == scale
    }
}

/// Fraction of a tile pixel covered by the centred dot.
fn coverage(x: f64, y: f64, centre: f64, radius: f64) -> f64 {
    let samples = f64::from(SUPERSAMPLING);
    let inside = (0..SUPERSAMPLING)
        .flat_map(|sy| (0..SUPERSAMPLING).map(move |sx| (sx, sy)))
        .filter(|&(sx, sy)| {
            let dx = x + (f64::from(sx) + 0.5) / samples - centre;
            let dy = y + (f64::from(sy) + 0.5) / samples - centre;
            dx * dx + dy * dy <= radius * radius
        })
        .count();
    inside as f64 / (samples * samples)
}

/// Fills `area` with the repeating dots, rebuilding the tile only when its inputs change.
pub(super) fn paint(
    grid: Grid,
    scale: f64,
    area: Rect,
    tile: &mut Option<Tile>,
    painter: &mut Painter<'_>,
) {
    let tile = match tile {
        Some(tile) if tile.serves(grid, scale) => tile,
        _ => tile.insert(Tile::new(grid, scale)),
    };
    // Tile pixels map back to logical units, with a dot centred on the camera origin. The tile
    // snaps to device pixels, offset by half a pixel so sampling reads texel centres exactly
    // rather than averaging neighbours.
    let corner = grid.origin.to_vec2() - Vec2::new(grid.step / 2., grid.step / 2.);
    let corner = ((corner * scale).round() + Vec2::new(0.5, 0.5)) / scale;
    let placement = Affine::translate(corner) * Affine::scale(grid.step / f64::from(tile.pixels));
    painter
        .fill(area, &tile.brush)
        .brush_transform(Some(placement))
        .draw();
}

#[cfg(test)]
mod tests;
