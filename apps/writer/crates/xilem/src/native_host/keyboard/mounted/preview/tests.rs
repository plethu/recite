use super::*;
use crate::attention::{EntryContext, PaneEntry};
use xilem::masonry::{
    core::{WidgetId, keyboard as text_key},
    kurbo::Rect,
    widgets::{Label, Portal},
};

const RETURN: WidgetTag<Button> = WidgetTag::named("preview-return");
const SETUP: WidgetTag<Button> = WidgetTag::named("preview-setup");
const VALUE: WidgetTag<TextInput> = WidgetTag::named("preview-value");
const FIELD_CONTEXT: WidgetTag<Flex> = WidgetTag::named("preview-field-context");
const VIEWPORT: WidgetTag<Portal<Flex>> = WidgetTag::named("preview-value-viewport");
const DIAGNOSTIC: WidgetTag<Portal<Label>> = WidgetTag::named("preview-diagnostic");

#[test]
fn preview_pane_prefers_setup_and_falls_back_to_return_when_setup_is_disabled() {
    let (mut root, _signals) = mounted();
    let pane = root.get_layer_root(0).children()[0].children()[0].children()[1].id();
    let body = Flex::column()
        .with_fixed(NewWidget::new(Button::with_text("Return to writing")).with_tag(RETURN))
        .with_fixed(
            NewWidget::new(
                Flex::column()
                    .with_fixed(NewWidget::new(Button::with_text("Start preview")).with_tag(SETUP)),
            )
            .with_props(PaneEntry(true)),
        );
    root.edit_widget(pane, |mut widget| {
        widget.insert_prop(Region(Some(Pane::Preview)));
        SizedBox::set_child(&mut widget.downcast::<SizedBox>(), NewWidget::new(body));
    });
    let setup = root.get_widget_with_tag(SETUP).unwrap().id();
    let returning = root.get_widget_with_tag(RETURN).unwrap().id();
    assert!(panes::focus_pane(&mut root, Pane::Preview));
    assert_eq!(root.focused_widget(), Some(setup));
    root.edit_widget(setup, |mut widget| widget.ctx.set_disabled(true));
    root.focus_on(None);
    assert!(panes::focus_pane(&mut root, Pane::Preview));
    assert_eq!(root.focused_widget(), Some(returning));
    assert!(!root.focus_on(Some(setup)));
}

#[test]
fn diagnostics_prefer_their_disclosure_control_and_remain_reachable_by_tab() {
    let (mut root, _signals) = mounted();
    let pane = root.get_layer_root(0).children()[0].children()[0].children()[1].id();
    let body = Flex::column()
        .with_fixed(NewWidget::new(Button::with_text("Return to writing")).with_tag(RETURN))
        .with_fixed(
            NewWidget::new(Flex::column().with_fixed(
                NewWidget::new(Button::with_text("Hide preview error")).with_tag(SETUP),
            ))
            .with_props(PaneEntry(true)),
        )
        .with_fixed(
            NewWidget::new(
                Portal::new(NewWidget::new(Label::new(
                    "The failed attempt's diagnostic.",
                )))
                .constrain_horizontal(true),
            )
            .with_tag(DIAGNOSTIC),
        );
    root.edit_widget(pane, |mut widget| {
        widget.insert_prop(Region(Some(Pane::Preview)));
        SizedBox::set_child(&mut widget.downcast::<SizedBox>(), NewWidget::new(body));
    });
    let anchor = root.get_widget_with_tag(SETUP).unwrap().id();
    let diagnostic = root.get_widget_with_tag(DIAGNOSTIC).unwrap();
    assert!(diagnostic.ctx().accepts_focus());
    let diagnostic = diagnostic.id();
    assert!(panes::focus_pane(&mut root, Pane::Preview));
    assert_eq!(root.focused_widget(), Some(anchor));
    root.handle_text_event(core::TextEvent::Keyboard(text_key::KeyboardEvent {
        state: text_key::KeyState::Down,
        key: text_key::Key::Named(text_key::NamedKey::Tab),
        code: text_key::Code::Tab,
        ..Default::default()
    }));
    assert_eq!(root.focused_widget(), Some(diagnostic));
}

