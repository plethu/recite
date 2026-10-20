//! A field's native editing state belongs to one document and stable passage ID.
//! Replacing that identity also retires queued messages from the old field.
use xilem::core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker};
use xilem::masonry::{properties::Dimensions, widgets::Passthrough};
use xilem::{Pod, ViewCtx, WidgetView};

pub(super) struct Keyed<K, V> {
    key: K,
    inner: V,
}
pub(super) struct KeyedState<T> {
    generation: u64,
    inner: T,
}
pub(super) fn keyed<K, V>(key: K, inner: V) -> Keyed<K, V> {
    Keyed { key, inner }
}
impl<K, V> ViewMarker for Keyed<K, V> {}
impl<State: 'static, K: PartialEq + Send + Sync + 'static, V: WidgetView<State>>
    View<State, (), ViewCtx> for Keyed<K, V>
{
    type Element = Pod<Passthrough>;
    type ViewState = KeyedState<V::ViewState>;

    fn build(&self, ctx: &mut ViewCtx, state: &mut State) -> (Self::Element, Self::ViewState) {
        let (child, inner) = ctx.with_id(ViewId::new(0), |ctx| self.inner.build(ctx, state));
        (
            Pod::new_with_props(Passthrough::new(child.new_widget), Dimensions::MAX),
            KeyedState {
                generation: 0,
                inner,
            },
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        state: &mut State,
    ) {
        if self.key == prev.key {
            ctx.with_id(ViewId::new(local.generation), |ctx| {
                let mut child = Passthrough::child_mut(&mut element);
                self.inner
                    .rebuild(&prev.inner, &mut local.inner, ctx, child.downcast(), state);
            });
        } else {
            prev.teardown(local, ctx, element.reborrow_mut());
            local.generation = local.generation.wrapping_add(1);
            let (child, inner) = ctx.with_id(ViewId::new(local.generation), |ctx| {
                self.inner.build(ctx, state)
            });
            Passthrough::set_child(&mut element, child.new_widget);
            local.inner = inner;
        }
    }

    fn teardown(
        &self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ViewId::new(local.generation), |ctx| {
            let mut child = Passthrough::child_mut(&mut element);
            self.inner.teardown(&mut local.inner, ctx, child.downcast());
        });
    }

    fn message(
        &self,
        local: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        state: &mut State,
    ) -> MessageResult<()> {
        if message
            .take_first()
            .is_none_or(|id| id.routing_id() != local.generation)
        {
            return MessageResult::Stale;
        }
        let mut child = Passthrough::child_mut(&mut element);
        self.inner
            .message(&mut local.inner, message, child.downcast(), state)
    }
}
