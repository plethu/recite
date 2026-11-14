//! Paper grain for desk surfaces: the identity tile, decoded once and repeated as an image brush.
//!
//! Grain belongs to the desk only. Sheets, panels, fields and cards paint opaque backgrounds over
//! it, so the reading surface and prose never sit on texture.
#[cfg(test)]
mod tests;
mod widget;
use std::{io::Cursor, sync::Arc, sync::OnceLock};
use widget::Desk;
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    masonry::properties::Dimensions,
    peniko::{
        Blob, Color, Extend, ImageAlphaType, ImageBrush, ImageData, ImageFormat, ImageQuality,
    },
};

const TILE: &[u8] = include_bytes!("../../../../../../../assets/identity/grain.png");

/// The tile's coverage: one alpha byte per pixel, with its width and height.
fn coverage() -> Option<&'static (u32, u32, Vec<u8>)> {
    static COVERAGE: OnceLock<Option<(u32, u32, Vec<u8>)>> = OnceLock::new();
    COVERAGE
        .get_or_init(|| {
            let mut reader = png::Decoder::new(Cursor::new(TILE)).read_info().ok()?;
            let mut pixels = vec![0; reader.output_buffer_size()?];
            let frame = reader.next_frame(&mut pixels).ok()?;
            (frame.color_type == png::ColorType::Rgba && frame.bit_depth == png::BitDepth::Eight)
                .then(|| {
                    let alpha = pixels[..frame.buffer_size()]
                        .chunks_exact(4)
                        .map(|pixel| pixel[3])
                        .collect();
                    (frame.width, frame.height, alpha)
                })
        })
        .as_ref()
}

/// The tile in `ink` at its own alpha, repeating in both directions. `None` if it cannot be read.
pub(crate) fn grain(ink: Color) -> Option<ImageBrush> {
    let (width, height, alpha) = coverage()?;
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
pub(crate) fn desk<V>(grain: Option<ImageBrush>, child: V) -> Grained<V> {
    Grained { grain, child }
}
pub(crate) struct Grained<V> {
    grain: Option<ImageBrush>,
    child: V,
}
impl<V> ViewMarker for Grained<V> {}
impl<S: 'static, V: WidgetView<S>> View<S, (), ViewCtx> for Grained<V> {
    type Element = Pod<Desk>;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut S) -> (Self::Element, Self::ViewState) {
        let (child, local) = ctx.with_id(ViewId::new(0), |ctx| self.child.build(ctx, state));
        (
            Pod::new_with_props(
                Desk::new(self.grain.clone(), child.new_widget),
                Dimensions::AUTO,
            ),
            local,
        )
    }
    fn rebuild(
        &self,
        previous: &Self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut pod: Mut<'_, Self::Element>,
        state: &mut S,
    ) {
        if self.grain != previous.grain {
            Desk::set_grain(&mut pod, self.grain.clone());
        }
        ctx.with_id(ViewId::new(0), |ctx| {
            self.child.rebuild(
                &previous.child,
                local,
                ctx,
                Desk::child_mut(&mut pod).downcast(),
                state,
            );
        });
    }
    fn teardown(
        &self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut pod: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            self.child
                .teardown(local, ctx, Desk::child_mut(&mut pod).downcast())
        });
    }
    fn message(
        &self,
        local: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut pod: Mut<'_, Self::Element>,
        state: &mut S,
    ) -> MessageResult<()> {
        if message.take_first() != Some(ViewId::new(0)) {
            return MessageResult::Stale;
        }
        self.child
            .message(local, message, Desk::child_mut(&mut pod).downcast(), state)
    }
}
