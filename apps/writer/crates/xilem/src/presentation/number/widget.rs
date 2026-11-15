//! A compact number field: "− 20 px +" in one tonal well, exposed as a named spin button.
use xilem::masonry::{
    accesskit::{Action, ActionData, Node, Role},
    core::{
        AccessCtx, AccessEvent, ArcStr, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget,
        PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update,
        UpdateCtx, Widget, WidgetMut, WidgetPod,
        keyboard::{Key, NamedKey},
    },
    imaging::Painter,
    kurbo::{Axis, Line, Point, Size, Stroke},
    layout::{AsUnit, LayoutSize, LenReq, Length, SizeDef},
    peniko::Color,
    widgets::Label,
};

/// The value's least width, in stepper targets.
const VALUE: f64 = 2.5;

/// What a press asks of the value: one step either way, or a value within the range, such as a
/// bound from Home and End or a value set by assistive technology.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Spin {
    Step { up: bool },
    To(f64),
}

/// The current value, its bounds and step, for assistive technology and the stepper faces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Range {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

pub(crate) struct NumberField {
    name: ArcStr,
    /// The value as read aloud, with its unit: "20 px".
    value: ArcStr,
    range: Range,
    /// The stepper glyphs' ink, and the side of each square stepper target.
    ink: Color,
    target: f64,
    text: WidgetPod<Label>,
}
impl NumberField {
    pub fn new(
        name: ArcStr,
        value: ArcStr,
        range: Range,
        (ink, target): (Color, f64),
        text: NewWidget<Label>,
    ) -> Self {
        Self {
            name,
            value,
            range,
            ink,
            target,
            text: text.to_pod(),
        }
    }
    pub fn text_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, Label> {
        this.ctx.get_mut(&mut this.widget.text)
    }
    pub fn set_name(this: &mut WidgetMut<'_, Self>, name: ArcStr) {
        this.widget.name = name;
        this.ctx.request_accessibility_update();
    }
    pub fn set_value(this: &mut WidgetMut<'_, Self>, value: ArcStr) {
        this.widget.value = value.clone();
        this.ctx.request_accessibility_update();
        Label::set_text(&mut Self::text_mut(this), value);
    }
    pub fn set_range(this: &mut WidgetMut<'_, Self>, range: Range) {
        this.widget.range = range;
        this.ctx.request_accessibility_update();
        this.ctx.request_paint_only();
    }
    pub fn set_face(this: &mut WidgetMut<'_, Self>, ink: Color, target: f64) {
        this.widget.ink = ink;
        this.widget.target = target;
        this.ctx.request_layout();
    }
    fn spin(&self, ctx: &mut EventCtx<'_>, spin: Spin) {
        let Range {
            value, min, max, ..
        } = self.range;
        let spin = match spin {
            Spin::Step { up: true } if value >= max => return,
            Spin::Step { up: false } if value <= min => return,
            Spin::Step { .. } => spin,
            Spin::To(target) => {
                let target = target.clamp(min, max);
                if target == value {
                    return;
                }
                Spin::To(target)
            }
        };
        ctx.submit_action::<Spin>(spin);
    }
}

impl Widget for NumberField {
    type Action = Spin;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        if let PointerEvent::Down(press) = event {
            ctx.request_focus();
            let x = ctx.local_position(press.state.position).x;
            let content = ctx.content_box();
            if x <= content.x0 + self.target {
                self.spin(ctx, Spin::Step { up: false });
            } else if x >= content.x1 - self.target {
                self.spin(ctx, Spin::Step { up: true });
            }
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        let TextEvent::Keyboard(key) = event else {
            return;
        };
        if !key.state.is_down() {
            return;
        }
        let spin = match key.key {
            Key::Named(NamedKey::ArrowUp | NamedKey::ArrowRight) => Spin::Step { up: true },
            Key::Named(NamedKey::ArrowDown | NamedKey::ArrowLeft) => Spin::Step { up: false },
            Key::Named(NamedKey::End) => Spin::To(self.range.max),
            Key::Named(NamedKey::Home) => Spin::To(self.range.min),
            _ => return,
        };
        self.spin(ctx, spin);
        ctx.set_handled();
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        match (event.action, &event.data) {
            (Action::Increment, _) => self.spin(ctx, Spin::Step { up: true }),
            (Action::Decrement, _) => self.spin(ctx, Spin::Step { up: false }),
            (Action::SetValue, Some(ActionData::NumericValue(value))) => {
                self.spin(ctx, Spin::To(*value));
            }
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(
            event,
            Update::HoveredChanged(_)
                | Update::ActiveChanged(_)
                | Update::FocusChanged(_)
                | Update::DisabledChanged(_)
        ) {
            ctx.request_paint_only();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.text);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        let context = LayoutSize::maybe(axis.cross(), cross_length);
        let text = ctx
            .compute_length(&mut self.text, len_req.into(), context, axis, cross_length)
            .get();
        // Every field in a column shares one width for values up to "100%" or "360 px", and is
        // at least a stepper target tall whatever its padding.
        match axis {
            Axis::Horizontal => text.max(VALUE * self.target) + 2. * self.target,
            Axis::Vertical => text.max(self.target),
        }
        .px()
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _: &PropertiesRef<'_>, size: Size) {
        let text = ctx.compute_size(&mut self.text, SizeDef::fit(size), size.into());
        ctx.run_layout(&mut self.text, text);
        ctx.place_child(
            &mut self.text,
            Point::new(
                (size.width - text.width) / 2.,
                (size.height - text.height) / 2.,
            ),
        );
        ctx.derive_baselines(&self.text);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let content = ctx.content_box();
        let middle = content.center().y;
        let arm = self.target * 0.25;
        let stroke = Stroke::new(1.5);
        let face = |enabled: bool| {
            if enabled && !ctx.is_disabled() {
                self.ink
            } else {
                self.ink.with_alpha(0.4)
            }
        };
        let minus = content.x0 + self.target / 2.;
        let plus = content.x1 - self.target / 2.;
        let lower = face(self.range.value > self.range.min);
        let raise = face(self.range.value < self.range.max);
        painter
            .stroke(
                Line::new((minus - arm, middle), (minus + arm, middle)),
                &stroke,
                lower,
            )
            .draw();
        painter
            .stroke(
                Line::new((plus - arm, middle), (plus + arm, middle)),
                &stroke,
                raise,
            )
            .draw();
        painter
            .stroke(
                Line::new((plus, middle - arm), (plus, middle + arm)),
                &stroke,
                raise,
            )
            .draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::SpinButton
    }

    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(&*self.name);
        node.set_value(&*self.value);
        node.set_numeric_value(self.range.value);
        node.set_min_numeric_value(self.range.min);
        node.set_max_numeric_value(self.range.max);
        node.set_numeric_value_step(self.range.step);
        if self.range.value < self.range.max {
            node.add_action(Action::Increment);
        }
        if self.range.value > self.range.min {
            node.add_action(Action::Decrement);
        }
        node.add_action(Action::SetValue);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.text.id()])
    }

    fn accepts_focus(&self) -> bool {
        true
    }
}
