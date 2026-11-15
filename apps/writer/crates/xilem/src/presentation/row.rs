//! List rows: a row tall, quiet at rest, the selected one on the second-ink plate with a 2px ink
//! bar down its leading edge and the accessible `selected` state, so no row spells its selection
//! into its name.
#[cfg(test)]
mod tests;
mod widget;
use super::control_style::{ControlStyle, styled};
use super::material::{Bar, Edge, bar};
use recite_writer_session::palette::Palette;
use std::marker::PhantomData;
pub(crate) use widget::{RowButton, RowPress};
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    peniko::Color,
};

/// A row of `child` that calls `press` when pressed, marked selected while `selected` holds.
pub(crate) fn row<S: 'static, V: WidgetView<S>, F: Fn(&mut S) + Send + Sync + 'static>(
    selected: bool,
    child: V,
    press: F,
) -> Row<S, V, F> {
    Row {
        selected,
        disabled: false,
        child,
        press,
        phantom: PhantomData,
    }
}

/// Dress a row view in the list look: the plate and leading ink bar while `selected`.
pub(crate) fn listed<S: 'static, R: WidgetView<S, Widget = RowButton>>(
    palette: &Palette<Color>,
    selected: bool,
    row: R,
) -> impl WidgetView<S> + use<S, R> {
    bar(
        selected.then_some(Bar {
            color: palette.ink,
            edge: Edge::Leading,
            inset: 0.,
            lift: 0.,
        }),
        styled(
            if selected {
                ControlStyle::RowSelected
            } else {
                ControlStyle::Row
            },
            row,
        ),
    )
}

pub(crate) struct Row<S, V, F> {
    selected: bool,
    disabled: bool,
    child: V,
    press: F,
    phantom: PhantomData<fn(S)>,
}
impl<S, V, F> Row<S, V, F> {
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

const CONTENT: ViewId = ViewId::new(0);

impl<S, V, F> ViewMarker for Row<S, V, F> {}
impl<S: 'static, V: WidgetView<S>, F: Fn(&mut S) + Send + Sync + 'static> View<S, (), ViewCtx>
    for Row<S, V, F>
{
    type Element = Pod<RowButton>;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut S) -> (Self::Element, Self::ViewState) {
        let (child, local) = ctx.with_id(CONTENT, |ctx| self.child.build(ctx, state));
        let mut pod = ctx.with_action_widget(|ctx| {
            ctx.create_pod(RowButton::new(self.selected, child.new_widget))
        });
        pod.new_widget.options.disabled = self.disabled;
        (pod, local)
    }
    fn rebuild(
        &self,
        previous: &Self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        state: &mut S,
    ) {
        if previous.disabled != self.disabled {
            element.ctx.set_disabled(self.disabled);
        }
        if previous.selected != self.selected {
            RowButton::set_selected(&mut element, self.selected);
        }
        ctx.with_id(CONTENT, |ctx| {
            self.child.rebuild(
                &previous.child,
                local,
                ctx,
                RowButton::child_mut(&mut element).downcast(),
                state,
            );
        });
    }
    fn teardown(
        &self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(CONTENT, |ctx| {
            self.child
                .teardown(local, ctx, RowButton::child_mut(&mut element).downcast());
        });
        ctx.teardown_action_source(element);
    }
    fn message(
        &self,
        local: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        state: &mut S,
    ) -> MessageResult<()> {
        match message.take_first() {
            Some(CONTENT) => self.child.message(
                local,
                message,
                RowButton::child_mut(&mut element).downcast(),
                state,
            ),
            None => match message.take_message::<RowPress>() {
                Some(_) => {
                    (self.press)(state);
                    MessageResult::Action(())
                }
                None => MessageResult::Stale,
            },
            Some(_) => MessageResult::Stale,
        }
    }
}
