use super::*;
use xilem::masonry::{core::keyboard as text_key, kurbo::Rect, widgets::Portal};
use xilem::winit::keyboard::{Key as NativeKey, KeyCode, PhysicalKey as NativePhysical};

const FIRST: WidgetTag<TextInput> = WidgetTag::named("tab-first-field");
const LAST: WidgetTag<TextInput> = WidgetTag::named("tab-last-field");

fn bounds(root: &RenderRoot, id: WidgetId) -> Rect {
    let widget = root.get_widget(id).unwrap();
    widget
        .ctx()
        .window_transform()
        .transform_rect_bbox(widget.ctx().border_box())
}

#[test]
fn native_tab_reveals_complete_inputs_and_focus_observation_does_not_scroll() {
    let (mut root, _signals) = mounted();
    let pane = root.get_layer_root(0).children()[0].children()[0].children()[2].id();
    let body = Flex::column()
        .with_fixed(NewWidget::new(TextInput::new("First")).with_tag(FIRST))
        .with_fixed(NewWidget::new(SizedBox::empty().height(500.px())))
        .with_fixed(NewWidget::new(TextInput::new("Last")).with_tag(LAST));
    root.edit_widget(pane, |mut widget| {
        SizedBox::set_child(
            &mut widget.downcast::<SizedBox>(),
            NewWidget::new(Portal::new(NewWidget::new(body))),
        );
    });
    let mut keyboard = Keyboard::default();
    keyboard.synchronize(&mut root);
    let first = field(&root, FIRST);
    let last = field(&root, LAST);
    root.focus_on(Some(first));
    let viewport = bounds(&root, pane);
    assert!(bounds(&root, root.get_widget_with_tag(LAST).unwrap().id()).y0 > viewport.y1);
    let logical = NativeKey::Named(NamedKey::Tab);
    for (shift, target, control) in [(false, last, LAST), (true, first, FIRST)] {
        keyboard.modifiers = if shift {
            ModifiersState::SHIFT
        } else {
            ModifiersState::empty()
        };
        assert!(!keyboard.key_event(
            &root,
            Keystroke {
                logical_key: &logical,
                physical_key: NativePhysical::Code(KeyCode::Tab),
                state: ElementState::Pressed,
                repeat: false,
            },
            false
        ));
        root.handle_text_event(core::TextEvent::Keyboard(text_key::KeyboardEvent {
            state: text_key::KeyState::Down,
            key: text_key::Key::Named(text_key::NamedKey::Tab),
            code: text_key::Code::Tab,
            modifiers: if shift {
                text_key::Modifiers::SHIFT
            } else {
                text_key::Modifiers::empty()
            },
            ..Default::default()
        }));
        assert_eq!(root.focused_widget(), Some(target));
        keyboard.synchronize(&mut root);
        let input = bounds(&root, root.get_widget_with_tag(control).unwrap().id());
        assert!(
            input.y0 >= viewport.y0 && input.y1 <= viewport.y1,
            "{input:?} outside {viewport:?}"
        );
    }
    root.focus_on(Some(last));
    keyboard.synchronize(&mut root);
    assert!(bounds(&root, root.get_widget_with_tag(LAST).unwrap().id()).y0 > viewport.y1);
}
