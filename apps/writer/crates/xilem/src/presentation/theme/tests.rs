use super::*;
use crate::presentation::control_style::ControlStyle;
use recite_writer_session::palette::INK_HOVER;
use xilem::masonry::{
    accesskit::{Action, ActionRequest, TreeId},
    core::PointerButton,
    core::{NewWidget, WidgetId},
    kurbo::Point,
    testing::TestHarness,
    widgets::{ButtonPress, CheckboxToggled, Flex, RadioButtonSelected, RadioGroup},
};

#[test]
fn live_indicator_scale_reflows_mounted_controls_and_preserves_focus_and_actions() {
    let checkbox = NewWidget::new(Checkbox::new(false, "Reviewed"));
    let checkbox_id = checkbox.id();
    let radio = NewWidget::new(RadioButton::new(false, "Script"));
    let radio_id = radio.id();
    let root = NewWidget::new(
        Flex::column()
            .with_fixed(checkbox)
            .with_fixed(NewWidget::new(RadioGroup::new(radio))),
    );
    let palette = palette(&WriterConfig::default());
    let mut harness = TestHarness::create_with_size(properties(&palette, 1.), root, (800, 500));
    harness.focus_on(Some(radio_id));
    for (scale, side, gap) in [(1., 18., 4.), (2., 36., 8.), (1., 18., 4.)] {
        harness.set_default_properties(Arc::new(properties(&palette, scale)));
        harness.redraw();
        assert_eq!(harness.focused_widget_id(), Some(radio_id));
        for id in [checkbox_id, radio_id] {
            let control = harness.get_widget_with_id(id);
            let metrics = control.get_prop::<CheckmarkSize>();
            assert_eq!((metrics.side.get(), metrics.gap.get()), (side, gap));
            assert!(control.ctx().content_box().height() >= side);
            let label = control.children()[0];
            let label_origin = control
                .ctx()
                .to_local(label.ctx().window_transform() * Point::ORIGIN);
            assert_eq!(label_origin.x, side + gap);
            assert_eq!(
                control.get_prop::<CheckmarkStrokeWidth>().width,
                f64::from(scale)
            );
        }
        if scale == 2. {
            for id in [checkbox_id, radio_id] {
                click(&mut harness, id);
            }
            assert_eq!(
                harness.pop_action::<CheckboxToggled>(),
                Some((CheckboxToggled(true), checkbox_id))
            );
            assert_eq!(
                harness.pop_action::<RadioButtonSelected>(),
                Some((RadioButtonSelected, radio_id))
            );
            harness.focus_on(Some(radio_id));
        }
    }
}

fn click(harness: &mut TestHarness<Flex>, id: WidgetId) {
    harness.process_access_event(ActionRequest {
        action: Action::Click,
        target_tree: TreeId::ROOT,
        target_node: id.to_raw().into(),
        data: None,
    });
}

#[test]
fn quiet_controls_keep_native_geometry_focus_and_actions_in_every_state() {
    let palette = palette(&WriterConfig::default());
    let button = NewWidget::new(Button::new(NewWidget::new(Label::new("Apply draft"))))
        .with_class(ControlStyle::Action.class());
    let id = button.id();
    let mut harness = TestHarness::create_with_size(
        properties(&palette, 1.),
        NewWidget::new(Flex::column().with_fixed(button)),
        (500, 200),
    );
    let size = harness.get_widget_with_id(id).ctx().border_box().size();
    assert_eq!(
        harness
            .get_widget_with_id(id)
            .get_prop::<BorderColor>()
            .color,
        Color::TRANSPARENT
    );
    harness.mouse_move_to(id);
    assert_eq!(
        harness
            .get_widget_with_id(id)
            .get_prop::<BorderColor>()
            .color,
        Color::TRANSPARENT
    );
    harness.focus_on(Some(id));
    assert_eq!(
        harness
            .get_widget_with_id(id)
            .get_prop::<BorderColor>()
            .color,
        palette.accent
    );
    assert_eq!(
        harness.get_widget_with_id(id).ctx().border_box().size(),
        size
    );
    click(&mut harness, id);
    assert!(harness.pop_action::<ButtonPress>().is_some());
    harness.edit_widget_with_id(id, |mut widget| widget.ctx.set_disabled(true));
    harness.redraw();
    assert_eq!(
        harness
            .get_widget_with_id(id)
            .get_prop::<BorderColor>()
            .color,
        Color::TRANSPARENT
    );
    assert_eq!(
        harness.get_widget_with_id(id).ctx().border_box().size(),
        size
    );
    click(&mut harness, id);
    assert!(harness.pop_action::<ButtonPress>().is_none());
}

