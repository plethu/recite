//! A scaled label for Masonry's native radio interaction and accessibility.
use recite_writer_session::typography::FONT_UI;
use xilem::masonry::parley::style::FontWeight;
use xilem::peniko::Color;
use xilem::{
    Pod, ViewCtx,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::{
        core::{ArcStr, NewWidget, Property, StyleProperty, UsesProperty},
        properties::ContentColor,
        widgets::{Label, RadioButton, RadioButtonSelected},
    },
};

/// Whether a radio is its group's one Tab stop: the checked choice. The native host reads it to
/// pass over the other choices, which arrow keys reach instead.
#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct GroupStop(pub bool);
impl Property for GroupStop {
    fn static_default() -> &'static Self {
        static OTHER: GroupStop = GroupStop(false);
        &OTHER
    }
}
impl UsesProperty<GroupStop> for RadioButton {}

pub(crate) struct Radio<F> {
    text: ArcStr,
    checked: bool,
    size: f32,
    weight: FontWeight,
    disabled: bool,
    label_color: Option<Color>,
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
        size: super::Type::Control.size(scale),
        weight: super::Type::Control.weight(),
        disabled: false,
        label_color: None,
        select,
    }
}
impl<F> Radio<F> {
    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = Some(color);
        self
    }
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }
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
        let mut label = NewWidget::new(
            Label::new(self.text.clone())
                .with_style(StyleProperty::FontSize(self.size))
                .with_style(StyleProperty::FontWeight(self.weight))
                .with_style(StyleProperty::FontFamily(super::print::font(FONT_UI))),
        );
        if let Some(color) = self.label_color {
            label = label.with_props(ContentColor::new(color));
        }
        let mut pod = ctx
            .with_action_widget(|ctx| ctx.create_pod(RadioButton::from_label(self.checked, label)));
        pod.new_widget = pod.new_widget.with_props(GroupStop(self.checked));
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
            element.insert_prop(GroupStop(self.checked));
        }
        if old.size != self.size {
            Label::insert_style(
                &mut RadioButton::label_mut(&mut element),
                StyleProperty::FontSize(self.size),
            );
        }
        if old.weight != self.weight {
            Label::insert_style(
                &mut RadioButton::label_mut(&mut element),
                StyleProperty::FontWeight(self.weight),
            );
        }
        if old.label_color != self.label_color {
            let mut label = RadioButton::label_mut(&mut element);
            if let Some(color) = self.label_color {
                label.insert_prop(ContentColor::new(color));
            } else {
                label.remove_prop::<ContentColor>();
            }
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
