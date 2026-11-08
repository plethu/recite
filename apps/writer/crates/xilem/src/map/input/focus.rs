//! A one-shot native focus request, acknowledged by the local viewport.
use super::*;
use xilem::masonry::core::{ActionCtx, ErasedAction};

#[derive(Debug)]
struct FocusViewport;
impl InputWidget {
    pub fn restore_overview_focus(this: &mut WidgetMut<'_, Self>) {
        // The persistent child outlives detail cards; its action reaches this viewport.
        Self::child_mut(this)
            .ctx
            .submit_untyped_action(Box::new(FocusViewport));
    }
    pub fn overview_focus_action(
        &self,
        ctx: &mut ActionCtx<'_>,
        action: &ErasedAction,
        source: core::WidgetId,
    ) {
        if self.kind == Kind::Viewport(Detail::Overview)
            && source == self.child.id()
            && action.downcast_ref::<FocusViewport>().is_some()
        {
            ctx.request_focus();
            ctx.request_scroll_to_this();
            ctx.set_handled();
        }
    }
    pub fn request_card_focus(&self, ctx: &mut UpdateCtx<'_>) {
        if self.kind.is_card()
            && let Some(focus) = &self.focus
        {
            ctx.submit_action::<NativeAction>(NativeAction::FocusTarget {
                target: self.child.id(),
                token: focus.token.clone(),
            });
        }
    }
    pub fn configure_focus(this: &mut WidgetMut<'_, Self>, focus: Option<&FocusRequest>) {
        if this.widget.focus.as_ref() == focus {
            return;
        }
        this.widget.focus = focus.cloned();
        if this.widget.kind.is_card()
            && let Some(focus) = focus
        {
            this.ctx
                .submit_action::<NativeAction>(NativeAction::FocusTarget {
                    target: this.widget.child.id(),
                    token: focus.token.clone(),
                });
        }
    }
    pub fn focus_action(&mut self, ctx: &mut ActionCtx<'_>, action: &ErasedAction) {
        if !matches!(self.kind, Kind::Viewport(_)) {
            return;
        }
        if let Some(NativeAction::FocusTarget { target, token }) =
            action.downcast_ref::<NativeAction>()
        {
            if let Some(focus) = &self.focus
                && focus.token == *token
            {
                ctx.set_focus(*target);
                // This viewport is untransformed. The pinned scroll propagation uses
                // layout origins, so a request from the transformed card is incorrect.
                ctx.request_scroll_to(focus.bounds);
                ctx.submit_action::<NativeAction>(Input::Focused(token.clone()));
            }
            ctx.set_handled();
        }
    }
}