#[test]
fn primary_controls_keep_their_target_and_show_distinct_focus_and_disabled_states() {
    for dark in [false, true] {
        let config = WriterConfig {
            theme: if dark {
                WriterTheme::Dark
            } else {
                WriterTheme::Light
            },
            ..Default::default()
        };
        let palette = palette(&config);
        let button = NewWidget::new(Button::new(NewWidget::new(Label::new("Create project"))))
            .with_class(ControlStyle::Primary.class());
        let id = button.id();
        let mut harness = TestHarness::create_with_size(
            properties(&palette, 1.),
            NewWidget::new(Flex::column().with_fixed(button)),
            (500, 200),
        );
        let size = harness.get_widget_with_id(id).ctx().border_box().size();
        assert_eq!(
            *harness.get_widget_with_id(id).get_prop::<Background>(),
            Background::Color(palette.accent)
        );
        harness.mouse_move_to(id);
        harness.focus_on(Some(id));
        // The fill's own border never carries focus: plum on ink is 1.73:1. The focus ring does.
        assert_eq!(
            harness
                .get_widget_with_id(id)
                .get_prop::<BorderColor>()
                .color,
            palette.accent
        );
        assert_eq!(
            harness.get_widget_with_id(id).ctx().border_box().size(),
            size
        );
        click(&mut harness, id);
        assert!(harness.pop_action::<ButtonPress>().is_some());
        harness.edit_widget_with_id(id, |mut widget| widget.ctx.set_disabled(true));
        harness.redraw();
        assert_eq!(
            *harness.get_widget_with_id(id).get_prop::<Background>(),
            Background::Color(palette.inset)
        );
        assert_eq!(
            harness.get_widget_with_id(id).ctx().border_box().size(),
            size
        );
        click(&mut harness, id);
        assert!(harness.pop_action::<ButtonPress>().is_none());
    }
}

#[test]
fn prose_and_shaped_text_take_the_palette_ink_rather_than_the_toolkit_default() {
    use xilem::masonry::{properties::ContentColor, widgets::Prose};
    for dark in [false, true] {
        let config = WriterConfig {
            theme: if dark {
                recite_config::WriterTheme::Dark
            } else {
                recite_config::WriterTheme::Light
            },
            ..WriterConfig::default()
        };
        let palette = palette(&config);
        let prose = NewWidget::new(Prose::new("Guidance for the writer."));
        let id = prose.id();
        let mut harness =
            TestHarness::create_with_size(properties(&palette, 1.), prose, (400, 100));
        harness.redraw();
        let area = harness.get_widget_with_id(id).children()[0].id();
        assert_eq!(
            harness
                .get_widget_with_id(area)
                .get_prop::<ContentColor>()
                .color,
            palette.ink,
            "dark={dark}"
        );
        let shaped = NewWidget::new(ShapedText::new("The lower gate is stuck open."));
        let id = shaped.id();
        let harness = TestHarness::create_with_size(properties(&palette, 1.), shaped, (400, 100));
        assert_eq!(
            harness
                .get_widget_with_id(id)
                .get_prop::<ContentColor>()
                .color,
            palette.ink,
            "shaped text, dark={dark}"
        );
    }
}

fn content_origin(harness: &TestHarness<Flex>, id: WidgetId) -> Point {
    let label = harness.get_widget_with_id(id).children()[0];
    label.ctx().window_transform() * Point::ORIGIN
}