#[test]
fn contextual_result_requests_reveal_once_without_overriding_later_focus() {
    let (mut root, _signals) = mounted();
    let (mut context, _) = context();
    let mut writer = Writer::from_session(Session::examples().unwrap(), None);
    let mut attention = panes::Attention::default();
    let pane = root.get_layer_root(0).children()[0].children()[0].children()[1].id();
    root.edit_widget(pane, |mut widget| {
        widget.insert_prop(Region(Some(Pane::External)));
    });
    let target = root.get_widget_with_tag(MAP).unwrap().id();
    let navigation = field(&root, NAVIGATION);
    context.screen = Some(Screen::External);
    attention.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(target));
    root.focus_on(Some(navigation));
    writer.focus_pane(PaneMotion::To(Pane::External));
    context.attention = writer.attention.clone();
    attention.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(target));
    root.focus_on(Some(navigation));
    attention.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(navigation));
}

fn bounds(root: &RenderRoot, id: WidgetId) -> Rect {
    let widget = root.get_widget(id).unwrap();
    widget
        .ctx()
        .window_transform()
        .transform_rect_bbox(widget.ctx().border_box())
}

#[test]
fn explicit_entry_reveals_field_context_and_prioritises_the_control_in_a_small_viewport() {
    for scale in [1., 2.] {
        for height in [120., 40.] {
            let (mut root, _signals) = mounted();
            let value = NewWidget::new(TextInput::new("invalid")).with_tag(VALUE);
            let context = NewWidget::new(
                Flex::column()
                    .with_fixed(NewWidget::new(
                        SizedBox::new(NewWidget::new(Label::new("Tickets")))
                            .height((24. * scale).px()),
                    ))
                    .with_fixed(NewWidget::new(
                        SizedBox::new(value).height((32. * scale).px()),
                    ))
                    .with_fixed(NewWidget::new(
                        SizedBox::new(NewWidget::new(Label::new("Enter a whole number.")))
                            .height((24. * scale).px()),
                    )),
            )
            .with_tag(FIELD_CONTEXT)
            .with_props(PaneEntry(true))
            .with_props(EntryContext(true));
            let content = Flex::column()
                .with_fixed(NewWidget::new(
                    SizedBox::empty().height((600. * scale).px()),
                ))
                .with_fixed(context)
                .with_fixed(NewWidget::new(
                    SizedBox::empty().height((400. * scale).px()),
                ));
            let viewport =
                NewWidget::new(Portal::new(NewWidget::new(content)).constrain_horizontal(true))
                    .with_tag(VIEWPORT);
            let body = NewWidget::new(
                Flex::column().with_fixed(NewWidget::new(
                    SizedBox::new(viewport)
                        .width(400.px())
                        .height((height * scale).px()),
                )),
            )
            .with_props(Region(Some(Pane::Preview)));
            let root_id = root.get_layer_root(0).id();
            root.edit_widget(root_id, |mut widget| {
                Passthrough::set_child(&mut widget.downcast(), body)
            });
            root.redraw();
            let value = root.get_widget_with_tag(VALUE).unwrap().id();
            let input = field(&root, VALUE);
            assert!(panes::focus_pane(&mut root, Pane::Preview));
            root.redraw();
            assert_eq!(root.focused_widget(), Some(input));
            let viewport = bounds(&root, root.get_widget_with_tag(VIEWPORT).unwrap().id());
            assert!(
                viewport.contains_rect(bounds(&root, value)),
                "input takes priority: {scale} {height}"
            );
            if height == 120. {
                assert!(
                    viewport.contains_rect(bounds(
                        &root,
                        root.get_widget_with_tag(FIELD_CONTEXT).unwrap().id()
                    )),
                    "label and validation context fit: {scale}"
                );
            }
        }
    }
}
