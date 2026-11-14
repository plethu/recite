//! Decorative paint around one child: desk grain beneath it, or a focus ring above it.
//! Neither owns input, focus or an accessible name; the child keeps all three.
#[cfg(test)]
mod tests;
mod widget;
use widget::Decor;
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
