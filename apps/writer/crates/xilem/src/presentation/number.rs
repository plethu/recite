//! A compact number field with the spin button role, in place of separate minus and plus buttons
//! beside a sentence: one Tab stop named for its setting, arrow keys, Home and End.
#[cfg(test)]
mod tests;
mod widget;
use super::Type;
use recite_writer_session::typography::FONT_UI;
pub(crate) use widget::{NumberField, Range, Spin};
use xilem::{
    Pod, ViewCtx,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::{
        core::{ArcStr, NewWidget, StyleProperty},
        widgets::Label,
    },
    peniko::Color,
};

pub(crate) struct Number<F> {
    name: ArcStr,
    value: ArcStr,
    range: Range,
    scale: f32,
    ink: Color,
    spin: F,
}

/// A number field named `name` showing `value`, which calls `spin` for each requested change.
pub(crate) fn number_field<S: 'static, F: Fn(&mut S, Spin) + Send + Sync + 'static>(
    scale: f32,
    name: impl Into<ArcStr>,
    value: impl Into<ArcStr>,
    range: Range,
    ink: Color,
    spin: F,
) -> Number<F> {
    Number {
        name: name.into(),
        value: value.into(),
        range,
        scale,
        ink,
        spin,
    }
}
impl<F> Number<F> {
    /// Each stepper's square target: at least 24px at any scale.
    fn target(&self) -> f64 {
        24. * f64::from(self.scale)
    }
}
impl<F> ViewMarker for Number<F> {}
impl<S: 'static, F: Fn(&mut S, Spin) + Send + Sync + 'static> View<S, (), ViewCtx> for Number<F> {
    type Element = Pod<NumberField>;
    type ViewState = ();
    fn build(&self, ctx: &mut ViewCtx, _: &mut S) -> (Self::Element, ()) {
        // The value is the spin button's own value; its painted text stays out of the tree.
        let text = Label::new(self.value.clone())
            .with_style(StyleProperty::FontSize(Type::Control.size(self.scale)))
            .with_style(StyleProperty::FontFamily(super::print::font(FONT_UI)))
            .accessibility_hidden(true);
        let field = NumberField::new(
            self.name.clone(),
            self.value.clone(),
            self.range,
            (self.ink, self.target()),
            NewWidget::new(text),
        );
        (ctx.with_action_widget(|ctx| ctx.create_pod(field)), ())
    }
    fn rebuild(
        &self,
        old: &Self,
        _: &mut (),
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut S,
    ) {
        if old.name != self.name {
            NumberField::set_name(&mut element, self.name.clone());
        }
        if old.value != self.value {
            NumberField::set_value(&mut element, self.value.clone());
        }
        if old.range != self.range {
            NumberField::set_range(&mut element, self.range);
        }
        if old.ink != self.ink || old.scale != self.scale {
            NumberField::set_face(&mut element, self.ink, self.target());
            Label::insert_style(
                &mut NumberField::text_mut(&mut element),
                StyleProperty::FontSize(Type::Control.size(self.scale)),
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
        match message.take_message::<Spin>() {
            Some(spin) => {
                (self.spin)(state, *spin);
                MessageResult::Action(())
            }
            None => MessageResult::Stale,
        }
    }
}
