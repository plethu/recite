use super::*;
use xilem::masonry::{
    accesskit::{Action, ActionRequest, TreeId},
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
        assert_eq!(
            harness
                .get_widget_with_id(id)
                .get_prop::<BorderColor>()
                .color,
            palette.on_accent
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
fn prose_text_takes_the_palette_ink_rather_than_the_toolkit_default() {
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
    }
}
