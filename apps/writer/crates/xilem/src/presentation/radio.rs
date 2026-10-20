//! A scaled label for Masonry's native radio interaction and accessibility.
use xilem::{
    Pod, ViewCtx,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::{
        core::{ArcStr, NewWidget, StyleProperty},
        widgets::{Label, RadioButton, RadioButtonSelected},
    },
};

pub(crate) struct Radio<F> {
    text: ArcStr,
    checked: bool,
    size: f32,
    disabled: bool,
    select: F,
}
pub(crate) fn radio_button<S: 'static>(
    scale: f32,
    text: impl Into<ArcStr>,
    checked: bool,
    select: impl Fn(&mut S) + Send + Sync + 'static,
) -> Radio<impl Fn(&mut S) + Send + Sync + 'static> {
    Radio {
        text: text.into(),
        checked,
        size: 14. * scale,
        disabled: false,
        select,
    }
}
impl<F> Radio<F> {
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}
impl<F> ViewMarker for Radio<F> {}
impl<S: 'static, F: Fn(&mut S) + Send + Sync + 'static> View<S, (), ViewCtx> for Radio<F> {
    type Element = Pod<RadioButton>;
    type ViewState = ();
    fn build(&self, ctx: &mut ViewCtx, _: &mut S) -> (Self::Element, ()) {
        let label = NewWidget::new(
            Label::new(self.text.clone()).with_style(StyleProperty::FontSize(self.size)),
        );
        let mut pod = ctx
            .with_action_widget(|ctx| ctx.create_pod(RadioButton::from_label(self.checked, label)));
        pod.new_widget.options.disabled = self.disabled;
        (pod, ())
    }
    fn rebuild(
        &self,
        old: &Self,
        _: &mut (),
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut S,
    ) {
        if old.disabled != self.disabled {
            element.ctx.set_disabled(self.disabled);
        }
        if old.text != self.text {
            RadioButton::set_text(&mut element, self.text.clone());
        }
        if old.checked != self.checked {
            RadioButton::set_checked(&mut element, self.checked);
        }
        if old.size != self.size {
            Label::insert_style(
                &mut RadioButton::label_mut(&mut element),
                StyleProperty::FontSize(self.size),
            );
        }
    }
    fn teardown(&self, _: &mut (), ctx: &mut ViewCtx, element: Mut<'_, Self::Element>) {
        ctx.teardown_action_source(element);
    }
    fn message(
        &self,
        _: &mut (),
        message: &mut MessageCtx,
        _: Mut<'_, Self::Element>,
        state: &mut S,
    ) -> MessageResult<()> {
        if message.take_message::<RadioButtonSelected>().is_some() {
            (self.select)(state);
            MessageResult::Action(())
        } else {
            MessageResult::Stale
        }
    }
}
