//! Decorative paint around one child: desk grain beneath it, or a focus ring above it.
//! Neither owns input, focus or an accessible name; the child keeps all three.
#[cfg(test)]
mod tests;
mod widget;
use crate::presentation::marks::Marks;
pub(crate) use widget::Decor;
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    masonry::properties::Dimensions,
    peniko::{Color, ImageBrush},
};

#[derive(Clone, PartialEq)]
pub(crate) enum Decoration {
    /// The desk's repeating grain, painted beneath the child.
    Grain(Option<ImageBrush>),
    /// A 2px ring inside the child's edge while focus is within it, for filled controls whose
    /// own border would vanish into the fill.
    Ring(Option<Color>),
    /// A 2px outline `offset` pixels outside the child's edge while focus is within it, for text
    /// whose own border would crowd its glyphs and caret. It takes no layout room.
    Outline(Option<Color>, f64),
    /// A 2px bar along the bottom: a chosen tab's underline or a chosen segment's mark.
    Bar(Option<Bar>),
    /// The modal backdrop over the whole child: a dim, then a print texture, beneath the child.
    Backdrop(Color, ImageBrush),
    /// The same backdrop painted over the child, while given: a region a modal layer leaves in
    /// place, such as the window behind Settings around the toolbar that toggles it.
    Cover(Option<(Color, ImageBrush)>),
    /// Printer's marks around the child: crop marks, the unsaved dot. They paint outside it, take no
    /// room and never take input.
    Marks(Marks),
    /// A slip's stock: the hard offset edge of the paper beneath it, filling the child's last
    /// `offset` pixels on the right and at the foot, less the corners the slip's face leaves bare.
    Stock(Color, f64),
}

/// Where a [`Decoration::Bar`] sits along the bottom of its child. A bar runs along the bottom
/// only: a leading-edge bar was the old mark for the current beat, unsaved work, selected rows and
/// banners, and the printer's marks (`presentation::marks`) replace it.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Bar {
    pub color: Color,
    /// Distance in from both ends of the bottom edge.
    pub inset: f64,
    /// Distance up from the bottom edge to the bar.
    pub lift: f64,
}
impl Bar {
    /// A solid 2px bar in `color` under the child.
    pub fn under(color: Color, inset: f64, lift: f64) -> Self {
        Self { color, inset, lift }
    }
}

pub(crate) struct Decorated<V> {
    decoration: Decoration,
    child: V,
}
impl<V> Decorated<V> {
    pub fn new(decoration: Decoration, child: V) -> Self {
        Self { decoration, child }
    }
}
impl<V> ViewMarker for Decorated<V> {}
impl<S: 'static, V: WidgetView<S>> View<S, (), ViewCtx> for Decorated<V> {
    type Element = Pod<Decor>;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut S) -> (Self::Element, Self::ViewState) {
        let (child, local) = ctx.with_id(ViewId::new(0), |ctx| self.child.build(ctx, state));
        (
            Pod::new_with_props(
                Decor::new(self.decoration.clone(), child.new_widget),
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
        if self.decoration != previous.decoration {
            Decor::set_decoration(&mut pod, self.decoration.clone());
        }
        ctx.with_id(ViewId::new(0), |ctx| {
            self.child.rebuild(
                &previous.child,
                local,
                ctx,
                Decor::child_mut(&mut pod).downcast(),
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
                .teardown(local, ctx, Decor::child_mut(&mut pod).downcast())
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
            .message(local, message, Decor::child_mut(&mut pod).downcast(), state)
    }
}
