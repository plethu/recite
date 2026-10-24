//! Visible cards retain their native identity when panning changes mount membership.
use crate::Writer;
use std::collections::BTreeMap;
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    masonry::{core::CollectionWidget, layout::UnitPoint, widgets::ZStack},
};

pub(super) struct Cards<V> {
    children: Vec<(String, V)>,
}
pub(super) fn cards<V>(children: Vec<(String, V)>) -> Cards<V> {
    Cards { children }
}
pub(super) struct Local<T> {
    entries: Vec<Entry<T>>,
    next: ViewId,
}
struct Entry<T> {
    beat: String,
    id: ViewId,
    state: T,
}
impl<T> Local<T> {
    fn allocate(&mut self) -> ViewId {
        let id = self.next;
        self.next = ViewId::new(id.routing_id().wrapping_add(1));
        id
    }
}
impl<V> ViewMarker for Cards<V> {}
impl<V: WidgetView<Writer>> View<Writer, (), ViewCtx> for Cards<V> {
    type Element = Pod<ZStack>;
    type ViewState = Local<V::ViewState>;
    fn build(&self, ctx: &mut ViewCtx, writer: &mut Writer) -> (Self::Element, Self::ViewState) {
        let mut widget = ZStack::new().with_alignment(UnitPoint::TOP_LEFT);
        let mut local = Local {
            entries: Vec::with_capacity(self.children.len()),
            next: ViewId::new(0),
        };
        for (beat, view) in &self.children {
            let id = local.allocate();
            let (child, state) = ctx.with_id(id, |ctx| view.build(ctx, writer));
            widget = widget.with(child.new_widget, UnitPoint::TOP_LEFT);
            local.entries.push(Entry {
                beat: beat.clone(),
                id,
                state,
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
        writer: &mut Writer,
    ) {
        let previous: BTreeMap<_, _> = previous
            .children
            .iter()
            .map(|(beat, view)| (beat, view))
            .collect();
        let mut positions: BTreeMap<_, _> = local
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.beat.clone(), index))
            .collect();
        // Append/swap avoids shifting every native child when visibility changes.
        // Lookups scale with mounted cards, rather than the complete source graph.
        for (index, (beat, view)) in self.children.iter().enumerate() {
            let existing = positions.get(beat).copied();
            let old = existing.unwrap_or_else(|| {
                let id = local.allocate();
                let (child, state) = ctx.with_id(id, |ctx| view.build(ctx, writer));
                ZStack::add(&mut widget, child.new_widget, UnitPoint::TOP_LEFT);
                local.entries.push(Entry {
                    beat: beat.clone(),
                    id,
                    state,
                });
                local.entries.len() - 1
            });
            if old != index {
                ZStack::swap(&mut widget, old, index);
                local.entries.swap(old, index);
                positions.insert(local.entries[old].beat.clone(), old);
            }
            positions.insert(beat.clone(), index);
            if existing.is_some()
                && let Some(previous) = previous.get(beat)
            {
                let entry = &mut local.entries[index];
                ctx.with_id(entry.id, |ctx| {
                    view.rebuild(
                        previous,
                        &mut entry.state,
                        ctx,
                        ZStack::get_mut(&mut widget, index).downcast(),
                        writer,
                    )
                });
            }
        }
        while local.entries.len() > self.children.len() {
            let index = local.entries.len() - 1;
            let entry = &mut local.entries[index];
            if let Some(view) = previous.get(&entry.beat) {
                ctx.with_id(entry.id, |ctx| {
                    view.teardown(
                        &mut entry.state,
                        ctx,
                        ZStack::get_mut(&mut widget, index).downcast(),
                    )
                });
            }
            ZStack::remove(&mut widget, index);
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
                    ZStack::get_mut(&mut widget, index).downcast(),
                )
            });
        }
    }
    fn message(
        &self,
        local: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut widget: Mut<'_, Self::Element>,
        writer: &mut Writer,
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
            ZStack::get_mut(&mut widget, index).downcast(),
            writer,
        )
    }
}
