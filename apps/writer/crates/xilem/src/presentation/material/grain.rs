//! Paper grain for desk surfaces: the identity tile, decoded once and repeated as an image brush.
//!
//! Grain belongs to the desk only. Sheets, panels, fields and cards paint opaque backgrounds over
//! it, so the reading surface and prose never sit on texture.
#[cfg(test)]
mod tests;
use super::decor::{Decorated, Decoration};
use std::{io::Cursor, sync::Arc, sync::OnceLock};
use xilem::peniko::{
    Blob, Color, Extend, ImageAlphaType, ImageBrush, ImageData, ImageFormat, ImageQuality,
};

const TILE: &[u8] = include_bytes!("../../../../../../../assets/identity/grain.png");
/// The same fibre at about two thirds of the alpha, for the paper-toned grain on dark paper.
const TILE_DARK: &[u8] = include_bytes!("../../../../../../../assets/identity/grain-dark.png");

type Coverage = Option<(u32, u32, Vec<u8>)>;

/// The tile's coverage: one alpha byte per pixel, with its width and height.
fn coverage(dark: bool) -> Option<&'static (u32, u32, Vec<u8>)> {
    static LIGHT: OnceLock<Coverage> = OnceLock::new();
    static DARK: OnceLock<Coverage> = OnceLock::new();
    let (cell, tile) = if dark {
        (&DARK, TILE_DARK)
    } else {
        (&LIGHT, TILE)
    };
    cell.get_or_init(|| {
        let mut reader = png::Decoder::new(Cursor::new(tile)).read_info().ok()?;
        let mut pixels = vec![0; reader.output_buffer_size()?];
        let frame = reader.next_frame(&mut pixels).ok()?;
        (frame.color_type == png::ColorType::Rgba && frame.bit_depth == png::BitDepth::Eight).then(
            || {
                let alpha = pixels[..frame.buffer_size()]
                    .chunks_exact(4)
                    .map(|pixel| pixel[3])
                    .collect();
                (frame.width, frame.height, alpha)
            },
        )
    })
    .as_ref()
}

/// The light or dark tile in `ink` at its own alpha, repeating in both directions. `None` if it
/// cannot be read.
pub(crate) fn grain(ink: Color, dark: bool) -> Option<ImageBrush> {
    let (width, height, alpha) = coverage(dark)?;
    let [red, green, blue, _] = ink.to_rgba8().to_u8_array();
    let pixels = alpha
        .iter()
        .flat_map(|&alpha| [red, green, blue, alpha])
        .collect::<Vec<_>>();
    Some(
        ImageBrush::new(ImageData {
            data: Blob::new(Arc::new(pixels)),
            format: ImageFormat::Rgba8,
            alpha_type: ImageAlphaType::Alpha,
            width: *width,
            height: *height,
        })
        .with_extend(Extend::Repeat)
        .with_quality(ImageQuality::Low),
    )
}

/// A desk surface: grain painted over the background and beneath `child`.
pub(crate) fn desk<V>(grain: Option<ImageBrush>, child: V) -> Decorated<V> {
    Decorated::new(Decoration::Grain(grain), child)
}
