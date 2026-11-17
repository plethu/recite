//! The modal backdrop: a static overprint dim under a paper-coloured print texture, behind
//! pickers and other modal slips. Nothing animates and nothing blurs; Vello CPU has no backdrop
//! filter. Like the grain, the texture is one small tile built once and repeated by the renderer.
#[cfg(test)]
mod tests;
use super::decor::{Decorated, Decoration};
use std::sync::Arc;
use xilem::peniko::{
    Blob, Color, Extend, ImageAlphaType, ImageBrush, ImageData, ImageFormat, ImageQuality,
};

/// The texture printed over the dim.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Texture {
    /// A dot screen at a 5px pitch.
    #[default]
    Dots,
    /// A 45° hatch at the same pitch. Shown only in the gallery, for the maintainer's comparison.
    #[cfg_attr(not(test), expect(dead_code, reason = "a gallery-only comparison"))]
    Hatch,
}

/// The screen's pitch in logical pixels: one dot, or one hatch line, per tile.
pub(crate) const PITCH: u32 = 5;

/// Coverage of one tile, 0 to 255 per pixel.
fn coverage(texture: Texture) -> [u8; (PITCH * PITCH) as usize] {
    let mut tile = [0; (PITCH * PITCH) as usize];
    let at = |x: u32, y: u32| (y * PITCH + x) as usize;
    match texture {
        Texture::Dots => {
            // A dot about a pixel and a half across: a full centre and softer arms.
            tile[at(2, 2)] = 255;
            for (x, y) in [(1, 2), (3, 2), (2, 1), (2, 3)] {
                tile[at(x, y)] = 96;
            }
        }
        Texture::Hatch => {
            for step in 0..PITCH {
                tile[at(step, PITCH - 1 - step)] = 255;
            }
        }
    }
    tile
}

/// The texture in `ink` at `ink`'s own alpha, repeating in both directions.
pub(crate) fn screen(texture: Texture, ink: Color) -> ImageBrush {
    let [red, green, blue, alpha] = ink.to_rgba8().to_u8_array();
    let pixels = coverage(texture)
        .iter()
        .flat_map(|&cover| {
            let cover = (u16::from(cover) * u16::from(alpha) / 255) as u8;
            [red, green, blue, cover]
        })
        .collect::<Vec<_>>();
    ImageBrush::new(ImageData {
        data: Blob::new(Arc::new(pixels)),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::Alpha,
        width: PITCH,
        height: PITCH,
    })
    .with_extend(Extend::Repeat)
    .with_quality(ImageQuality::Low)
}

/// A modal layer: `dim` and `screen` painted over the whole layer, beneath `child`.
pub(crate) fn backdrop<V>(dim: Color, screen: ImageBrush, child: V) -> Decorated<V> {
    Decorated::new(Decoration::Backdrop(dim, screen), child)
}

/// `child` where a modal layer leaves it in place: while given, a dim and screen painted over it,
/// as [`backdrop`] paints them beneath its own child.
pub(crate) fn cover<V>(backdrop: Option<(Color, ImageBrush)>, child: V) -> Decorated<V> {
    Decorated::new(Decoration::Cover(backdrop), child)
}