#[test]
fn every_class_takes_ink_on_hover_two_pixel_focus_and_a_sunk_press_inside_one_box() {
    for dark in [false, true] {
        let config = WriterConfig {
            theme: if dark {
                WriterTheme::Dark
            } else {
                WriterTheme::Light
            },
            ..Default::default()
        };
        let palette = palette(&config);
        for (class, rest_border, hover_border) in [
            (None, palette.boundary, palette.ink),
            (
                Some(ControlStyle::Action),
                Color::TRANSPARENT,
                Color::TRANSPARENT,
            ),
            (
                Some(ControlStyle::Navigation),
                Color::TRANSPARENT,
                Color::TRANSPARENT,
            ),
            (
                Some(ControlStyle::Icon),
                Color::TRANSPARENT,
                Color::TRANSPARENT,
            ),
        ] {
            let mut button = NewWidget::new(Button::new(NewWidget::new(Label::new("Save"))));
            if let Some(class) = class {
                button = button.with_class(class.class());
            }
            let id = button.id();
            let mut harness = TestHarness::create_with_size(
                properties(&palette, 1.),
                NewWidget::new(Flex::column().with_fixed(button)),
                (500, 200),
            );
            let control = |harness: &TestHarness<Flex>| {
                let widget = harness.get_widget_with_id(id);
                (
                    widget.ctx().border_box().size(),
                    widget.get_prop::<Background>().clone(),
                    widget.get_prop::<BorderColor>().color,
                    widget.get_prop::<BorderWidth>().width.get(),
                    widget.get_prop::<CornerRadius>().radius.get(),
                )
            };
            let (size, _, border, width, radius) = control(&harness);
            assert_eq!((border, width, radius), (rest_border, 1., 2.));
            let origin = content_origin(&harness, id);
            harness.mouse_move_to(id);
            let hovered = control(&harness);
            assert_eq!(
                hovered.1,
                Background::Color(palette.ink.with_alpha(INK_HOVER))
            );
            assert_eq!((hovered.0, hovered.2), (size, hover_border));
            harness.mouse_button_press(Some(PointerButton::Primary));
            let pressed = control(&harness);
            assert_eq!(pressed.1, Background::Color(palette.pressed));
            assert_eq!((pressed.0, pressed.2, pressed.3), (size, palette.ink, 2.));
            let sunk = content_origin(&harness, id);
            assert_eq!((sunk.x - origin.x, sunk.y - origin.y), (1., 1.));
            harness.mouse_button_release(Some(PointerButton::Primary));
            assert!(harness.pop_action::<ButtonPress>().is_some());
            // Focus is a 2px accent border; hover does not take it away.
            harness.focus_on(Some(id));
            let focused = control(&harness);
            assert_eq!(
                (focused.0, focused.2, focused.3),
                (size, palette.accent, 2.)
            );
            assert_eq!(content_origin(&harness, id), origin);
            harness.edit_widget_with_id(id, |mut widget| widget.ctx.set_disabled(true));
            harness.redraw();
            let disabled = control(&harness);
            assert_eq!(disabled.0, size);
            if class.is_none() {
                assert_eq!(disabled.2, palette.rule);
            }
        }
    }
}

#[test]
fn a_selected_segment_is_a_flush_stamp_that_keeps_its_box_through_every_state() {
    for dark in [false, true] {
        let config = WriterConfig {
            theme: if dark {
                WriterTheme::Dark
            } else {
                WriterTheme::Light
            },
            ..Default::default()
        };
        let palette = palette(&config);
        let radio = NewWidget::new(RadioButton::new(true, "Script"))
            .with_class(ControlStyle::Selected.class());
        let id = radio.id();
        let mut harness = TestHarness::create_with_size(
            properties(&palette, 1.),
            NewWidget::new(Flex::column().with_fixed(NewWidget::new(RadioGroup::new(radio)))),
            (500, 200),
        );
        let state = |harness: &TestHarness<Flex>| {
            let widget = harness.get_widget_with_id(id);
            (
                widget.ctx().border_box().size(),
                widget.get_prop::<Background>().clone(),
                widget.get_prop::<BorderColor>().color,
            )
        };
        let (size, fill, border) = state(&harness);
        assert_eq!(
            (fill, border),
            (Background::Color(palette.stamp), palette.stamp)
        );
        harness.mouse_move_to(id);
        harness.focus_on(Some(id));
        let (focused, _, border) = state(&harness);
        assert_eq!(focused, size);
        // Accent on a stamp is 1.63:1 in dark mode; the ring in `on_stamp` carries focus.
        assert_ne!(border, palette.accent);
    }
}
