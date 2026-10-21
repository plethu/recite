//! A bounded native column retains each keyed field through insertions and reordering.
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    masonry::{
        core::CollectionWidget,
        widgets::{Flex, FlexParams},
    },
    view::CrossAxisAlignment,
};

pub(crate) struct Column<K, V> {
    children: Vec<(K, V)>,
}
pub(crate) fn column<K, V>(children: Vec<(K, V)>) -> Column<K, V> {
    Column { children }
}
pub(crate) struct Local<K, T> {
    entries: Vec<Entry<K, T>>,
    next: ViewId,
}
struct Entry<K, T> {
    key: K,
    id: ViewId,
    state: T,
}
impl<K, T> Local<K, T> {
    fn allocate(&mut self) -> ViewId {
        let id = self.next;
        self.next = ViewId::new(id.routing_id().wrapping_add(1));
        id
    }
}
impl<K, V> ViewMarker for Column<K, V> {}
impl<S: 'static, K: Clone + PartialEq + Send + Sync + 'static, V: WidgetView<S>>
    View<S, (), ViewCtx> for Column<K, V>
{
    type Element = Pod<Flex>;
    type ViewState = Local<K, V::ViewState>;

    fn build(&self, ctx: &mut ViewCtx, state: &mut S) -> (Self::Element, Self::ViewState) {
        let mut widget = Flex::column().cross_axis_alignment(CrossAxisAlignment::Start);
        let mut local = Local {
            entries: Vec::with_capacity(self.children.len()),
            next: ViewId::new(0),
        };
        for (key, view) in &self.children {
            let id = local.allocate();
            let (child, inner) = ctx.with_id(id, |ctx| view.build(ctx, state));
            widget = widget.with(child.new_widget, FlexParams::default());
            local.entries.push(Entry {
                key: key.clone(),
                id,
                state: inner,
            });
        }
        (ctx.create_pod(widget), local)
    }

    fn rebuild(
        &self,
        previous: &Self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut widget: Mut<'_, Self::Element>,
        state: &mut S,
    ) {
        // The manuscript mounts at most one page. Linear matching keeps this small
        // adapter inspectable, and native child order follows reading/Tab order.
        for (index, (key, view)) in self.children.iter().enumerate() {
            if let Some(old) = local.entries[index..]
                .iter()
                .position(|item| &item.key == key)
            {
                let old = index + old;
                if old != index {
                    Flex::swap(&mut widget, index, old);
                    local.entries.swap(index, old);
                }
                if let Some((_, previous)) = previous.children.iter().find(|(old, _)| old == key) {
                    let entry = &mut local.entries[index];
                    ctx.with_id(entry.id, |ctx| {
                        view.rebuild(
                            previous,
                            &mut entry.state,
                            ctx,
                            Flex::get_mut(&mut widget, index).downcast(),
                            state,
                        );
                    });
                }
            } else {
                let id = local.allocate();
                let (child, inner) = ctx.with_id(id, |ctx| view.build(ctx, state));
                Flex::insert(&mut widget, index, child.new_widget, FlexParams::default());
                local.entries.insert(
                    index,
                    Entry {
                        key: key.clone(),
                        id,
                        state: inner,
                    },
                );
            }
        }
        while local.entries.len() > self.children.len() {
            let index = local.entries.len() - 1;
            let entry = &mut local.entries[index];
            if let Some((_, view)) = previous.children.iter().find(|(key, _)| key == &entry.key) {
                ctx.with_id(entry.id, |ctx| {
                    view.teardown(
                        &mut entry.state,
                        ctx,
                        Flex::get_mut(&mut widget, index).downcast(),
                    );
                });
            }
            Flex::remove(&mut widget, index);
            local.entries.pop();
        }
    }

    fn teardown(
        &self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut widget: Mut<'_, Self::Element>,
    ) {
        for (index, ((_, view), entry)) in self.children.iter().zip(&mut local.entries).enumerate()
        {
            ctx.with_id(entry.id, |ctx| {
                view.teardown(
                    &mut entry.state,
                    ctx,
                    Flex::get_mut(&mut widget, index).downcast(),
                );
            });
        }
    }

    fn message(
        &self,
        local: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut widget: Mut<'_, Self::Element>,
        state: &mut S,
    ) -> MessageResult<()> {
        let Some(id) = message.take_first() else {
            return MessageResult::Stale;
        };
        let Some(index) = local.entries.iter().position(|entry| entry.id == id) else {
            return MessageResult::Stale;
        };
        self.children[index].1.message(
            &mut local.entries[index].state,
            message,
            Flex::get_mut(&mut widget, index).downcast(),
            state,
        )
    }
}
